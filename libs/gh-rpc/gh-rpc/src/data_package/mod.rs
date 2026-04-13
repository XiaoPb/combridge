//! Frame data encoding and decoding module
//!
//! This module implements frame data encoding/decoding compatible with the C version.
//! It provides differential encoding for efficient data compression and supports
//! various data types including raw data, physical values, and AGC information.

#![allow(dead_code)]

use rpc::types::{FrameError, PackHeader};
use heapless::Vec;

/// Buffer size for encoding frame data
const BYTES_BUFFER_SIZE: usize = 2048;
/// Maximum number of channels supported
const MAX_CHANNELS: usize = 32;
/// Maximum GS data elements
const MAX_GS_DATA: usize = 6;
/// Maximum algorithm data elements
const MAX_ALGO_DATA: usize = 32;

/// Encodes a signed integer using zigzag encoding.
pub fn zigzag_encode(value: i32) -> u32 {
    ((value >> 31) ^ (value << 1)) as u32
}

/// Decodes a zigzag-encoded unsigned integer back to a signed integer.
pub fn zigzag_decode(value: u32) -> i32 {
    ((value >> 1) as i32) ^ (-((value & 1) as i32))
}

/// Encodes an unsigned integer using variable-length encoding.
pub fn varint_encode(value: u32, buffer: &mut [u8]) -> usize {
    let mut value = value;
    let mut pos = 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buffer[pos] = byte;
        pos += 1;
        if value == 0 {
            break;
        }
    }
    pos
}

/// Decodes a variable-length encoded unsigned integer.
pub fn varint_decode(buffer: &[u8]) -> Result<(u32, usize), FrameError> {
    let mut value: u32 = 0;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= buffer.len() {
            return Err(FrameError::InvalidLength);
        }

        let byte = buffer[pos];
        value |= ((byte & 0x7F) as u32) << shift;
        pos += 1;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift >= 35 {
            return Err(FrameError::InvalidFormat);
        }
    }

    Ok((value, pos))
}

/// AGC (Automatic Gain Control) information.
#[derive(Debug, Clone, Copy, Default)]
pub struct AgcInfo {
    /// 增益编码
    pub gain_code: u8,
    /// 背景消除范围
    pub bg_cancel_range: u8,
    /// 直流消除范围
    pub dc_cancel_range: u8,
    /// 直流消除编码
    pub dc_cancel_code: u8,
    /// LED 驱动满量程
    pub led_drv_fs: u8,
    /// LED 驱动 0
    pub led_drv0: u8,
    /// LED 驱动 1
    pub led_drv1: u8,
}

impl AgcInfo {
    /// 将低 32 位打包
    pub fn to_low_u32(&self) -> u32 {
        let mut value: u32 = 0;
        value |= (self.gain_code as u32) & 0x0F;
        value |= ((self.bg_cancel_range as u32) & 0x03) << 4;
        value |= ((self.dc_cancel_range as u32) & 0x03) << 6;
        value |= ((self.dc_cancel_code as u32) & 0xFF) << 8;
        value |= ((self.led_drv_fs as u32) & 0xFF) << 16;
        value |= ((self.led_drv0 as u32) & 0xFF) << 24;
        value
    }

    /// 将高 32 位打包
    pub fn to_high_u32(&self) -> u32 {
        (self.led_drv1 as u32) & 0xFF
    }

    /// 从低 32 位和高 32 位解包
    pub fn from_low_high(low: u32, high: u32) -> Self {
        Self {
            gain_code: (low & 0x0F) as u8,
            bg_cancel_range: ((low >> 4) & 0x03) as u8,
            dc_cancel_range: ((low >> 6) & 0x03) as u8,
            dc_cancel_code: ((low >> 8) & 0xFF) as u8,
            led_drv_fs: ((low >> 16) & 0xFF) as u8,
            led_drv0: ((low >> 24) & 0xFF) as u8,
            led_drv1: (high & 0xFF) as u8,
        }
    }
}

