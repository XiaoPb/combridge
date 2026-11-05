//! 帧层：`AA 11 CMD LEN PAYLOAD CRC8`
//!
//! - `LEN` 仅为 payload 长度
//! - 总长 = LEN + 5
//! - CRC8 覆盖 `AA 11 CMD LEN PAYLOAD`

use crate::crc8;
use crate::error::FrameError;

pub const HEADER: [u8; 2] = [0xAA, 0x11];
/// 设备运行时单包 payload 上限（见文档 2.3）
pub const MAX_PAYLOAD: usize = 233;
/// 设备侧允许的最大 LEN（`GH3X2X_UPROTOCOL_PAYLOAD_LEN_MAX`）
pub const MAX_LEN: usize = 238;
pub const OVERHEAD: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub cmd: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(cmd: u8, payload: Vec<u8>) -> Self {
        Self { cmd, payload }
    }

    pub fn encode(&self) -> Result<Vec<u8>, FrameError> {
        if self.payload.len() > MAX_LEN {
            return Err(FrameError::PayloadTooLong(self.payload.len()));
        }
        let mut out = Vec::with_capacity(self.payload.len() + OVERHEAD);
        out.extend_from_slice(&HEADER);
        out.push(self.cmd);
        out.push(self.payload.len() as u8);
        out.extend_from_slice(&self.payload);
        let c = crc8::crc8(&out);
        out.push(c);
        Ok(out)
    }

    /// 解析一个**完整**帧（长度必须刚好等于 LEN+5）
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < OVERHEAD {
            return Err(FrameError::Short(bytes.len()));
        }
        if bytes[0] != HEADER[0] || bytes[1] != HEADER[1] {
            return Err(FrameError::Short(bytes.len()));
        }
        let len = bytes[3] as usize;
        if bytes.len() != len + OVERHEAD {
            return Err(FrameError::UnexpectedLength { cmd: bytes[2], len });
        }
        let expect = crc8::crc8(&bytes[..len + 4]);
        let got = bytes[len + 4];
        if expect != got {
            return Err(FrameError::CrcMismatch { got, expect });
        }
        Ok(Self {
            cmd: bytes[2],
            payload: bytes[4..4 + len].to_vec(),
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DecoderStats {
    pub frames: u64,
    pub crc_errors: u64,
    pub resyncs: u64,
    pub bad_len: u64,
}

/// 流式解码器：可接收半帧，内部做重同步（与设备侧的容错行为对应）
#[derive(Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
    pub stats: DecoderStats,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.buf.clear();
    }

    pub fn push(&mut self, data: &[u8]) -> Vec<Frame> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();

        loop {
            // 1) 找帧头
            let Some(pos) = find_header(&self.buf) else {
                // 没有帧头：保留最后 1 字节（可能是半个 AA）
                if self.buf.len() > 1 {
                    let keep = self.buf.len() - 1;
                    self.buf.drain(..keep);
                    self.stats.resyncs += 1;
                }
                break;
            };
            if pos > 0 {
                self.buf.drain(..pos);
                self.stats.resyncs += 1;
            }

            // 2) 至少要有 4 字节头
            if self.buf.len() < 4 {
                break;
            }
            let len = self.buf[3] as usize;
            if len > MAX_LEN {
                // 非法长度：丢掉这一个 0xAA，继续找
                self.buf.drain(..1);
                self.stats.bad_len += 1;
                continue;
            }

            // 3) 等整帧
            let total = len + OVERHEAD;
            if self.buf.len() < total {
                break;
            }

            // 4) 校验
            let expect = crc8::crc8(&self.buf[..len + 4]);
            let got = self.buf[len + 4];
            if expect == got {
                out.push(Frame {
                    cmd: self.buf[2],
                    payload: self.buf[4..4 + len].to_vec(),
                });
                self.stats.frames += 1;
            } else {
                self.stats.crc_errors += 1;
            }
            self.buf.drain(..total);
        }

        out
    }
}

fn find_header(b: &[u8]) -> Option<usize> {
    if b.len() < 2 {
        return None;
    }
    b.windows(2).position(|w| w == HEADER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let f = Frame::new(0xA0, vec![238]);
        let bytes = f.encode().unwrap();
        assert_eq!(bytes, vec![0xAA, 0x11, 0xA0, 0x01, 0xEE, 0xBF]);
        assert_eq!(Frame::decode(&bytes).unwrap(), f);
    }

    #[test]
    fn decoder_handles_split_and_noise() {
        let mut d = FrameDecoder::new();
        let bytes = Frame::new(0x19, vec![0x01]).encode().unwrap();
        assert!(d.push(&[0x00, 0xAA]).is_empty());
        let got = d.push(&bytes[1..]);
        // 第一段留下了 0xAA，拼上剩余字节后应还原
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].cmd, 0x19);
    }

    #[test]
    fn decoder_drops_bad_crc() {
        let mut d = FrameDecoder::new();
        let mut bytes = Frame::new(0x19, vec![0x01]).encode().unwrap();
        *bytes.last_mut().unwrap() ^= 0xFF;
        assert!(d.push(&bytes).is_empty());
        assert_eq!(d.stats.crc_errors, 1);
    }
}
