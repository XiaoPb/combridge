//! G协议帧解码器
//!
//! 用于解码G协议帧数据
//! 数据格式：使用 varint 编码

use crate::types::{DecodeError, GhFrameData, GhFrameDataFlag, GhFuncFrame, GhFuncFixIdx};
use rpc::{LogCallback, LogLevel, PackHeader, NullLogger};
use std::sync::{Arc, Mutex};

const MAX_CHANNELS: usize = 32;
const MAX_GS_DATA: usize = 6;
const MAX_ALGO_DATA: usize = 32;

pub fn varint_decode(buffer: &[u8], pos: &mut usize) -> Result<u32, DecodeError> {
    let mut value: u32 = 0;
    let mut shift = 0;

    loop {
        if *pos >= buffer.len() {
            return Err(DecodeError::InsufficientData);
        }

        let byte = buffer[*pos];
        *pos += 1;
        value |= ((byte & 0x7F) as u32) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift >= 35 {
            return Err(DecodeError::InvalidFormat);
        }
    }

    Ok(value)
}

pub fn zigzag_decode(x: u32) -> i32 {
    ((x >> 1) as i32) ^ (-((x & 1) as i32))
}

#[derive(Debug, Clone, Default)]
struct DataFrame {
    pack_header: PackHeader,
    rawdata: Vec<i32>,
    rawdata_size: usize,
    phy_value: Vec<i32>,
    phy_value_size: usize,
    gs_data: Vec<i32>,
    gs_data_size: usize,
    flags: Vec<u32>,
    flag_data_bits: usize,
    algo_data: Vec<i32>,
    algo_data_bits: usize,
    agc_info: Vec<i32>,
    agc_info_high: Vec<i32>,
    agc_info_size: usize,
    timestamp: i32,
    timestamp_high: i32,
    frame_id: i32,
    function_id: i32,
    slot_cfg: i32,
}

#[derive(Clone, Default)]
struct DecoderState {
    pos: usize,
    start_flag: bool,
    last_frame_id: i32,
    last_rawdata: [i32; MAX_CHANNELS],
    last_phy_value: [i32; MAX_CHANNELS],
    last_timestamp: i32,
    last_timestamp_high: i32,
    last_gs_data: [i32; MAX_GS_DATA],
    last_gs_data_size: usize,
    last_flags: [i32; MAX_CHANNELS],
    last_flag_data_bits: usize,
    last_algo_data: [i32; MAX_ALGO_DATA],
    last_agc_info: [i32; MAX_CHANNELS],
    last_agc_info_high: [i32; MAX_CHANNELS],
    last_agc_size: usize,
}

