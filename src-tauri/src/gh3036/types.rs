//! GH3036 协议数据类型定义
//!
//! 本模块定义 GH3036 协议相关的数据类型

use serde::{Deserialize, Serialize};

pub use gh_rpc::data_package::{FuncFrame, DataFrame, FuncId, ChannelData, GSensorData, AgcInfo, FrameDataFlag, FrameDecoder};
pub use gh_rpc::cmd::{
    CMD_GET_VERSION, CMD_REGS_WRITE, CMD_REGS_READ, CMD_REG_BIT_FIELD_WRITE,
    CMD_CHIP_CTRL, CMD_SW_FUNCTION, CMD_DOWNLOAD_CONFIG, CMD_REGS_LIST_WRITE,
    CMD_FW_UPDATE, CMD_GET_CHIP_LINK_STATUS, CMD_TIMESTAMP_SET, CMD_TIME_SET,
    CMD_SET_WORK_MODE, CMD_LOW_POWER, CMD_REGS_BIT_FIELD_WRITE,
    CMD_FACTORY_SET_MODE, CMD_FACTORY_GET_MODE,
    VER_TYPE_FW, VER_TYPE_DEMO, VER_TYPE_BOOTLOADER, VER_TYPE_PROTOCOL,
    VER_TYPE_VIRTUAL_REG, VER_TYPE_DRV, VER_TYPE_CHIP, VER_TYPE_BLE, VER_TYPE_ALGO,
    VER_TYPE_FUNC_SUPPORT,
    HR_VERSION_OFFSET, HRV_VERSION_OFFSET, SPO2_VERSION_OFFSET, ADT_VERSION_OFFSET, NADT_VERSION_OFFSET,
    CHIP_CTRL_HARD_RESET, CHIP_CTRL_RX_RESET, CHIP_CTRL_SOFT_RESET,
    CHIP_CTRL_WAKEUP, CHIP_CTRL_SLEEP,
};

pub use rpc::PackHeader;

pub const GH_ACC_AXIS_NUM: usize = 3;
pub const GH_GYRO_AXIS_NUM: usize = 3;

pub const FACTORY_TEST_MODE_CHIP_INIT_OFFSET: u8 = 1;
pub const FACTORY_TEST_MODE_CHIP_UID_OFFSET: u8 = 2;
pub const FACTORY_TEST_MODE_BASE_NOISE_OFFSET: u8 = 3;
pub const FACTORY_TEST_MODE_PPG_NOISE_OFFSET: u8 = 4;
pub const FACTORY_TEST_MODE_LPCTR_OFFSET: u8 = 5;
pub const FACTORY_TEST_MODE_LPLCTR_OFFSET: u8 = 6;

