//! GH3036 协议数据类型定义
//!
//! 本模块定义 GH3036 协议相关的数据类型

use serde::{Deserialize, Serialize};

pub use gh_rpc::{
    GhFuncFrame, GhFrameData, GhFuncFixIdx, GhGsensorData, GhAgcInfo, GhFrameDataFlag,
    FrameDecoder, DecodeError,
    KEY_GH3X_GET_VERSION, KEY_GH3X_REGS_WRITE_CMD, KEY_GH3X_REGS_READ_CMD,
    KEY_GH3X_REG_BIT_FIELD_WRITE_CMD, KEY_GH3X_CHIP_CTRL, KEY_GH3X_SW_FUNCTION_CMD,
    KEY_DOWNLOAD_CONFIG, KEY_GH3X_REGS_LIST_WRITE_CMD, KEY_GET_CHIP_LINK_STATUS,
    KEY_GH_TIMESTAMP_SET, KEY_GH_TIME_SET, KEY_GH_SET_WORK_MODE_CMD, KEY_GH_LOW_POWER_CMD,
    KEY_F_SET_MODE, KEY_F_GET_MODE,
    FMT_GH3X_GET_VERSION, FMT_GH3X_REGS_WRITE_CMD, FMT_GH3X_REGS_READ_CMD,
    FMT_GH3X_REG_BIT_FIELD_WRITE_CMD, FMT_GH3X_CHIP_CTRL, FMT_GH3X_SW_FUNCTION_CMD,
    FMT_DOWNLOAD_CONFIG, FMT_GH3X_REGS_LIST_WRITE_CMD, FMT_GET_CHIP_LINK_STATUS,
    FMT_GH_TIMESTAMP_SET, FMT_GH_TIME_SET, FMT_GH_SET_WORK_MODE_CMD, FMT_GH_LOW_POWER_CMD,
    FMT_F_SET_MODE, FMT_F_GET_MODE,
    RET_GH3X_GET_VERSION, RET_GH3X_REGS_READ_CMD, RET_GET_CHIP_LINK_STATUS, RET_F_GET_MODE,
};

pub use rpc::types::*;

pub const VER_TYPE_FW: u8 = 0;
pub const VER_TYPE_DEMO: u8 = 1;
pub const VER_TYPE_BOOTLOADER: u8 = 2;
pub const VER_TYPE_PROTOCOL: u8 = 3;
pub const VER_TYPE_VIRTUAL_REG: u8 = 4;
pub const VER_TYPE_DRV: u8 = 5;
pub const VER_TYPE_CHIP: u8 = 6;
pub const VER_TYPE_BLE: u8 = 7;
pub const VER_TYPE_ALGO: u8 = 8;
pub const VER_TYPE_FUNC_SUPPORT: u8 = 9;

pub const HR_VERSION_OFFSET: u8 = 0;
pub const HRV_VERSION_OFFSET: u8 = 1;
pub const SPO2_VERSION_OFFSET: u8 = 2;
pub const ADT_VERSION_OFFSET: u8 = 3;
pub const NADT_VERSION_OFFSET: u8 = 4;

pub const CHIP_CTRL_HARD_RESET: u8 = 0xC0;
pub const CHIP_CTRL_RX_RESET: u8 = 0xC1;
pub const CHIP_CTRL_SOFT_RESET: u8 = 0xC2;
pub const CHIP_CTRL_WAKEUP: u8 = 0xC3;
pub const CHIP_CTRL_SLEEP: u8 = 0xC4;

pub const GH_ACC_AXIS_NUM: usize = 3;
pub const GH_GYRO_AXIS_NUM: usize = 3;

pub const FACTORY_TEST_MODE_CHIP_INIT_OFFSET: u8 = 0;
pub const FACTORY_TEST_MODE_CHIP_UID_OFFSET: u8 = 1;
pub const FACTORY_TEST_MODE_BASE_NOISE_OFFSET: u8 = 2;
pub const FACTORY_TEST_MODE_PPG_NOISE_OFFSET: u8 = 3;
pub const FACTORY_TEST_MODE_LPCTR_OFFSET: u8 = 4;
pub const FACTORY_TEST_MODE_LPLCTR_OFFSET: u8 = 5;

