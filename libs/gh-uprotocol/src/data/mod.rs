//! 0x0B 数据包解析：8 字节包头 + 若干条帧记录（非压缩 / 4bit 差分 / 拼接）

pub mod zip;

use crate::error::DataError;
use std::collections::HashMap;

pub const PACKET_HEADER_LEN: usize = 8;

#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub function_id: u8,
    pub data_type: u8,
    pub channel_mask: u32,
    pub package_type: u8,
    pub data_len: u8,
}

impl PacketHeader {
    pub fn parse(b: &[u8]) -> Result<Self, DataError> {
        if b.len() < PACKET_HEADER_LEN {
            return Err(DataError::ShortHeader);
        }
        Ok(Self {
            function_id: b[0],
            data_type: b[1],
            channel_mask: u32::from_be_bytes([b[2], b[3], b[4], b[5]]),
            package_type: b[6],
            data_len: b[7],
        })
    }

    pub fn has_gs(&self) -> bool {
        self.data_type & 0x01 != 0
    }
    pub fn has_algo(&self) -> bool {
        self.data_type & 0x02 != 0
    }
    pub fn has_agc(&self) -> bool {
        self.data_type & 0x04 != 0
    }
    pub fn has_amb(&self) -> bool {
        self.data_type & 0x08 != 0
    }
    pub fn has_gyro(&self) -> bool {
        self.data_type & 0x10 != 0
    }
    pub fn has_cap(&self) -> bool {
        self.data_type & 0x20 != 0
    }
    pub fn has_temp(&self) -> bool {
        self.data_type & 0x40 != 0
    }
    pub fn zip(&self) -> bool {
        self.package_type & 0x01 != 0
    }
    /// 本包首帧为绝对帧
    pub fn odd_even(&self) -> bool {
        self.package_type & 0x02 != 0
    }
    /// FIFO package 模式：rawdata 段只有 1 字节包号
    pub fn fifo_package(&self) -> bool {
        (self.package_type >> 2) & 0x07 == 1
    }
    pub fn splice_cnt(&self) -> u8 {
        (self.package_type >> 3) & 0x03
    }
    pub fn splice_over(&self) -> bool {
        self.package_type & 0x20 != 0
    }
    /// 通道数 = 掩码中 1 的个数。
    ///
    /// 设备写的是"通道位图"而不是连续位段：实测 HR 4 通道时掩码为 0x33
    /// （bit0/1/4/5），按最高位置位+1 会算成 6，导致整包解析失败。
    pub fn channels(&self) -> usize {
        let n = self.channel_mask.count_ones() as usize;
        if n == 0 {
            1
        } else {
            n
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Record {
    pub function_id: u8,
    pub frame_id: u8,
    pub slot_cfg: Option<i32>,
    pub gs: Option<[i16; 3]>,
    pub gyro: Option<[i16; 3]>,
    pub cap: Option<[u32; 4]>,
    pub temp: Option<[u32; 4]>,
    pub raw: Vec<u32>,
    pub agc: Vec<u32>,
    pub flags: [u32; 5],
    pub algo: Vec<(u8, i32)>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DataStats {
    pub packets: u64,
    pub records: u64,
    pub crc_errors: u64,
    pub bad_packets: u64,
    pub splice_fragments: u64,
    pub splice_aborts: u64,
    pub lost_frames: u64,
}

#[derive(Debug, Default, Clone)]
pub struct ZipState {
    pub raw: Vec<i32>,
    pub tag: Vec<u8>,
    pub agc: Vec<i32>,
    pub init: bool,
}

pub struct DataDecoder {
    zip: HashMap<u8, ZipState>,
    splice: HashMap<u8, Vec<u8>>,
    last_frame: HashMap<u8, u8>,
    /// 覆盖通道数（设备包头未直接携带通道数）
    pub chnl_override: Option<usize>,
    pub stats: DataStats,
}

impl Default for DataDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl DataDecoder {
    pub fn new() -> Self {
        Self {
            zip: HashMap::new(),
            splice: HashMap::new(),
            last_frame: HashMap::new(),
            chnl_override: None,
            stats: DataStats::default(),
        }
    }

    /// 喂入一个 0x0B 的 payload，返回本次可解析出的完整记录
    pub fn feed(&mut self, payload: &[u8]) -> Result<Vec<Record>, DataError> {
        let h = PacketHeader::parse(payload)?;
        self.stats.packets += 1;

        let end = PACKET_HEADER_LEN + h.data_len as usize;
        let body: &[u8] = if payload.len() >= end {
            &payload[PACKET_HEADER_LEN..end]
        } else {
            self.stats.bad_packets += 1;
            &payload[PACKET_HEADER_LEN..]
        };

        // 拼接包：先攒齐，再整体解析
        if h.splice_cnt() > 0 || h.splice_over() {
            self.stats.splice_fragments += 1;
            let buf = self.splice.entry(h.function_id).or_default();
            buf.extend_from_slice(body);
            if !h.splice_over() {
                return Ok(Vec::new());
            }
            let full = std::mem::take(buf);
            return self.decode_frames(&h, &full).map_err(|e| {
                self.stats.splice_aborts += 1;
                e
            });
        }

        self.decode_frames(&h, body)
    }

    fn chnl(&self, h: &PacketHeader) -> usize {
        self.chnl_override.unwrap_or_else(|| h.channels())
    }

    fn decode_frames(&mut self, h: &PacketHeader, body: &[u8]) -> Result<Vec<Record>, DataError> {
        let chnl = self.chnl(h);
        if chnl == 0 || chnl > 32 {
            return Err(DataError::BadPackageType(h.package_type));
        }

        let mut out = Vec::new();
        let mut pos = 0usize;
        let mut first = true;

        while pos < body.len() {
            let state = self.zip.entry(h.function_id).or_default();
            let (rec, used) = if h.zip() {
                decode_zip_record(body, pos, h, chnl, state, first)?
            } else {
                decode_plain_record(body, pos, h, chnl)?
            };
            if used == 0 {
                break;
            }
            pos += used;

            // 帧号连续性（仅统计）
            if let Some(prev) = self.last_frame.insert(h.function_id, rec.frame_id) {
                let expect = prev.wrapping_add(1);
                if rec.frame_id != expect {
                    self.stats.lost_frames +=
                        rec.frame_id.wrapping_sub(expect) as u64 % 256;
                }
            }

            self.stats.records += 1;
            out.push(rec);
            first = false;
        }

        if pos != body.len() && !body.is_empty() {
            self.stats.bad_packets += 1;
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// 非压缩帧
// ---------------------------------------------------------------------------

fn need(b: &[u8], pos: usize, n: usize, what: &'static str) -> Result<(), DataError> {
    if pos + n > b.len() {
        return Err(DataError::Truncated {
            what,
            need: n,
            got: b.len().saturating_sub(pos),
        });
    }
    Ok(())
}

fn be16(b: &[u8], pos: usize) -> i16 {
    i16::from_be_bytes([b[pos], b[pos + 1]])
}

fn be24(b: &[u8], pos: usize) -> u32 {
    ((b[pos] as u32) << 16) | ((b[pos + 1] as u32) << 8) | b[pos + 2] as u32
}

fn le32(b: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes([b[pos], b[pos + 1], b[pos + 2], b[pos + 3]])
}

/// rawdata 段是 32 位大端（高 8 位为 tag/标志）
fn be32(b: &[u8], pos: usize) -> u32 {
    u32::from_be_bytes([b[pos], b[pos + 1], b[pos + 2], b[pos + 3]])
}

fn read_common_tail(
    b: &[u8],
    mut p: usize,
    h: &PacketHeader,
    _chnl: usize,
    rec: &mut Record,
) -> Result<usize, DataError> {
    // g-sensor / gyro
    if h.has_gs() {
        need(b, p, 6, "gs")?;
        rec.gs = Some([be16(b, p), be16(b, p + 2), be16(b, p + 4)]);
        p += 6;
        if h.has_gyro() {
            need(b, p, 6, "gyro")?;
            rec.gyro = Some([be16(b, p), be16(b, p + 2), be16(b, p + 4)]);
            p += 6;
        }
    }
    // cap
    if h.has_cap() {
        need(b, p, 12, "cap")?;
        rec.cap = Some([be24(b, p), be24(b, p + 3), be24(b, p + 6), be24(b, p + 9)]);
        p += 12;
    }
    // temp
    if h.has_temp() {
        need(b, p, 12, "temp")?;
        rec.temp = Some([be24(b, p), be24(b, p + 3), be24(b, p + 6), be24(b, p + 9)]);
        p += 12;
    }
    Ok(p)
}

fn decode_plain_record(
    b: &[u8],
    pos: usize,
    h: &PacketHeader,
    chnl: usize,
) -> Result<(Record, usize), DataError> {
    need(b, pos, 1, "frame_id")?;
    let mut rec = Record {
        function_id: h.function_id,
        frame_id: b[pos],
        raw: vec![0; chnl],
        agc: vec![0; chnl],
        ..Default::default()
    };
    let mut p = pos + 1;

    p = read_common_tail(b, p, h, chnl, &mut rec)?;

    // rawdata
    if h.fifo_package() {
        need(b, p, 1, "fifo_id")?;
        rec.raw = vec![b[p] as u32];
        p += 1;
    } else {
        need(b, p, chnl * 4, "raw")?;
        for i in 0..chnl {
            rec.raw[i] = be32(b, p + i * 4) & 0x00FF_FFFF;
        }
        p += chnl * 4;
    }

    // agc
    if h.has_agc() {
        need(b, p, chnl * 4, "agc")?;
        for i in 0..chnl {
            rec.agc[i] = le32(b, p + i * 4);
        }
        p += chnl * 4;
    }

    // ambience
    if h.has_amb() {
        need(b, p, chnl * 3, "amb")?;
        p += chnl * 3;
    }

    // 结果块
    p = parse_result_block(b, p, &mut rec)?;

    Ok((rec, p - pos))
}

// ---------------------------------------------------------------------------
// 压缩帧
// ---------------------------------------------------------------------------

fn decode_zip_record(
    b: &[u8],
    pos: usize,
    h: &PacketHeader,
    chnl: usize,
    st: &mut ZipState,
    first_in_packet: bool,
) -> Result<(Record, usize), DataError> {
    need(b, pos, 1, "frame_id")?;
    let mut rec = Record {
        function_id: h.function_id,
        frame_id: b[pos],
        raw: vec![0; chnl],
        agc: vec![0; chnl],
        ..Default::default()
    };
    let mut p = pos + 1;

    p = read_common_tail(b, p, h, chnl, &mut rec)?;

    let absolute = first_in_packet && h.odd_even();

    if h.fifo_package() {
        need(b, p, 1, "fifo_id")?;
        rec.raw = vec![b[p] as u32];
        p += 1;
        st.init = false;
    } else if absolute {
        need(b, p, chnl * 4, "raw(abs)")?;
        zip::ensure(st, chnl);
        for i in 0..chnl {
            let v = u32::from_be_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]]);
            p += 4;
            st.raw[i] = (v & 0x00FF_FFFF) as i32;
            st.tag[i] = (v >> 24) as u8;
            rec.raw[i] = v & 0x00FF_FFFF;
        }
        st.init = true;
    } else {
        zip::ensure(st, chnl);
        let (vals, used) = zip::decode_raw_block(&b[..], p, chnl, st)?;
        p += used;
        rec.raw.copy_from_slice(&vals);
    }

    // AGC
    if h.has_agc() {
        if absolute {
            need(b, p, chnl * 4, "agc(abs)")?;
            for i in 0..chnl {
                st.agc[i] = le32(b, p + i * 4) as i32;
                rec.agc[i] = st.agc[i] as u32;
            }
            p += chnl * 4;
        } else {
            let (vals, used) = zip::decode_agc_block(&b[..], p, chnl, st)?;
            p += used;
            for i in 0..chnl {
                rec.agc[i] = vals[i] as u32;
            }
        }
    }

    // ambience
    if h.has_amb() {
        need(b, p, chnl * 3, "amb")?;
        p += chnl * 3;
    }

    p = parse_result_block(b, p, &mut rec)?;

    Ok((rec, p - pos))
}

// ---------------------------------------------------------------------------
// 结果块
// ---------------------------------------------------------------------------

fn parse_result_block(
    b: &[u8],
    pos: usize,
    rec: &mut Record,
) -> Result<usize, DataError> {
    need(b, pos, 1, "result_len")?;
    let len = b[pos] as usize;
    let start = pos + 1;
    let end = start + len;
    if end > b.len() {
        return Err(DataError::Truncated {
            what: "result",
            need: len,
            got: b.len().saturating_sub(start),
        });
    }
    let mut p = start;
    while p + 5 <= end {
        let ty = b[p];
        let v = i32::from_le_bytes([b[p + 1], b[p + 2], b[p + 3], b[p + 4]]);
        p += 5;
        if ty & 0x80 != 0 {
            rec.algo.push((ty & 0x7F, v));
        } else if (2..=4).contains(&ty) {
            rec.flags[ty as usize] = v as u32;
        }
    }
    Ok(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_bits() {
        let h = PacketHeader {
            function_id: 1,
            data_type: 0x07,
            channel_mask: 0x3,
            package_type: 0x02,
            data_len: 10,
        };
        assert!(h.has_gs() && h.has_algo() && h.has_agc());
        assert!(!h.has_cap() && !h.zip());
        assert!(h.odd_even());
        assert_eq!(h.channels(), 2);
    }

    #[test]
    fn plain_record() {
        // frame_id=7, gs=(1,2,3), raw ch0=0x010203, ch1=0x040506,
        // agc ch0=0x0000000F, ch1=0x00001200, result len=0
        let mut b = vec![7u8];
        for v in [1i16, 2, 3] {
            b.extend_from_slice(&v.to_be_bytes());
        }
        b.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x00, 0x04, 0x05, 0x06]);
        b.extend_from_slice(&[0x0F, 0, 0, 0, 0x00, 0x12, 0, 0]);
        b.push(0); // result len

        let h = PacketHeader {
            function_id: 1,
            data_type: 0x07,
            channel_mask: 0x3,
            package_type: 0,
            data_len: b.len() as u8,
        };
        let (rec, used) = decode_plain_record(&b, 0, &h, 2).unwrap();
        assert_eq!(used, b.len());
        assert_eq!(rec.frame_id, 7);
        assert_eq!(rec.gs, Some([1, 2, 3]));
        assert_eq!(rec.raw, vec![0x010203, 0x040506]);
        assert_eq!(rec.agc, vec![0x0000_000F, 0x0000_1200]);
    }
}
