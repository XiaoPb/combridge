//! GH-RPC Type Definitions
//!
//! G协议数据类型定义，参考C代码gh_data_common.h

pub use rpc::types::*;

/// AGC信息结构（参考C代码gh_agc_info_t）
///
/// 存储格式（64位）：
/// word0 (bits 0-31):
///   - gain_code: 4 bits (0-3)
///   - bg_cancel_range: 2 bits (4-5)
///   - dc_cancel_range: 2 bits (6-7)
///   - dc_cancel_code: 8 bits (8-15)
///   - led_drv0: 8 bits (16-23)
///   - led_drv1: 8 bits (24-31)
/// word1 (bits 32-63):
///   - bg_cancel_code: 8 bits (32-39)
///   - tia_gain: 3 bits (40-42)
///   - reserved: 5 bits (43-47)
///
/// 注意：传输格式使用 gh_agc_upload_t，解码时：
/// - led_drv_fs 从传输格式提取，存储到帧级别 GhFuncFrame.led_drv_fs
/// - bg_cancel_code 和 tia_gain 不在传输格式中，解码时设为默认值
#[derive(Debug, Clone, Copy, Default)]
pub struct GhAgcInfo {
    pub gain_code: u8,
    pub bg_cancel_range: u8,
    pub dc_cancel_range: u8,
    pub dc_cancel_code: u8,
    pub led_drv0: u8,
    pub led_drv1: u8,
    pub bg_cancel_code: u8,
    pub tia_gain: u8,
}

impl GhAgcInfo {
    /// 从传输格式（gh_agc_upload_t）解码 AGC 信息
    /// 返回 (GhAgcInfo, led_drv_fs)，其中 led_drv_fs 是帧级别数据
    pub fn from_bytes(data: &[u8]) -> Result<(Self, u8), DecodeError> {
        if data.len() < 8 {
            return Err(DecodeError::InsufficientData);
        }

        let word0 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let word1 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        let led_drv_fs = ((word0 >> 16) & 0xFF) as u8;
        
        Ok((Self {
            gain_code: (word0 & 0x0F) as u8,
            bg_cancel_range: ((word0 >> 4) & 0x03) as u8,
            dc_cancel_range: ((word0 >> 6) & 0x03) as u8,
            dc_cancel_code: ((word0 >> 8) & 0xFF) as u8,
            led_drv0: ((word0 >> 24) & 0xFF) as u8,
            led_drv1: (word1 & 0xFF) as u8,
            bg_cancel_code: 0,
            tia_gain: 0,
        }, led_drv_fs))
    }

    /// 编码为传输格式（gh_agc_upload_t）
    pub fn to_bytes(&self, led_drv_fs: u8) -> [u8; 8] {
        let word0 = (self.gain_code as u32)
            | ((self.bg_cancel_range as u32) << 4)
            | ((self.dc_cancel_range as u32) << 6)
            | ((self.dc_cancel_code as u32) << 8)
            | ((led_drv_fs as u32) << 16)
            | ((self.led_drv0 as u32) << 24);
        let word1 = self.led_drv1 as u32;

        let mut result = [0u8; 8];
        result[0..4].copy_from_slice(&word0.to_le_bytes());
        result[4..8].copy_from_slice(&word1.to_le_bytes());
        result
    }
}

/// 帧数据标志（参考C代码gh_frame_data_flag_t）
#[derive(Debug, Clone, Copy, Default)]
pub struct GhFrameDataFlag {
    pub led_adj_flag: bool,
    pub sa_flag: bool,
    pub param_change_flag: bool,
    pub dre_update: bool,
    pub skip_ok_flag: bool,
}

impl GhFrameDataFlag {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            led_adj_flag: (byte & 0x01) != 0,
            sa_flag: ((byte >> 1) & 0x01) != 0,
            param_change_flag: ((byte >> 2) & 0x01) != 0,
            dre_update: ((byte >> 3) & 0x01) != 0,
            skip_ok_flag: ((byte >> 4) & 0x01) != 0,
        }
    }

    pub fn to_byte(&self) -> u8 {
        (self.led_adj_flag as u8)
            | ((self.sa_flag as u8) << 1)
            | ((self.param_change_flag as u8) << 2)
            | ((self.dre_update as u8) << 3)
            | ((self.skip_ok_flag as u8) << 4)
    }
}