impl DecoderState {
    fn new() -> Self {
        Self::default()
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn read_varint(&mut self, buffer: &[u8]) -> Result<u32, DecodeError> {
        varint_decode(buffer, &mut self.pos)
    }

    fn read_i32(&mut self, buffer: &[u8]) -> Result<i32, DecodeError> {
        let value = self.read_varint(buffer)?;
        Ok(zigzag_decode(value))
    }

    fn read_i32_array(&mut self, buffer: &[u8], count: usize) -> Result<Vec<i32>, DecodeError> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_i32(buffer)?);
        }
        Ok(result)
    }

    fn decode_single_frame(&mut self, buffer: &[u8], logger: &dyn LogCallback) -> Result<DataFrame, DecodeError> {
        let mut frame = DataFrame::default();
        let start_pos = self.pos;

        let header_raw = self.read_varint(buffer)?;
        let header_value = zigzag_decode(header_raw) as u32;
        frame.pack_header = PackHeader::from_bits_truncate(header_value);
        
        logger.log(LogLevel::Debug, "decode", &format!(
            "pack_header: bits=0x{:08X}, rawdata_en={}, phy_value_en={}, gs_data_en={}, flags_en={}, alg_data_en={}, agc_info_en={}, timestamp_en={}, func_id_en={}, slot_cfg_en={}",
            header_value,
            frame.pack_header.contains(PackHeader::RAWDATA_EN),
            frame.pack_header.contains(PackHeader::PHY_VALUE_EN),
            frame.pack_header.contains(PackHeader::GS_DATA_EN),
            frame.pack_header.contains(PackHeader::FLAGS_EN),
            frame.pack_header.contains(PackHeader::ALG_DATA_EN),
            frame.pack_header.contains(PackHeader::AGC_INFO_EN),
            frame.pack_header.contains(PackHeader::TIMESTAMP_EN),
            frame.pack_header.contains(PackHeader::FUNC_ID_EN),
            frame.pack_header.contains(PackHeader::SLOT_CFG_EN)
        ));

        if frame.pack_header.contains(PackHeader::RAWDATA_EN) {
            frame.rawdata_size = self.read_i32(buffer)? as usize;
            if frame.rawdata_size > MAX_CHANNELS {
                frame.rawdata_size = MAX_CHANNELS;
            }
            frame.rawdata = self.read_i32_array(buffer, frame.rawdata_size)?;
            logger.log(LogLevel::Debug, "decode", &format!("rawdata_size={}, data={:?}", frame.rawdata_size, &frame.rawdata[..std::cmp::min(5, frame.rawdata_size)]));
        }

        if frame.pack_header.contains(PackHeader::PHY_VALUE_EN) {
            frame.phy_value_size = self.read_i32(buffer)? as usize;
            if frame.phy_value_size > MAX_CHANNELS {
                frame.phy_value_size = MAX_CHANNELS;
            }
            frame.phy_value = self.read_i32_array(buffer, frame.phy_value_size)?;
            logger.log(LogLevel::Debug, "decode", &format!("phy_value_size={}, data={:?}", frame.phy_value_size, &frame.phy_value[..std::cmp::min(5, frame.phy_value_size)]));
        }

        if frame.pack_header.contains(PackHeader::GS_DATA_EN) {
            frame.gs_data_size = self.read_i32(buffer)? as usize;
            if frame.gs_data_size > MAX_GS_DATA {
                frame.gs_data_size = MAX_GS_DATA;
            }
            frame.gs_data = self.read_i32_array(buffer, frame.gs_data_size)?;
            logger.log(LogLevel::Debug, "decode", &format!("gs_data_size={}, data={:?}", frame.gs_data_size, frame.gs_data));
        }

        if frame.pack_header.contains(PackHeader::FLAGS_EN) {
            frame.flag_data_bits = self.read_i32(buffer)? as usize;
            if frame.flag_data_bits > MAX_CHANNELS {
                frame.flag_data_bits = MAX_CHANNELS;
            }
            let flags_i32 = self.read_i32_array(buffer, frame.flag_data_bits)?;
            frame.flags = flags_i32.iter().map(|&v| v as u32).collect();
            logger.log(LogLevel::Debug, "decode", &format!("flag_data_bits={}, data={:?}", frame.flag_data_bits, &frame.flags[..std::cmp::min(5, frame.flags.len())]));
        }

        if frame.pack_header.contains(PackHeader::ALG_DATA_EN) {
            frame.algo_data_bits = self.read_i32(buffer)? as usize;
            if frame.algo_data_bits > MAX_ALGO_DATA {
                frame.algo_data_bits = MAX_ALGO_DATA;
            }
            frame.algo_data = self.read_i32_array(buffer, frame.algo_data_bits)?;
            logger.log(LogLevel::Debug, "decode", &format!("algo_data_bits={}", frame.algo_data_bits));
        }

        if frame.pack_header.contains(PackHeader::AGC_INFO_EN) {
            frame.agc_info_size = self.read_i32(buffer)? as usize;
            if frame.agc_info_size > MAX_CHANNELS {
                frame.agc_info_size = MAX_CHANNELS;
            }
            frame.agc_info = self.read_i32_array(buffer, frame.agc_info_size)?;
            frame.agc_info_high = self.read_i32_array(buffer, frame.agc_info_size)?;
            logger.log(LogLevel::Debug, "decode", &format!("agc_info_size={}", frame.agc_info_size));
        }

        if frame.pack_header.contains(PackHeader::TIMESTAMP_EN) {
            frame.timestamp = self.read_i32(buffer)?;
            frame.timestamp_high = self.read_i32(buffer)?;
            logger.log(LogLevel::Debug, "decode", &format!("timestamp={}, timestamp_high={}", frame.timestamp, frame.timestamp_high));
        }

        frame.frame_id = self.read_i32(buffer)?;
        logger.log(LogLevel::Debug, "decode", &format!("frame_id={}", frame.frame_id));

        if frame.pack_header.contains(PackHeader::FUNC_ID_EN) {
            frame.function_id = self.read_i32(buffer)?;
            logger.log(LogLevel::Debug, "decode", &format!("function_id={}", frame.function_id));
        }

        if frame.pack_header.contains(PackHeader::SLOT_CFG_EN) {
            frame.slot_cfg = self.read_i32(buffer)?;
            logger.log(LogLevel::Debug, "decode", &format!("slot_cfg={}", frame.slot_cfg));
        }

        logger.log(LogLevel::Debug, "decode", &format!("frame decoded, consumed {} bytes", self.pos - start_pos));
        Ok(frame)
    }

    fn process_single_frame(&mut self, data_frame: &DataFrame) -> GhFuncFrame {
        let mut func_frame = GhFuncFrame::default();

        if self.last_frame_id >= 0 {
            let expected_next = (self.last_frame_id + 1) % 1001;
            if data_frame.frame_id != expected_next {
                self.start_flag = true;
                self.last_rawdata = [0; MAX_CHANNELS];
                self.last_phy_value = [0; MAX_CHANNELS];
                self.last_timestamp = 0;
                self.last_timestamp_high = 0;
                self.last_gs_data = [0; MAX_GS_DATA];
                self.last_flags = [0; MAX_CHANNELS];
                self.last_algo_data = [0; MAX_ALGO_DATA];
                self.last_agc_info = [0; MAX_CHANNELS];
                self.last_agc_info_high = [0; MAX_CHANNELS];
            }
        }

        func_frame.frame_cnt = data_frame.frame_id as u32;
        func_frame.id = GhFuncFixIdx::from(data_frame.function_id as u8);
        self.last_frame_id = data_frame.frame_id;

        if data_frame.pack_header.contains(PackHeader::TIMESTAMP_EN) {
            let ts_low = data_frame.timestamp as u32;
            let ts_high = data_frame.timestamp_high as u32;
            if self.start_flag {
                func_frame.timestamp = (ts_low as u64) | ((ts_high as u64) << 32);
                self.last_timestamp = data_frame.timestamp;
                self.last_timestamp_high = data_frame.timestamp_high;
            } else {
                let last_ts = (self.last_timestamp as u32 as u64) 
                    | ((self.last_timestamp_high as u32 as u64) << 32);
                let diff_ts_low = data_frame.timestamp as i32;
                let diff_ts_high = data_frame.timestamp_high as i32;
                let diff_ts = (diff_ts_low as u64) | ((diff_ts_high as u64) << 32);
                func_frame.timestamp = last_ts.wrapping_add(diff_ts);
                self.last_timestamp = (func_frame.timestamp & 0xFFFFFFFF) as i32;
                self.last_timestamp_high = ((func_frame.timestamp >> 32) & 0xFFFFFFFF) as i32;
            }
        }

        if data_frame.pack_header.contains(PackHeader::RAWDATA_EN) && data_frame.rawdata_size > 0 {
            func_frame.ch_num = data_frame.rawdata_size as u8;
            for i in 0..data_frame.rawdata_size {
                let mut ch_data = GhFrameData::default();
                if self.start_flag {
                    ch_data.rawdata = data_frame.rawdata[i];
                    self.last_rawdata[i] = data_frame.rawdata[i];
                } else {
                    ch_data.rawdata = self.last_rawdata[i] + data_frame.rawdata[i];
                    self.last_rawdata[i] = ch_data.rawdata;
                }
                func_frame.data.push(ch_data);
            }
        }

        if data_frame.pack_header.contains(PackHeader::PHY_VALUE_EN) && data_frame.phy_value_size > 0 {
            if func_frame.ch_num == 0 {
                func_frame.ch_num = data_frame.phy_value_size as u8;
            }
            for i in 0..data_frame.phy_value_size {
                if i >= func_frame.data.len() {
                    let mut ch_data = GhFrameData::default();
                    if self.start_flag {
                        ch_data.ipd_pa = data_frame.phy_value[i];
                        self.last_phy_value[i] = data_frame.phy_value[i];
                    } else {
                        ch_data.ipd_pa = self.last_phy_value[i] + data_frame.phy_value[i];
                        self.last_phy_value[i] = ch_data.ipd_pa;
                    }
                    func_frame.data.push(ch_data);
                } else {
                    if self.start_flag {
                        func_frame.data[i].ipd_pa = data_frame.phy_value[i];
                        self.last_phy_value[i] = data_frame.phy_value[i];
                    } else {
                        func_frame.data[i].ipd_pa = self.last_phy_value[i] + data_frame.phy_value[i];
                        self.last_phy_value[i] = func_frame.data[i].ipd_pa;
                    }
                }
            }
        }

        if data_frame.pack_header.contains(PackHeader::GS_DATA_EN) && data_frame.gs_data_size > 0 {
            self.last_gs_data_size = data_frame.gs_data_size;
            for i in 0..data_frame.gs_data_size {
                if i < 3 {
                    if self.start_flag {
                        func_frame.gsensor_data.acc[i] = data_frame.gs_data[i] as i16;
                        self.last_gs_data[i] = data_frame.gs_data[i];
                    } else {
                        let val = self.last_gs_data[i] + data_frame.gs_data[i];
                        func_frame.gsensor_data.acc[i] = val as i16;
                        self.last_gs_data[i] = val;
                    }
                }
            }
        }

        if data_frame.pack_header.contains(PackHeader::FLAGS_EN) {
            self.last_flag_data_bits = data_frame.flag_data_bits;
            for i in 0..data_frame.flag_data_bits {
                if self.start_flag {
                    self.last_flags[i] = data_frame.flags[i] as i32;
                } else {
                    self.last_flags[i] = self.last_flags[i] + data_frame.flags[i] as i32;
                }
                if i < func_frame.data.len() {
                    func_frame.data[i].flag = GhFrameDataFlag::from_byte(self.last_flags[i] as u8);
                }
            }
        } else if self.last_flag_data_bits > 0 {
            for i in 0..self.last_flag_data_bits {
                if i < func_frame.data.len() {
                    func_frame.data[i].flag = GhFrameDataFlag::from_byte(self.last_flags[i] as u8);
                }
            }
        }

        if data_frame.pack_header.contains(PackHeader::AGC_INFO_EN) && data_frame.agc_info_size > 0 {
            self.last_agc_size = data_frame.agc_info_size;
            for i in 0..data_frame.agc_info_size {
                self.last_agc_info[i] = data_frame.agc_info[i];
                self.last_agc_info_high[i] = data_frame.agc_info_high[i];
                
                if i < func_frame.data.len() {
                    let word0 = self.last_agc_info[i] as u32;
                    let word1 = self.last_agc_info_high[i] as u32;
                    let mut agc_bytes = [0u8; 8];
                    agc_bytes[0..4].copy_from_slice(&word0.to_le_bytes());
                    agc_bytes[4..8].copy_from_slice(&word1.to_le_bytes());
                    let (agc_info, led_drv_fs) = crate::types::GhAgcInfo::from_bytes(&agc_bytes).unwrap_or_default();
                    func_frame.data[i].agc_info = agc_info;
                    if i == 0 {
                        func_frame.led_drv_fs[0] = led_drv_fs;
                    }
                }
            }
        } else if self.last_agc_size > 0 {
            for i in 0..self.last_agc_size {
                if i < func_frame.data.len() {
                    let word0 = self.last_agc_info[i] as u32;
                    let word1 = self.last_agc_info_high[i] as u32;
                    let mut agc_bytes = [0u8; 8];
                    agc_bytes[0..4].copy_from_slice(&word0.to_le_bytes());
                    agc_bytes[4..8].copy_from_slice(&word1.to_le_bytes());
                    let (agc_info, led_drv_fs) = crate::types::GhAgcInfo::from_bytes(&agc_bytes).unwrap_or_default();
                    func_frame.data[i].agc_info = agc_info;
                    if i == 0 {
                        func_frame.led_drv_fs[0] = led_drv_fs;
                    }
                }
            }
        }
        func_frame.ch_max = 32;
        func_frame.led_drv_fs[1] = func_frame.led_drv_fs[0];
        self.start_flag = false;
        func_frame
    }

    fn decode_frames_internal(&mut self, data: &[u8], logger: &dyn LogCallback) -> Result<Vec<GhFuncFrame>, DecodeError> {
        let mut frames = Vec::new();
        self.reset();

        logger.log(LogLevel::Debug, "decode", &format!("start decoding {} bytes", data.len()));

        while self.pos < data.len() {
            logger.log(LogLevel::Debug, "decode", &format!("--- frame {} at pos {} ---", frames.len() + 1, self.pos));
            
            match self.decode_single_frame(data, logger) {
                Ok(data_frame) => {
                    let func_frame = self.process_single_frame(&data_frame);
                    
                    frames.push(func_frame);
                }
                Err(DecodeError::InsufficientData) => {
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(frames)
    }
}

#[derive(Clone)]
pub struct FrameDecoder {
    state: Arc<Mutex<DecoderState>>,
    logger: Arc<dyn LogCallback>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(DecoderState::new())),
            logger: Arc::new(NullLogger),
        }
    }

    pub fn with_logger(mut self, logger: Arc<dyn LogCallback>) -> Self {
        self.logger = logger;
        self
    }

    pub fn decode_frames(&self, data: &[u8]) -> Result<Vec<GhFuncFrame>, DecodeError> {
        let mut state = self.state.lock().map_err(|_| DecodeError::InvalidFormat)?;
        state.decode_frames_internal(data, self.logger.as_ref())
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_decode() {
        let data = [0x5D];
        let mut pos = 0;
        let value = varint_decode(&data, &mut pos).unwrap();
        assert_eq!(value, 93);
        assert_eq!(pos, 1);

        let data = [0x80, 0x01];
        let mut pos = 0;
        let value = varint_decode(&data, &mut pos).unwrap();
        assert_eq!(value, 128);
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_empty_data() {
        let data: [u8; 0] = [];
        let decoder = FrameDecoder::new();
        let result = decoder.decode_frames(&data);
        assert!(result.is_ok());
        let frames = result.unwrap();
        assert!(frames.is_empty());
    }
}