pub const FACTORY_TEST_MODE_NONE: u8 = 0;
pub const FACTORY_TEST_MODE_CHIP_INIT: u8 = 1 << FACTORY_TEST_MODE_CHIP_INIT_OFFSET;
pub const FACTORY_TEST_MODE_CHIP_UID: u8 = 1 << FACTORY_TEST_MODE_CHIP_UID_OFFSET;
pub const FACTORY_TEST_MODE_BASE_NOISE: u8 = 1 << FACTORY_TEST_MODE_BASE_NOISE_OFFSET;
pub const FACTORY_TEST_MODE_PPG_NOISE: u8 = 1 << FACTORY_TEST_MODE_PPG_NOISE_OFFSET;
pub const FACTORY_TEST_MODE_LPCTR: u8 = 1 << FACTORY_TEST_MODE_LPCTR_OFFSET;
pub const FACTORY_TEST_MODE_LPLCTR: u8 = 1 << FACTORY_TEST_MODE_LPLCTR_OFFSET;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum GhFuncFixIdx {
    Adt = 0,
    Hr = 1,
    Spo2 = 2,
    Hrv = 3,
    Gnadt = 4,
    Irnadt = 5,
    Test1 = 6,
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

impl GhFuncFixIdx {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Adt),
            1 => Some(Self::Hr),
            2 => Some(Self::Spo2),
            3 => Some(Self::Hrv),
            4 => Some(Self::Gnadt),
            5 => Some(Self::Irnadt),
            6 => Some(Self::Test1),
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
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::Adt => "ADT",
            Self::Hr => "HR",
            Self::Spo2 => "SPO2",
            Self::Hrv => "HRV",
            Self::Gnadt => "GNADT",
            Self::Irnadt => "IRNADT",
            Self::Test1 => "TEST1",
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

impl From<FuncId> for GhFuncFixIdx {
    fn from(func_id: FuncId) -> Self {
        match func_id {
            FuncId::ADT => Self::Adt,
            FuncId::HR => Self::Hr,
            FuncId::SPO2 => Self::Spo2,
            FuncId::HRV => Self::Hrv,
            FuncId::GNADT => Self::Gnadt,
            FuncId::IRNADT => Self::Irnadt,
            _ => Self::Max,
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
    pub fn from_func_frame(frame: &FuncFrame) -> Self {
        let func_id = GhFuncFixIdx::from(frame.id);
        let function_name = func_id.name().to_string();
        
        let gs_data: Vec<i32> = [
            frame.gsensor_data.acc[0] as i32,
            frame.gsensor_data.acc[1] as i32,
            frame.gsensor_data.acc[2] as i32,
            frame.gsensor_data.gyro[0] as i32,
            frame.gsensor_data.gyro[1] as i32,
            frame.gsensor_data.gyro[2] as i32,
        ].to_vec();
        
        let rawdata: Vec<i32> = frame.p_data.iter().map(|d| d.rawdata).collect();
        let phy_value: Vec<i32> = frame.p_data.iter().map(|d| d.ipd_pa).collect();
        
        let algo_data: Vec<i32> = frame.p_algo_res.iter().map(|&v| v as i32).collect();
        
        let agc_info: Vec<i32> = frame.p_data.iter()
            .flat_map(|d| {
                let low = d.agc_info.to_low_u32() as i32;
                let high = d.agc_info.to_high_u32() as i32;
                [low, high]
            })
            .collect();
        
        let flags: Vec<i32> = frame.p_data.iter()
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
            function_id: func_id as i32,
            function_name,
            frame_id: frame.frame_cnt as i32,
            timestamp: frame.timestamp,
            gs_data,
            rawdata,
            flags,
            algo_data,
            agc_info,
            phy_value,
        }
    }
    
    pub fn from_data_frame(frame: &DataFrame) -> Self {
        let func_id = GhFuncFixIdx::from_i32(frame.function_id);
        let function_name = func_id
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| format!("UNKNOWN_{}", frame.function_id));
        
        let timestamp = ((frame.timestamp_high as u64) << 32) | (frame.timestamp as u64);
        
        Self {
            function_id: frame.function_id,
            function_name,
            frame_id: frame.frame_id,
            timestamp,
            gs_data: frame.gs_data.iter().copied().collect(),
            rawdata: frame.rawdata.iter().copied().collect(),
            flags: frame.flags.iter().map(|&v| v as i32).collect(),
            algo_data: frame.algo_data.iter().copied().collect(),
            agc_info: {
                let mut info = Vec::new();
                for i in 0..frame.agc_info_size {
                    if i < frame.agc_info.len() && i < frame.agc_info_high.len() {
                        info.push(frame.agc_info[i]);
                        info.push(frame.agc_info_high[i]);
                    }
                }
                info
            },
            phy_value: frame.phy_value.iter().copied().collect(),
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
            name: CMD_GET_VERSION.to_string(),
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
            name: CMD_REGS_WRITE.to_string(),
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
            name: CMD_REGS_READ.to_string(),
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
            name: CMD_REG_BIT_FIELD_WRITE.to_string(),
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
            name: CMD_CHIP_CTRL.to_string(),
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
            name: CMD_DOWNLOAD_CONFIG.to_string(),
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
            name: CMD_REGS_LIST_WRITE.to_string(),
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
            name: CMD_SW_FUNCTION.to_string(),
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
            name: CMD_LOW_POWER.to_string(),
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
            name: CMD_SET_WORK_MODE.to_string(),
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
            name: CMD_TIMESTAMP_SET.to_string(),
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
            name: CMD_TIME_SET.to_string(),
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
            name: CMD_FACTORY_SET_MODE.to_string(),
            description: "产测模式设置".to_string(),
            params: vec![
                RpcParam {
                    name: "factoryMode".to_string(),
                    param_type: "u8".to_string(),
                    description: "产测模式（位掩码：0x02=CHIP_INIT, 0x04=CHIP_UID, 0x08=BASE_NOISE, 0x10=PPG_NOISE, 0x20=LPCTR, 0x40=LPLCTR）".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "FG".to_string(),
            name: CMD_FACTORY_GET_MODE.to_string(),
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

    pub fn add_frame(&mut self, frame: &FuncFrame) {
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
        
        for (i, ch_data) in frame.p_data.iter().enumerate() {
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
                
                self.agc_info[i].push(ch_data.agc_info.to_low_u32() as i32);
                self.agc_info[i].push(ch_data.agc_info.to_high_u32() as i32);
            }
        }
        
        self.acc_x.push(frame.gsensor_data.acc[0]);
        self.acc_y.push(frame.gsensor_data.acc[1]);
        self.acc_z.push(frame.gsensor_data.acc[2]);
        self.gyro_x.push(frame.gsensor_data.gyro[0]);
        self.gyro_y.push(frame.gsensor_data.gyro[1]);
        self.gyro_z.push(frame.gsensor_data.gyro[2]);
        
        let algo: Vec<i32> = frame.p_algo_res.iter().map(|&v| v as i32).collect();
        self.algo_results.push(algo);
        
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