/// 帧数据结构（参考C代码gh_frame_data_t）
#[derive(Debug, Clone, Default)]
pub struct GhFrameData {
    pub ipd_pa: i32,
    pub rawdata: i32,
    pub flag: GhFrameDataFlag,
    pub agc_info: GhAgcInfo,
}

impl GhFrameData {
    pub fn from_bytes(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 17 {
            return Err(DecodeError::InsufficientData);
        }

        let ipd_pa = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let rawdata = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let flag = GhFrameDataFlag::from_byte(data[8]);
        let (agc_info, _led_drv_fs) = GhAgcInfo::from_bytes(&data[9..17])?;

        Ok(Self {
            ipd_pa,
            rawdata,
            flag,
            agc_info,
        })
    }

    pub fn to_bytes(&self, led_drv_fs: u8) -> Vec<u8> {
        let mut result = Vec::with_capacity(17);
        result.extend_from_slice(&self.ipd_pa.to_le_bytes());
        result.extend_from_slice(&self.rawdata.to_le_bytes());
        result.push(self.flag.to_byte());
        result.extend_from_slice(&self.agc_info.to_bytes(led_drv_fs));
        result
    }
}

/// 功能固定索引（参考C代码gh_func_fix_idx_e）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GhFuncFixIdx {
    Adt = 0,
    Hr = 1,
    Spo2 = 2,
    Hrv = 3,
    Gnadt = 4,
    Irnadt = 5,
    AlgoMax = 6,
    Test2 = 7,
    PpgCfg0 = 8,
    PpgCfg1 = 9,
    PpgCfg2 = 10,
    PpgCfg3 = 11,
    PpgCfg4 = 12,
    PpgCfg5 = 13,
    PpgCfg6 = 14,
    PpgCfg7 = 15,
    CapCfg = 16,
    Max = 17,
}

/// Test1 与 AlgoMax 共享相同的索引值 6
pub const GH_FUNC_FIX_IDX_TEST1: GhFuncFixIdx = GhFuncFixIdx::AlgoMax;

impl Default for GhFuncFixIdx {
    fn default() -> Self {
        Self::Adt
    }
}

impl From<u8> for GhFuncFixIdx {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Adt,
            1 => Self::Hr,
            2 => Self::Spo2,
            3 => Self::Hrv,
            4 => Self::Gnadt,
            5 => Self::Irnadt,
            6 => Self::AlgoMax,
            7 => Self::Test2,
            8 => Self::PpgCfg0,
            9 => Self::PpgCfg1,
            10 => Self::PpgCfg2,
            11 => Self::PpgCfg3,
            12 => Self::PpgCfg4,
            13 => Self::PpgCfg5,
            14 => Self::PpgCfg6,
            15 => Self::PpgCfg7,
            16 => Self::CapCfg,
            _ => Self::Max,
        }
    }
}

/// G传感器数据（参考C代码gh_gsensor_data_t）
#[derive(Debug, Clone, Copy, Default)]
pub struct GhGsensorData {
    pub acc: [i16; 3],
}

impl GhGsensorData {
    pub fn from_bytes(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 6 {
            return Err(DecodeError::InsufficientData);
        }

        Ok(Self {
            acc: [
                i16::from_le_bytes([data[0], data[1]]),
                i16::from_le_bytes([data[2], data[3]]),
                i16::from_le_bytes([data[4], data[5]]),
            ],
        })
    }

    pub fn to_bytes(&self) -> [u8; 6] {
        let mut result = [0u8; 6];
        result[0..2].copy_from_slice(&self.acc[0].to_le_bytes());
        result[2..4].copy_from_slice(&self.acc[1].to_le_bytes());
        result[4..6].copy_from_slice(&self.acc[2].to_le_bytes());
        result
    }
}

/// 功能帧结构（参考C代码gh_func_frame_t）
#[derive(Debug, Clone, Default)]
pub struct GhFuncFrame {
    pub frame_cnt: u32,
    pub timestamp: u64,
    pub gsensor_data: GhGsensorData,
    pub id: GhFuncFixIdx,
    pub ch_num: u8,
    pub ch_max: u8,
    pub gsensor_en: u8,
    pub fifo_end_flag: u8,
    pub led_drv_fs: [u8; 2],
    pub data: Vec<GhFrameData>,
    pub algo_data: Vec<i32>,
}