/// Frame data flag
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameDataFlag {
    /// LED 调整标志
    pub led_adj_flag: bool,
    /// SA 标志
    pub sa_flag: bool,
    /// 参数变更标志
    pub param_change_flag: bool,
    /// DRE 更新标志
    pub dre_update: bool,
    /// 跳过 OK 标志
    pub skip_ok_flag: bool,
}

impl FrameDataFlag {
    /// 从 u32 值解析标志位
    pub fn from_u32(value: u32) -> Self {
        Self {
            led_adj_flag: (value & 0x01) != 0,
            sa_flag: ((value >> 1) & 0x01) != 0,
            param_change_flag: ((value >> 2) & 0x01) != 0,
            dre_update: ((value >> 3) & 0x01) != 0,
            skip_ok_flag: ((value >> 4) & 0x01) != 0,
        }
    }
}

/// Channel data for a single frame
#[derive(Debug, Clone, Default)]
pub struct ChannelData {
    /// IPD/PA 值
    pub ipd_pa: i32,
    /// 原始数据
    pub rawdata: i32,
    /// 帧数据标志
    pub flag: FrameDataFlag,
    /// AGC 信息
    pub agc_info: AgcInfo,
}

/// GSensor data (accelerometer and optional gyroscope)
#[derive(Debug, Clone, Copy, Default)]
pub struct GSensorData {
    /// 加速度计数据 [x, y, z]
    pub acc: [i16; 3],
    /// 陀螺仪数据 [x, y, z]
    pub gyro: [i16; 3],
}

/// Function ID enumeration
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum FuncId {
    /// ADT 模式
    #[default]
    ADT = 0,
    /// 心率模式
    HR = 1,
    /// 血氧模式
    SPO2 = 2,
    /// 心率变异性模式
    HRV = 3,
    /// GNADT 模式
    GNADT = 4,
    /// IRNADT 模式
    IRNADT = 5,
    /// 未知模式
    Unknown = 255,
}

impl From<u32> for FuncId {
    fn from(value: u32) -> Self {
        match value {
            0 => FuncId::ADT,
            1 => FuncId::HR,
            2 => FuncId::SPO2,
            3 => FuncId::HRV,
            4 => FuncId::GNADT,
            5 => FuncId::IRNADT,
            _ => FuncId::Unknown,
        }
    }
}

/// Decoded function frame (gh_func_frame_t equivalent)
#[derive(Debug, Clone, Default)]
pub struct FuncFrame {
    /// 帧计数
    pub frame_cnt: u32,
    /// 时间戳
    pub timestamp: u64,
    /// 功能 ID
    pub id: FuncId,
    /// 通道数
    pub ch_num: u8,
    /// 通道数据列表
    pub p_data: Vec<ChannelData, MAX_CHANNELS>,
    /// GSensor 数据
    pub gsensor_data: GSensorData,
    /// LED 驱动满量程
    pub led_drv_fs: [u8; 2],
    /// 算法结果数据
    pub p_algo_res: Vec<i32, MAX_ALGO_DATA>,
}

/// Data frame containing raw decoded data (data_frame_t equivalent)
#[derive(Debug, Clone, Default)]
pub struct DataFrame {
    /// 包头标志
    pub pack_header: PackHeader,
    /// 原始数据
    pub rawdata: Vec<i32, MAX_CHANNELS>,
    /// 原始数据大小
    pub rawdata_size: usize,
    /// 物理值
    pub phy_value: Vec<i32, MAX_CHANNELS>,
    /// 物理值大小
    pub phy_value_size: usize,
    /// GS 数据
    pub gs_data: Vec<i32, MAX_GS_DATA>,
    /// GS 数据大小
    pub gs_data_size: usize,
    /// 标志位数据
    pub flags: Vec<u32, MAX_CHANNELS>,
    /// 标志位数据位数
    pub flag_data_bits: usize,
    /// 算法数据
    pub algo_data: Vec<i32, MAX_ALGO_DATA>,
    /// 算法数据位数
    pub algo_data_bits: usize,
    /// AGC 信息（低 32 位）
    pub agc_info: Vec<i32, MAX_CHANNELS>,
    /// AGC 信息（高 32 位）
    pub agc_info_high: Vec<i32, MAX_CHANNELS>,
    /// AGC 信息大小
    pub agc_info_size: usize,
    /// 时间戳（低 32 位）
    pub timestamp: i32,
    /// 时间戳（高 32 位）
    pub timestamp_high: i32,
    /// 帧 ID
    pub frame_id: i32,
    /// 功能 ID
    pub function_id: i32,
    /// 时隙配置
    pub slot_cfg: i32,
}