pub const FACTORY_TEST_MODE_NONE: u8 = 0;
pub const FACTORY_TEST_MODE_CHIP_INIT: u8 = 1 << FACTORY_TEST_MODE_CHIP_INIT_OFFSET;
pub const FACTORY_TEST_MODE_CHIP_UID: u8 = 1 << FACTORY_TEST_MODE_CHIP_UID_OFFSET;
pub const FACTORY_TEST_MODE_BASE_NOISE: u8 = 1 << FACTORY_TEST_MODE_BASE_NOISE_OFFSET;
pub const FACTORY_TEST_MODE_PPG_NOISE: u8 = 1 << FACTORY_TEST_MODE_PPG_NOISE_OFFSET;
pub const FACTORY_TEST_MODE_LPCTR: u8 = 1 << FACTORY_TEST_MODE_LPCTR_OFFSET;
pub const FACTORY_TEST_MODE_LPLCTR: u8 = 1 << FACTORY_TEST_MODE_LPLCTR_OFFSET;

pub trait GhFuncFixIdxExt {
    fn from_i32(value: i32) -> Option<GhFuncFixIdx>;
    fn name(&self) -> &'static str;
}

impl GhFuncFixIdxExt for GhFuncFixIdx {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Adt),
            1 => Some(Self::Hr),
            2 => Some(Self::Spo2),
            3 => Some(Self::Hrv),
            4 => Some(Self::Gnadt),
            5 => Some(Self::Irnadt),
            6 => Some(Self::AlgoMax),
            7 => Some(Self::Test2),
            8 => Some(Self::PpgCfg0),
            9 => Some(Self::PpgCfg1),
            10 => Some(Self::PpgCfg2),
            11 => Some(Self::PpgCfg3),
            12 => Some(Self::PpgCfg4),
            13 => Some(Self::PpgCfg5),
            14 => Some(Self::PpgCfg6),
            15 => Some(Self::PpgCfg7),
            16 => Some(Self::CapCfg),
            _ => None,
        }
    }
    
    fn name(&self) -> &'static str {
        match self {
            Self::Adt => "ADT",
            Self::Hr => "HR",
            Self::Spo2 => "SPO2",
            Self::Hrv => "HRV",
            Self::Gnadt => "GNADT",
            Self::Irnadt => "IRNADT",
            Self::AlgoMax => "TEST1",
            Self::Test2 => "TEST2",
            Self::PpgCfg0 => "PPG_CFG0",
            Self::PpgCfg1 => "PPG_CFG1",
            Self::PpgCfg2 => "PPG_CFG2",
            Self::PpgCfg3 => "PPG_CFG3",
            Self::PpgCfg4 => "PPG_CFG4",
            Self::PpgCfg5 => "PPG_CFG5",
            Self::PpgCfg6 => "PPG_CFG6",
            Self::PpgCfg7 => "PPG_CFG7",
            Self::CapCfg => "CAP_CFG",
            Self::Max => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gh3036FrameData {
    pub function_id: i32,
    pub function_name: String,
    pub frame_id: i32,
    pub timestamp: u64,
    pub gs_data: Vec<i32>,
    pub rawdata: Vec<i32>,
    pub flags: Vec<i32>,
    pub algo_data: Vec<i32>,
    pub agc_info: Vec<i32>,
    pub phy_value: Vec<i32>,
}

impl Gh3036FrameData {
    pub fn from_func_frame(frame: &GhFuncFrame) -> Self {
        let func_id = GhFuncFixIdx::from(frame.id as u8);
        let function_name = func_id.name().to_string();
        
        let gs_data: Vec<i32> = [
            frame.gsensor_data.acc[0] as i32,
            frame.gsensor_data.acc[1] as i32,
            frame.gsensor_data.acc[2] as i32,
        ].to_vec();
        
        let rawdata: Vec<i32> = frame.data.iter().map(|d| d.rawdata).collect();
        let phy_value: Vec<i32> = frame.data.iter().map(|d| d.ipd_pa).collect();
        
        let agc_info: Vec<i32> = frame.data.iter()
            .map(|d| {
                let word0 = (d.agc_info.gain_code as u32)
                    | ((d.agc_info.bg_cancel_range as u32) << 4)
                    | ((d.agc_info.dc_cancel_range as u32) << 6)
                    | ((d.agc_info.dc_cancel_code as u32) << 8)
                    | ((d.agc_info.led_drv0 as u32) << 16)
                    | ((d.agc_info.led_drv1 as u32) << 24);
                word0 as i32
            })
            .collect();
        
        let flags: Vec<i32> = frame.data.iter()
            .map(|d| {
                let mut flag_val = 0i32;
                if d.flag.led_adj_flag { flag_val |= 1; }
                if d.flag.sa_flag { flag_val |= 2; }
                if d.flag.param_change_flag { flag_val |= 4; }
                if d.flag.dre_update { flag_val |= 8; }
                if d.flag.skip_ok_flag { flag_val |= 16; }
                flag_val
            })
            .collect();
        
        Self {
            function_id: frame.id as i32,
            function_name,
            frame_id: frame.frame_cnt as i32,
            timestamp: frame.timestamp,
            gs_data,
            rawdata,
            flags,
            algo_data: Vec::new(),
            agc_info,
            phy_value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCommand {
    pub key: String,
    pub name: String,
    pub description: String,
    pub params: Vec<RpcParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub default_value: Option<String>,
}

pub fn get_rpc_commands() -> Vec<RpcCommand> {
    vec![
        RpcCommand {
            key: "V".to_string(),
            name: KEY_GH3X_GET_VERSION.to_string(),
            description: "获取芯片版本信息".to_string(),
            params: vec![
                RpcParam {
                    name: "verType".to_string(),
                    param_type: "u8".to_string(),
                    description: "版本类型（参考版本类型配置）".to_string(),
                    default_value: Some("1".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "W".to_string(),
            name: KEY_GH3X_REGS_WRITE_CMD.to_string(),
            description: "寄存器写入命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regs".to_string(),
                    param_type: "u16[]".to_string(),
                    description: "寄存器数据数组（地址和值交替）".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "R".to_string(),
            name: KEY_GH3X_REGS_READ_CMD.to_string(),
            description: "寄存器读取命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regAddr".to_string(),
                    param_type: "u16".to_string(),
                    description: "寄存器起始地址".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "readLen".to_string(),
                    param_type: "i32".to_string(),
                    description: "读取长度（16 位字数）".to_string(),
                    default_value: Some("1".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "B".to_string(),
            name: KEY_GH3X_REG_BIT_FIELD_WRITE_CMD.to_string(),
            description: "寄存器位域写入命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regAddr".to_string(),
                    param_type: "u16".to_string(),
                    description: "寄存器地址".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "lsb".to_string(),
                    param_type: "u8".to_string(),
                    description: "最低位位置（0-15）".to_string(),
                    default_value: Some("0".to_string()),
                },
                RpcParam {
                    name: "msb".to_string(),
                    param_type: "u8".to_string(),
                    description: "最高位位置（0-15）".to_string(),
                    default_value: Some("15".to_string()),
                },
                RpcParam {
                    name: "regVal".to_string(),
                    param_type: "u16".to_string(),
                    description: "要写入的值".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "C".to_string(),
            name: KEY_GH3X_CHIP_CTRL.to_string(),
            description: "芯片控制命令（复位、休眠等）".to_string(),
            params: vec![
                RpcParam {
                    name: "ctrlType".to_string(),
                    param_type: "u8".to_string(),
                    description: "控制类型（0xC2: 软复位, 0xC3: 唤醒, 0xC4: 睡眠）".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "D".to_string(),
            name: KEY_DOWNLOAD_CONFIG.to_string(),
            description: "下载配置到芯片".to_string(),
            params: vec![
                RpcParam {
                    name: "stage".to_string(),
                    param_type: "u8".to_string(),
                    description: "下载阶段".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "L".to_string(),
            name: KEY_GH3X_REGS_LIST_WRITE_CMD.to_string(),
            description: "寄存器列表批量写入命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regs".to_string(),
                    param_type: "u16[]".to_string(),
                    description: "寄存器列表（地址和值交替）".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "S".to_string(),
            name: KEY_GH3X_SW_FUNCTION_CMD.to_string(),
            description: "软件功能命令".to_string(),
            params: vec![
                RpcParam {
                    name: "targetFuncMode".to_string(),
                    param_type: "u32".to_string(),
                    description: "目标功能模式（位掩码）".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "ctrlType".to_string(),
                    param_type: "u8".to_string(),
                    description: "控制类型（0: 启动, 1: 停止）".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "P".to_string(),
            name: KEY_GH_LOW_POWER_CMD.to_string(),
            description: "低功耗命令".to_string(),
            params: vec![
                RpcParam {
                    name: "targetFuncMode".to_string(),
                    param_type: "u32".to_string(),
                    description: "目标功能模式（位掩码）".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "ctrlType".to_string(),
                    param_type: "u8".to_string(),
                    description: "控制类型".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "M".to_string(),
            name: KEY_GH_SET_WORK_MODE_CMD.to_string(),
            description: "设置工作模式".to_string(),
            params: vec![
                RpcParam {
                    name: "workMode".to_string(),
                    param_type: "u8".to_string(),
                    description: "工作模式".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "TS".to_string(),
            name: KEY_GH_TIMESTAMP_SET.to_string(),
            description: "设置时间戳（32 位）".to_string(),
            params: vec![
                RpcParam {
                    name: "timestamp".to_string(),
                    param_type: "u32".to_string(),
                    description: "时间戳值（Unix 时间戳）".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "TM".to_string(),
            name: KEY_GH_TIME_SET.to_string(),
            description: "设置时间（带时区）".to_string(),
            params: vec![
                RpcParam {
                    name: "timestamp".to_string(),
                    param_type: "u32".to_string(),
                    description: "时间戳值（Unix 时间戳）".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "hourOffset".to_string(),
                    param_type: "i8".to_string(),
                    description: "时区偏移（小时）".to_string(),
                    default_value: Some("8".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "FS".to_string(),
            name: KEY_F_SET_MODE.to_string(),
            description: "产测模式设置".to_string(),
            params: vec![
                RpcParam {
                    name: "factoryMode".to_string(),
                    param_type: "u8".to_string(),
                    description: "产测模式（位掩码：0x01=CHIP_INIT, 0x02=CHIP_UID, 0x04=BASE_NOISE, 0x08=PPG_NOISE, 0x10=LPCTR, 0x20=LPLCTR）".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "FG".to_string(),
            name: KEY_F_GET_MODE.to_string(),
            description: "产测模式结果获取".to_string(),
            params: vec![
                RpcParam {
                    name: "factoryMode".to_string(),
                    param_type: "u8".to_string(),
                    description: "产测模式（位掩码）".to_string(),
                    default_value: None,
                },
            ],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionTypeConfig {
    pub type_value: u8,
    pub name: String,
    pub description: String,
}

pub fn get_version_types() -> Vec<VersionTypeConfig> {
    vec![
        VersionTypeConfig {
            type_value: VER_TYPE_FW,
            name: "firmware".to_string(),
            description: "固件版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_VIRTUAL_REG,
            name: "virtual_reg".to_string(),
            description: "虚拟寄存器版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_BOOTLOADER,
            name: "bootloader".to_string(),
            description: "Bootloader版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_PROTOCOL,
            name: "protocol".to_string(),
            description: "协议版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_FUNC_SUPPORT,
            name: "func_support".to_string(),
            description: "功能支持".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_DRV,
            name: "driver".to_string(),
            description: "驱动版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_CHIP,
            name: "chip".to_string(),
            description: "芯片版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_BLE,
            name: "ble".to_string(),
            description: "BLE版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_DEMO,
            name: "demo".to_string(),
            description: "Demo版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_ALGO + HR_VERSION_OFFSET,
            name: "algo_hr".to_string(),
            description: "HR算法版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_ALGO + HRV_VERSION_OFFSET,
            name: "algo_hrv".to_string(),
            description: "HRV算法版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_ALGO + SPO2_VERSION_OFFSET,
            name: "algo_spo2".to_string(),
            description: "SPO2算法版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_ALGO + ADT_VERSION_OFFSET,
            name: "algo_adt".to_string(),
            description: "ADT算法版本".to_string(),
        },
        VersionTypeConfig {
            type_value: VER_TYPE_ALGO + NADT_VERSION_OFFSET,
            name: "algo_nadt".to_string(),
            description: "NADT算法版本".to_string(),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gh3036EventData {
    pub event_type: u8,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl Gh3036EventData {
    pub fn new(event_type: u8, data: &[u8]) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        Self {
            event_type,
            data: data.to_vec(),
            timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gh3036FramesEvent {
    pub function_id: u8,
    pub function_name: String,
    pub frame_count: usize,
    pub channel_count: usize,
    
    pub frame_cnts: Vec<u32>,
    pub timestamps: Vec<u64>,
    pub frame_ids: Vec<u32>,
    
    pub ipd_pa: Vec<Vec<i32>>,
    pub rawdata: Vec<Vec<i32>>,
    pub flags: Vec<Vec<i32>>,
    pub agc_info: Vec<Vec<i32>>,
    
    pub acc_x: Vec<i16>,
    pub acc_y: Vec<i16>,
    pub acc_z: Vec<i16>,
    pub gyro_x: Vec<i16>,
    pub gyro_y: Vec<i16>,
    pub gyro_z: Vec<i16>,
    
    pub algo_results: Vec<Vec<i32>>,
    pub led_drv_fs: Vec<[u8; 2]>,
}

impl Gh3036FramesEvent {
    pub fn new(function_id: u8, function_name: String) -> Self {
        Self {
            function_id,
            function_name,
            frame_count: 0,
            channel_count: 0,
            frame_cnts: Vec::new(),
            timestamps: Vec::new(),
            frame_ids: Vec::new(),
            ipd_pa: Vec::new(),
            rawdata: Vec::new(),
            flags: Vec::new(),
            agc_info: Vec::new(),
            acc_x: Vec::new(),
            acc_y: Vec::new(),
            acc_z: Vec::new(),
            gyro_x: Vec::new(),
            gyro_y: Vec::new(),
            gyro_z: Vec::new(),
            algo_results: Vec::new(),
            led_drv_fs: Vec::new(),
        }
    }

    pub fn add_frame(&mut self, frame: &GhFuncFrame) {
        self.frame_cnts.push(frame.frame_cnt);
        self.timestamps.push(frame.timestamp);
        self.frame_ids.push(frame.frame_cnt);
        
        if self.ipd_pa.is_empty() {
            self.channel_count = frame.ch_num as usize;
            self.ipd_pa = vec![Vec::new(); frame.ch_num as usize];
            self.rawdata = vec![Vec::new(); frame.ch_num as usize];
            self.flags = vec![Vec::new(); frame.ch_num as usize];
            self.agc_info = vec![Vec::new(); frame.ch_num as usize];
        }
        
        for (i, ch_data) in frame.data.iter().enumerate() {
            if i < self.ipd_pa.len() {
                self.ipd_pa[i].push(ch_data.ipd_pa);
                self.rawdata[i].push(ch_data.rawdata);
                
                let mut flag_val = 0i32;
                if ch_data.flag.led_adj_flag { flag_val |= 1; }
                if ch_data.flag.sa_flag { flag_val |= 2; }
                if ch_data.flag.param_change_flag { flag_val |= 4; }
                if ch_data.flag.dre_update { flag_val |= 8; }
                if ch_data.flag.skip_ok_flag { flag_val |= 16; }
                self.flags[i].push(flag_val);
                
                let word0 = (ch_data.agc_info.gain_code as u32)
                    | ((ch_data.agc_info.bg_cancel_range as u32) << 4)
                    | ((ch_data.agc_info.dc_cancel_range as u32) << 6)
                    | ((ch_data.agc_info.dc_cancel_code as u32) << 8)
                    | ((ch_data.agc_info.led_drv0 as u32) << 16)
                    | ((ch_data.agc_info.led_drv1 as u32) << 24);
                self.agc_info[i].push(word0 as i32);
            }
        }
        
        self.acc_x.push(frame.gsensor_data.acc[0]);
        self.acc_y.push(frame.gsensor_data.acc[1]);
        self.acc_z.push(frame.gsensor_data.acc[2]);
        self.gyro_x.push(0);
        self.gyro_y.push(0);
        self.gyro_z.push(0);
        
        self.algo_results.push(Vec::new());
        
        self.led_drv_fs.push(frame.led_drv_fs);
        
        self.frame_count += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.frame_count == 0
    }

    pub fn clear(&mut self) {
        self.frame_count = 0;
        self.channel_count = 0;
        self.frame_cnts.clear();
        self.timestamps.clear();
        self.frame_ids.clear();
        for ch in &mut self.ipd_pa { ch.clear(); }
        for rd in &mut self.rawdata { rd.clear(); }
        for f in &mut self.flags { f.clear(); }
        for a in &mut self.agc_info { a.clear(); }
        self.acc_x.clear();
        self.acc_y.clear();
        self.acc_z.clear();
        self.gyro_x.clear();
        self.gyro_y.clear();
        self.gyro_z.clear();
        self.algo_results.clear();
        self.led_drv_fs.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactoryTestStep {
    Idle,
    Prepare,
    ChipInit,
    Uuid,
    BaseNoise,
    PpgNoise,
    Lpctr,
    EnvironmentSwitch,
    Lplctr,
    Cleanup,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactoryTestStatus {
    Idle,
    Running,
    WaitingForEnvironmentSwitch,
    Completed,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryTestStepResult {
    pub step: FactoryTestStep,
    pub success: bool,
    pub message: String,
    pub data: Vec<u16>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryTestResult {
    pub chip_init_status: u16,
    pub uuid: Vec<u8>,
    pub base_noise: Vec<u16>,
    pub ppg_noise: Vec<u16>,
    pub lpctr: Vec<u16>,
    pub lplctr: Vec<u16>,
    pub overall_result: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationResult {
    pub base_noise_config: Option<String>,
    pub ppg_noise_config: Option<String>,
    pub lpctr_config: Option<String>,
    pub lplctr_config: Option<String>,
    pub errors: Vec<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryTestProgressEvent {
    pub current_step: FactoryTestStep,
    pub status: FactoryTestStatus,
    pub step_result: Option<FactoryTestStepResult>,
    pub progress: f32,
    pub message: String,
}
