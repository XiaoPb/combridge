//! 4bit 半字节差分（uprotocol zip）解码
//!
//! 块结构（rawdata）：
//! ```text
//! [N][marker][tag x chnl][nibbles...]
//! ```
//! AGC 块没有 marker：`[N][nibbles...]`

use super::ZipState;
use crate::error::DataError;

pub fn ensure(st: &mut ZipState, chnl: usize) {
    if st.raw.len() != chnl {
        st.raw = vec![0; chnl];
        st.tag = vec![0; chnl];
        st.agc = vec![0; chnl];
        st.init = false;
    }
}

struct Nib<'a> {
    b: &'a [u8],
    pos: usize,
    high: bool,
}

impl<'a> Nib<'a> {
    fn new(b: &'a [u8], start: usize) -> Self {
        Self {
            b,
            pos: start,
            high: true,
        }
    }

    fn get(&mut self) -> Result<u8, DataError> {
        if self.pos >= self.b.len() {
            return Err(DataError::Truncated {
                what: "nibble",
                need: 1,
                got: 0,
            });
        }
        let v = if self.high {
            self.b[self.pos] >> 4
        } else {
            self.b[self.pos] & 0x0F
        };
        if self.high {
            self.high = false;
        } else {
            self.pos += 1;
            self.high = true;
        }
        Ok(v)
    }

    fn bytes_used(&self, start: usize) -> usize {
        self.pos - start + if self.high { 0 } else { 1 }
    }
}

/// 解码 chnl 个通道：type 半字节 + (type/2+1) 个幅值半字节
fn decode_values(
    b: &[u8],
    start: usize,
    chnl: usize,
    prev: &[i32],
) -> Result<(Vec<i32>, usize), DataError> {
    let mut nib = Nib::new(b, start);
    let mut out = vec![0i32; chnl];

    for i in 0..chnl {
        let ty = nib.get()?;
        let dir = ty & 1;
        let groups = (ty >> 1) as usize + 1;
        let mut mag: u32 = 0;
        for _ in 0..groups {
            mag = (mag << 4) | nib.get()? as u32;
        }
        let base = *prev.get(i).unwrap_or(&0) as i64;
        let v = if dir == 0 {
            base + mag as i64
        } else {
            base - mag as i64
        };
        out[i] = v as i32;
    }

    let used = nib.bytes_used(start);
    Ok((out, used))
}

fn check_block(b: &[u8], pos: usize) -> Result<usize, DataError> {
    if pos + 2 > b.len() {
        return Err(DataError::Truncated {
            what: "zip header",
            need: 2,
            got: b.len().saturating_sub(pos),
        });
    }
    let n = b[pos] as usize;
    if pos + 1 + n > b.len() {
        return Err(DataError::Truncated {
            what: "zip block",
            need: n,
            got: b.len().saturating_sub(pos + 1),
        });
    }
    Ok(n)
}

/// 返回 (每通道 24bit 值, 消耗字节数)
pub fn decode_raw_block(
    b: &[u8],
    pos: usize,
    chnl: usize,
    st: &mut ZipState,
) -> Result<(Vec<u32>, usize), DataError> {
    let n = check_block(b, pos)?;
    let marker = b[pos + 1];
    let mut q = pos + 2;
    let end = pos + 1 + n;

    if marker == 1 {
        if q + chnl > end {
            return Err(DataError::Truncated {
                what: "zip tags",
                need: chnl,
                got: end.saturating_sub(q),
            });
        }
        st.tag[..chnl].copy_from_slice(&b[q..q + chnl]);
        q += chnl;
    }

    let (vals, used) = decode_values(b, q, chnl, &st.raw)?;
    if q + used > end {
        return Err(DataError::Truncated {
            what: "zip nibbles",
            need: used,
            got: end.saturating_sub(q),
        });
    }

    for i in 0..chnl {
        st.raw[i] = vals[i];
    }

    let out: Vec<u32> = vals.iter().map(|v| *v as u32).collect();
    Ok((out, 1 + n))
}

/// 返回 (每通道原始 32bit AGC, 消耗字节数)
pub fn decode_agc_block(
    b: &[u8],
    pos: usize,
    chnl: usize,
    st: &mut ZipState,
) -> Result<(Vec<i32>, usize), DataError> {
    let n = check_block(b, pos)?;
    let end = pos + 1 + n;

    let (vals, used) = decode_values(b, pos + 1, chnl, &st.agc)?;
    if pos + 1 + used > end {
        return Err(DataError::Truncated {
            what: "agc nibbles",
            need: used,
            got: end.saturating_sub(pos + 1),
        });
    }

    st.agc[..chnl].copy_from_slice(&vals[..chnl]);
    Ok((vals, 1 + n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_doc_example() {
        // 文档 6.5 的例子：prev={1000,2000}，块 03 00 05 1A
        let mut st = ZipState {
            raw: vec![1000, 2000],
            tag: vec![0, 0],
            agc: vec![0, 0],
            init: true,
        };
        let block = [0x03, 0x00, 0x05, 0x1A];
        let (vals, used) = decode_raw_block(&block, 0, 2, &mut st).unwrap();
        assert_eq!(used, 4);
        assert_eq!(vals, vec![1005, 1990]);
        assert_eq!(st.raw, vec![1005, 1990]);
    }

    #[test]
    fn decode_with_tags() {
        // N=1+2+2=5, marker=1, tags={0x12,0x34}, 两个通道各 1 个半字节 (0,5)(1,A)
        let mut st = ZipState {
            raw: vec![0, 20],
            tag: vec![0, 0],
            agc: vec![0, 0],
            init: true,
        };
        let block = [0x05, 0x01, 0x12, 0x34, 0x05, 0x1A];
        let (vals, used) = decode_raw_block(&block, 0, 2, &mut st).unwrap();
        assert_eq!(used, 6);
        assert_eq!(vals, vec![5, 10]);
        assert_eq!(st.tag, vec![0x12, 0x34]);
    }

    #[test]
    fn decode_negative_delta() {
        let mut st = ZipState {
            raw: vec![1000],
            tag: vec![0],
            agc: vec![0],
            init: true,
        };
        // n=2 (marker + 1 个半字节字节), marker=0, type=1 (dir=1,k=0), mag=10
        let block = [0x02, 0x00, 0x1A];
        let (vals, _) = decode_raw_block(&block, 0, 1, &mut st).unwrap();
        assert_eq!(vals, vec![990]);
    }
}