/// Frame decoder for deserializing data frames.
///
/// Compatible with C version's gh_protocol_process function.
/// Supports differential decoding where:
/// - First frame: values are used directly
/// - Subsequent frames: values are differences to be added to last values
pub struct FrameDecoder {
    pos: usize,
    start_flag: bool,
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

impl FrameDecoder {
    /// 创建新的帧解码器
    pub fn new() -> Self {
        Self {
            pos: 0,
            start_flag: true,
            last_rawdata: [0; MAX_CHANNELS],
            last_phy_value: [0; MAX_CHANNELS],
            last_timestamp: 0,
            last_timestamp_high: 0,
            last_gs_data: [0; MAX_GS_DATA],
            last_gs_data_size: 0,
            last_flags: [0; MAX_CHANNELS],
            last_flag_data_bits: 0,
            last_algo_data: [0; MAX_ALGO_DATA],
            last_agc_info: [0; MAX_CHANNELS],
            last_agc_info_high: [0; MAX_CHANNELS],
            last_agc_size: 0,
        }
    }

    fn read_varint(&mut self, buffer: &[u8]) -> Result<u32, FrameError> {
        let (value, bytes_read) = varint_decode(&buffer[self.pos..])?;
        self.pos += bytes_read;
        Ok(value)
    }

    fn read_varints(&mut self, buffer: &[u8], count: usize, output: &mut [i32]) -> Result<(), FrameError> {
        for i in 0..count {
            if i >= output.len() {
                break;
            }
            let zigzag_val = self.read_varint(buffer)?;
            output[i] = zigzag_decode(zigzag_val);
        }
        Ok(())
    }