impl GhFuncFrame {
    pub fn from_bytes(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 32 {
            return Err(DecodeError::InsufficientData);
        }

        let frame_cnt = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let timestamp = u64::from_le_bytes([
            data[4], data[5], data[6], data[7],
            data[8], data[9], data[10], data[11],
        ]);

        let gsensor_data = GhGsensorData::from_bytes(&data[12..18])?;

        let id = GhFuncFixIdx::from(data[18]);
        let ch_num = data[19];
        let ch_max = data[20];
        let gsensor_en = data[21];
        let fifo_end_flag = data[22];
        let led_drv_fs = [data[23], data[24]];

        let header_size = 25;
        let frame_data_size = 17;
        let mut frames = Vec::new();

        let mut offset = header_size;
        while offset + frame_data_size <= data.len() {
            if let Ok(frame_data) = GhFrameData::from_bytes(&data[offset..offset + frame_data_size]) {
                frames.push(frame_data);
            }
            offset += frame_data_size;
        }

        Ok(Self {
            frame_cnt,
            timestamp,
            gsensor_data,
            id,
            ch_num,
            ch_max,
            gsensor_en,
            fifo_end_flag,
            led_drv_fs,
            data: frames,
            algo_data: Vec::new(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(25 + self.data.len() * 17 + self.algo_data.len() * 4);

        result.extend_from_slice(&self.frame_cnt.to_le_bytes());
        result.extend_from_slice(&self.timestamp.to_le_bytes());
        result.extend_from_slice(&self.gsensor_data.to_bytes());
        result.push(self.id as u8);
        result.push(self.ch_num);
        result.push(self.ch_max);
        result.push(self.gsensor_en);
        result.push(self.fifo_end_flag);
        result.extend_from_slice(&self.led_drv_fs);

        for frame in &self.data {
            result.extend_from_slice(&frame.to_bytes(self.led_drv_fs[0]));
        }

        for algo in &self.algo_data {
            result.extend_from_slice(&algo.to_le_bytes());
        }

        result
    }
}

/// 解码错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    InsufficientData,
    InvalidFormat,
    InvalidChannelCount,
    CrcMismatch,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientData => write!(f, "数据不足"),
            Self::InvalidFormat => write!(f, "格式无效"),
            Self::InvalidChannelCount => write!(f, "通道数无效"),
            Self::CrcMismatch => write!(f, "CRC校验失败"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gh_agc_info_roundtrip() {
        let info = GhAgcInfo {
            gain_code: 5,
            bg_cancel_range: 2,
            dc_cancel_range: 1,
            dc_cancel_code: 100,
            led_drv0: 50,
            led_drv1: 60,
            bg_cancel_code: 200,
            tia_gain: 3,
        };

        let bytes = info.to_bytes();
        let decoded = GhAgcInfo::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.gain_code, 5);
        assert_eq!(decoded.bg_cancel_range, 2);
        assert_eq!(decoded.dc_cancel_range, 1);
        assert_eq!(decoded.dc_cancel_code, 100);
        assert_eq!(decoded.led_drv0, 50);
        assert_eq!(decoded.led_drv1, 60);
        assert_eq!(decoded.bg_cancel_code, 200);
        assert_eq!(decoded.tia_gain, 3);
    }

    #[test]
    fn test_gh_frame_data_flag_roundtrip() {
        let flag = GhFrameDataFlag {
            led_adj_flag: true,
            sa_flag: false,
            param_change_flag: true,
            dre_update: false,
            skip_ok_flag: true,
        };

        let byte = flag.to_byte();
        let decoded = GhFrameDataFlag::from_byte(byte);

        assert_eq!(decoded.led_adj_flag, true);
        assert_eq!(decoded.sa_flag, false);
        assert_eq!(decoded.param_change_flag, true);
        assert_eq!(decoded.dre_update, false);
        assert_eq!(decoded.skip_ok_flag, true);
    }

    #[test]
    fn test_gh_gsensor_data_roundtrip() {
        let data = GhGsensorData {
            acc: [100, -200, 300],
        };

        let bytes = data.to_bytes();
        let decoded = GhGsensorData::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.acc, [100, -200, 300]);
    }

    #[test]
    fn test_gh_func_fix_idx_conversion() {
        assert_eq!(GhFuncFixIdx::from(0), GhFuncFixIdx::Adt);
        assert_eq!(GhFuncFixIdx::from(1), GhFuncFixIdx::Hr);
        assert_eq!(GhFuncFixIdx::from(2), GhFuncFixIdx::Spo2);
        assert_eq!(GhFuncFixIdx::from(3), GhFuncFixIdx::Hrv);
        assert_eq!(GhFuncFixIdx::from(100), GhFuncFixIdx::Max);
    }
}