    fn decode_single_frame(&mut self, buffer: &[u8], _len: usize) -> Result<DataFrame, FrameError> {
        let mut frame = DataFrame::default();

        let zigzag_val = self.read_varint(buffer)?;
        let header_i32 = zigzag_decode(zigzag_val);
        let header_value = header_i32 as u32;
        frame.pack_header = PackHeader::from_bits_truncate(header_value);

        log::debug!("[FrameDecoder] PackHeader: pos={}, zigzag={}, i32={}, u32={}, bits={:032b}", 
            self.pos - 1, zigzag_val, header_i32, header_value, header_value);

        if frame.pack_header.contains(PackHeader::RAWDATA_EN) {
            let size_zigzag = self.read_varint(buffer)?;
            frame.rawdata_size = zigzag_decode(size_zigzag) as usize;
            if frame.rawdata_size > MAX_CHANNELS {
                frame.rawdata_size = MAX_CHANNELS;
            }
            let mut temp = [0i32; MAX_CHANNELS];
            self.read_varints(buffer, frame.rawdata_size, &mut temp)?;
            for i in 0..frame.rawdata_size {
                let _ = frame.rawdata.push(temp[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::PHY_VALUE_EN) {
            let size_zigzag = self.read_varint(buffer)?;
            frame.phy_value_size = zigzag_decode(size_zigzag) as usize;
            if frame.phy_value_size > MAX_CHANNELS {
                frame.phy_value_size = MAX_CHANNELS;
            }
            let mut temp = [0i32; MAX_CHANNELS];
            self.read_varints(buffer, frame.phy_value_size, &mut temp)?;
            for i in 0..frame.phy_value_size {
                let _ = frame.phy_value.push(temp[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::GS_DATA_EN) {
            let size_zigzag = self.read_varint(buffer)?;
            frame.gs_data_size = zigzag_decode(size_zigzag) as usize;
            if frame.gs_data_size > MAX_GS_DATA {
                frame.gs_data_size = MAX_GS_DATA;
            }
            let mut temp = [0i32; MAX_GS_DATA];
            self.read_varints(buffer, frame.gs_data_size, &mut temp)?;
            for i in 0..frame.gs_data_size {
                let _ = frame.gs_data.push(temp[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::FLAGS_EN) {
            let bits_zigzag = self.read_varint(buffer)?;
            frame.flag_data_bits = zigzag_decode(bits_zigzag) as usize;
            if frame.flag_data_bits > MAX_CHANNELS {
                frame.flag_data_bits = MAX_CHANNELS;
            }
            let mut temp = [0i32; MAX_CHANNELS];
            self.read_varints(buffer, frame.flag_data_bits, &mut temp)?;
            for i in 0..frame.flag_data_bits {
                let _ = frame.flags.push(temp[i] as u32);
            }
        }

        if frame.pack_header.contains(PackHeader::ALG_DATA_EN) {
            let bits_zigzag = self.read_varint(buffer)?;
            frame.algo_data_bits = zigzag_decode(bits_zigzag) as usize;
            if frame.algo_data_bits > MAX_ALGO_DATA {
                frame.algo_data_bits = MAX_ALGO_DATA;
            }
            let mut temp = [0i32; MAX_ALGO_DATA];
            self.read_varints(buffer, frame.algo_data_bits, &mut temp)?;
            for i in 0..frame.algo_data_bits {
                let _ = frame.algo_data.push(temp[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::AGC_INFO_EN) {
            let size_zigzag = self.read_varint(buffer)?;
            frame.agc_info_size = zigzag_decode(size_zigzag) as usize;
            if frame.agc_info_size > MAX_CHANNELS {
                frame.agc_info_size = MAX_CHANNELS;
            }
            let mut temp_low = [0i32; MAX_CHANNELS];
            let mut temp_high = [0i32; MAX_CHANNELS];
            self.read_varints(buffer, frame.agc_info_size, &mut temp_low)?;
            self.read_varints(buffer, frame.agc_info_size, &mut temp_high)?;
            for i in 0..frame.agc_info_size {
                let _ = frame.agc_info.push(temp_low[i]);
                let _ = frame.agc_info_high.push(temp_high[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::TIMESTAMP_EN) {
            let mut temp = [0i32; 2];
            self.read_varints(buffer, 2, &mut temp)?;
            frame.timestamp = temp[0];
            frame.timestamp_high = temp[1];
        }

        let mut temp = [0i32; 1];
        self.read_varints(buffer, 1, &mut temp)?;
        frame.frame_id = temp[0];

        if frame.pack_header.contains(PackHeader::FUNC_ID_EN) {
            let mut temp = [0i32; 1];
            self.read_varints(buffer, 1, &mut temp)?;
            frame.function_id = temp[0];
        }

        if frame.pack_header.contains(PackHeader::SLOT_CFG_EN) {
            let mut temp = [0i32; 1];
            self.read_varints(buffer, 1, &mut temp)?;
            frame.slot_cfg = temp[0];
        }

        Ok(frame)
    }

    fn process_single_frame(&mut self, data_frame: &DataFrame) -> FuncFrame {
        let mut func_frame = FuncFrame::default();

        func_frame.frame_cnt = data_frame.frame_id as u32;
        func_frame.id = FuncId::from(data_frame.function_id as u32);

        if data_frame.pack_header.contains(PackHeader::TIMESTAMP_EN) {
            if self.start_flag {
                func_frame.timestamp = (data_frame.timestamp as u64) 
                    | ((data_frame.timestamp_high as u64) << 32);
                self.last_timestamp = data_frame.timestamp;
                self.last_timestamp_high = data_frame.timestamp_high;
            } else {
                let last_ts = (self.last_timestamp as u64) 
                    | ((self.last_timestamp_high as u64) << 32);
                let diff_ts = (data_frame.timestamp as u64) 
                    | ((data_frame.timestamp_high as u64) << 32);
                func_frame.timestamp = last_ts + diff_ts;
                self.last_timestamp = (func_frame.timestamp & 0xFFFFFFFF) as i32;
                self.last_timestamp_high = ((func_frame.timestamp >> 32) & 0xFFFFFFFF) as i32;
            }
        }

        if data_frame.pack_header.contains(PackHeader::RAWDATA_EN) && data_frame.rawdata_size > 0 {
            func_frame.ch_num = data_frame.rawdata_size as u8;
            for i in 0..data_frame.rawdata_size {
                let mut ch_data = ChannelData::default();
                if self.start_flag {
                    ch_data.rawdata = data_frame.rawdata[i];
                    self.last_rawdata[i] = data_frame.rawdata[i];
                } else {
                    ch_data.rawdata = self.last_rawdata[i] + data_frame.rawdata[i];
                    self.last_rawdata[i] = ch_data.rawdata;
                }
                let _ = func_frame.p_data.push(ch_data);
            }
        }

        if data_frame.pack_header.contains(PackHeader::PHY_VALUE_EN) && data_frame.phy_value_size > 0 {
            if func_frame.ch_num == 0 {
                func_frame.ch_num = data_frame.phy_value_size as u8;
            }
            for i in 0..data_frame.phy_value_size {
                if i >= func_frame.p_data.len() {
                    let mut ch_data = ChannelData::default();
                    if self.start_flag {
                        ch_data.ipd_pa = data_frame.phy_value[i];
                        self.last_phy_value[i] = data_frame.phy_value[i];
                    } else {
                        ch_data.ipd_pa = self.last_phy_value[i] + data_frame.phy_value[i];
                        self.last_phy_value[i] = ch_data.ipd_pa;
                    }
                    let _ = func_frame.p_data.push(ch_data);
                } else {
                    if self.start_flag {
                        func_frame.p_data[i].ipd_pa = data_frame.phy_value[i];
                        self.last_phy_value[i] = data_frame.phy_value[i];
                    } else {
                        func_frame.p_data[i].ipd_pa = self.last_phy_value[i] + data_frame.phy_value[i];
                        self.last_phy_value[i] = func_frame.p_data[i].ipd_pa;
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
                } else if i < 6 {
                    if self.start_flag {
                        func_frame.gsensor_data.gyro[i - 3] = data_frame.gs_data[i] as i16;
                        self.last_gs_data[i] = data_frame.gs_data[i];
                    } else {
                        let val = self.last_gs_data[i] + data_frame.gs_data[i];
                        func_frame.gsensor_data.gyro[i - 3] = val as i16;
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
                if i < func_frame.p_data.len() {
                    func_frame.p_data[i].flag = FrameDataFlag::from_u32(self.last_flags[i] as u32);
                }
            }
        } else if self.last_flag_data_bits > 0 {
            for i in 0..self.last_flag_data_bits {
                if i < func_frame.p_data.len() {
                    func_frame.p_data[i].flag = FrameDataFlag::from_u32(self.last_flags[i] as u32);
                }
            }
        }

        if data_frame.pack_header.contains(PackHeader::ALG_DATA_EN) {
            self.last_algo_data[0] = data_frame.algo_data_bits as i32;
            for i in 0..data_frame.algo_data_bits {
                if self.start_flag {
                    self.last_algo_data[i + 1] = data_frame.algo_data[i];
                } else {
                    self.last_algo_data[i + 1] = self.last_algo_data[i + 1] + data_frame.algo_data[i];
                }
            }
        }
        
        for i in 0..=(self.last_algo_data[0] as usize) {
            if i < MAX_ALGO_DATA {
                let _ = func_frame.p_algo_res.push(self.last_algo_data[i]);
            }
        }

        if data_frame.pack_header.contains(PackHeader::AGC_INFO_EN) {
            for i in 0..data_frame.agc_info_size {
                if self.start_flag {
                    self.last_agc_info[i] = data_frame.agc_info[i];
                    self.last_agc_info_high[i] = data_frame.agc_info_high[i];
                } else {
                    self.last_agc_info[i] = self.last_agc_info[i] + data_frame.agc_info[i];
                    self.last_agc_info_high[i] = self.last_agc_info_high[i] + data_frame.agc_info_high[i];
                }
            }
            self.last_agc_size = data_frame.agc_info_size;
        }

        for i in 0..self.last_agc_size {
            if i < func_frame.p_data.len() {
                func_frame.p_data[i].agc_info = AgcInfo::from_low_high(
                    self.last_agc_info[i] as u32,
                    self.last_agc_info_high[i] as u32
                );
                func_frame.led_drv_fs[0] = func_frame.p_data[i].agc_info.led_drv_fs;
            }
        }

        self.start_flag = false;

        func_frame
    }

    /// Decode multiple frames from buffer (gh_protocol_process equivalent)
    ///
    /// # Arguments
    ///
    /// * `buffer` - Input byte buffer containing encoded frames
    /// * `frames` - Output vector to store decoded frames
    ///
    /// # Returns
    ///
    /// Number of frames decoded
    pub fn decode_frames(&mut self, buffer: &[u8], frames: &mut Vec<FuncFrame, 16>) -> usize {
        self.reset();
        frames.clear();

        while self.pos < buffer.len() {
            let start_pos = self.pos;
            
            match self.decode_single_frame(buffer, buffer.len()) {
                Ok(data_frame) => {
                    let func_frame = self.process_single_frame(&data_frame);
                    
                    if self.pos <= start_pos {
                        break;
                    }
                    
                    if frames.push(func_frame).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }

        frames.len()
    }

    /// Decode a single frame from buffer
    pub fn decode(&mut self, buffer: &[u8]) -> Result<FuncFrame, FrameError> {
        self.pos = 0;
        self.reset();
        
        let data_frame = self.decode_single_frame(buffer, buffer.len())?;
        Ok(self.process_single_frame(&data_frame))
    }

    /// Reset decoder state (clears all history data)
    pub fn reset(&mut self) {
        self.pos = 0;
        self.start_flag = true;
        self.last_rawdata.fill(0);
        self.last_phy_value.fill(0);
        self.last_timestamp = 0;
        self.last_timestamp_high = 0;
        self.last_gs_data.fill(0);
        self.last_gs_data_size = 0;
        self.last_flags.fill(0);
        self.last_flag_data_bits = 0;
        self.last_algo_data.fill(0);
        self.last_agc_info.fill(0);
        self.last_agc_info_high.fill(0);
        self.last_agc_size = 0;
    }

    /// Returns current position in buffer
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Returns true if there is more data to decode
    pub fn has_more(&self, buffer_len: usize) -> bool {
        self.pos < buffer_len
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame encoder for serializing data frames.
pub struct FrameEncoder {
    buffer: [u8; BYTES_BUFFER_SIZE],
    pos: usize,
}

impl FrameEncoder {
    /// 创建新的帧编码器
    pub fn new() -> Self {
        Self {
            buffer: [0u8; BYTES_BUFFER_SIZE],
            pos: 0,
        }
    }

    fn write_varint(&mut self, value: u32) {
        let encoded = varint_encode(value, &mut self.buffer[self.pos..]);
        self.pos += encoded;
    }

    fn write_zigzag(&mut self, value: i32) {
        self.write_varint(zigzag_encode(value));
    }

    fn write_zigzag_array(&mut self, values: &[i32]) {
        for &val in values {
            self.write_zigzag(val);
        }
    }

    /// Encode a data frame into bytes
    pub fn encode(&mut self, frame: &DataFrame) -> Result<&[u8], FrameError> {
        self.pos = 0;

        self.write_zigzag(frame.pack_header.bits() as i32);

        if frame.pack_header.contains(PackHeader::RAWDATA_EN) && frame.rawdata_size > 0 {
            self.write_zigzag(frame.rawdata_size as i32);
            for i in 0..frame.rawdata_size {
                self.write_zigzag(frame.rawdata[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::PHY_VALUE_EN) && frame.phy_value_size > 0 {
            self.write_zigzag(frame.phy_value_size as i32);
            for i in 0..frame.phy_value_size {
                self.write_zigzag(frame.phy_value[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::GS_DATA_EN) && frame.gs_data_size > 0 {
            self.write_zigzag(frame.gs_data_size as i32);
            for i in 0..frame.gs_data_size {
                self.write_zigzag(frame.gs_data[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::FLAGS_EN) && frame.flag_data_bits > 0 {
            self.write_zigzag(frame.flag_data_bits as i32);
            for i in 0..frame.flag_data_bits {
                self.write_zigzag(frame.flags[i] as i32);
            }
        }

        if frame.pack_header.contains(PackHeader::ALG_DATA_EN) && frame.algo_data_bits > 0 {
            self.write_zigzag(frame.algo_data_bits as i32);
            for i in 0..frame.algo_data_bits {
                self.write_zigzag(frame.algo_data[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::AGC_INFO_EN) && frame.agc_info_size > 0 {
            self.write_zigzag(frame.agc_info_size as i32);
            for i in 0..frame.agc_info_size {
                self.write_zigzag(frame.agc_info[i]);
            }
            for i in 0..frame.agc_info_size {
                self.write_zigzag(frame.agc_info_high[i]);
            }
        }

        if frame.pack_header.contains(PackHeader::TIMESTAMP_EN) {
            self.write_zigzag(frame.timestamp);
            self.write_zigzag(frame.timestamp_high);
        }

        self.write_zigzag(frame.frame_id);

        if frame.pack_header.contains(PackHeader::FUNC_ID_EN) {
            self.write_zigzag(frame.function_id);
        }

        if frame.pack_header.contains(PackHeader::SLOT_CFG_EN) {
            self.write_zigzag(frame.slot_cfg);
        }

        Ok(&self.buffer[..self.pos])
    }

    /// Reset encoder state
    pub fn reset(&mut self) {
        self.pos = 0;
        self.buffer.fill(0);
    }
}

impl Default for FrameEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_encode() {
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(-2), 3);
        assert_eq!(zigzag_encode(2), 4);
    }

    #[test]
    fn test_zigzag_decode() {
        assert_eq!(zigzag_decode(0), 0);
        assert_eq!(zigzag_decode(1), -1);
        assert_eq!(zigzag_decode(2), 1);
        assert_eq!(zigzag_decode(3), -2);
        assert_eq!(zigzag_decode(4), 2);
    }

    #[test]
    fn test_zigzag_roundtrip() {
        for i in -100..=100 {
            let encoded = zigzag_encode(i);
            let decoded = zigzag_decode(encoded);
            assert_eq!(decoded, i);
        }
    }

    #[test]
    fn test_varint_encode() {
        let mut buffer = [0u8; 10];

        let len = varint_encode(0, &mut buffer);
        assert_eq!(len, 1);
        assert_eq!(buffer[0], 0);

        let len = varint_encode(127, &mut buffer);
        assert_eq!(len, 1);
        assert_eq!(buffer[0], 127);

        let len = varint_encode(128, &mut buffer);
        assert_eq!(len, 2);
        assert_eq!(buffer[0], 0x80);
        assert_eq!(buffer[1], 1);

        let len = varint_encode(300, &mut buffer);
        assert_eq!(len, 2);
        assert_eq!(buffer[0], 0xAC);
        assert_eq!(buffer[1], 0x02);
    }

    #[test]
    fn test_varint_decode() {
        let buffer = [0u8; 10];

        let (value, len) = varint_decode(&buffer).unwrap();
        assert_eq!(value, 0);
        assert_eq!(len, 1);

        let buffer = [127u8, 0];
        let (value, len) = varint_decode(&buffer).unwrap();
        assert_eq!(value, 127);
        assert_eq!(len, 1);

        let buffer = [0x80u8, 1];
        let (value, len) = varint_decode(&buffer).unwrap();
        assert_eq!(value, 128);
        assert_eq!(len, 2);

        let buffer = [0xACu8, 0x02];
        let (value, len) = varint_decode(&buffer).unwrap();
        assert_eq!(value, 300);
        assert_eq!(len, 2);
    }

    #[test]
    fn test_frame_decoder_basic() {
        let mut decoder = FrameDecoder::new();
        let mut frames = Vec::new();
        
        let test_data: [u8; 211] = [
            0xAA, 0x11, 0xCF, 0x9A, 0x47, 0x5D, 0xCB, 0xD6, 0x05, 0x04, 0x00, 0xC8, 0x01, 0x04, 0x00, 0xD0,
            0x0F, 0x04, 0x00, 0x00, 0x04, 0x80, 0x80, 0xC0, 0x06, 0x80, 0x80, 0xC0, 0x06, 0x00, 0x00, 0xF0,
            0x96, 0xBF, 0xFA, 0x0F, 0xBA, 0x06, 0xE0, 0x01, 0x02, 0xC6, 0x05, 0x04, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x04, 0x80, 0x80, 0x98, 0x07, 0x80, 0x80, 0x98, 0x07, 0x00, 0x00, 0x00, 0x00, 0xE2, 0x01,
            0x02, 0xC6, 0x05, 0x04, 0x00, 0x00, 0x04, 0x00, 0x00, 0x04, 0x80, 0x80, 0xC8, 0x06, 0x80, 0x80,
            0xC8, 0x06, 0x00, 0x00, 0x00, 0x00, 0xE4, 0x01, 0x02, 0xC6, 0x05, 0x04, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x04, 0x80, 0x80, 0xF0, 0x07, 0x80, 0x80, 0xF0, 0x07, 0x00, 0x00, 0x00, 0x00, 0xE6, 0x01,
            0x02, 0xC6, 0x05, 0x04, 0x00, 0x00, 0x04, 0x00, 0x00, 0x04, 0x80, 0x80, 0xA8, 0x0E, 0x80, 0x80,
            0xA8, 0x0E, 0x00, 0x00, 0x00, 0x00, 0xE8, 0x01, 0x02, 0xC6, 0x05, 0x04, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x04, 0x80, 0x80, 0xF8, 0x0A, 0x80, 0x80, 0xF8, 0x0A, 0x00, 0x00, 0xD0, 0x0F, 0x00, 0xEA,
            0x01, 0x02, 0xC6, 0x05, 0x04, 0x00, 0x00, 0x04, 0x00, 0x00, 0x04, 0x80, 0x80, 0xB0, 0x01, 0x80,
            0x80, 0xB0, 0x01, 0x00, 0x00, 0x00, 0x00, 0xEC, 0x01, 0x02, 0xC6, 0x05, 0x04, 0x00, 0x00, 0x04,
            0x00, 0x00, 0x04, 0x80, 0x80, 0x80, 0x0B, 0x80, 0x80, 0x80, 0x0B, 0x00, 0x00, 0x00, 0x00, 0xEE,
            0x01, 0x02, 0xF6
        ];

        let actual_data = &test_data[7..210];
        
        let count = decoder.decode_frames(actual_data, &mut frames);
        
        println!("Decoded {} frames", count);
        for (i, frame) in frames.iter().enumerate() {
            println!("Frame {}: frame_cnt={}, ch_num={}, timestamp={}", 
                i, frame.frame_cnt, frame.ch_num, frame.timestamp);
        }
        
        assert!(count > 0);
    }
}
