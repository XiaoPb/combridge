use serde::{Deserialize, Serialize};

pub const GH_ACC_AXIS_NUM: usize = 3;
pub const GH_GYRO_AXIS_NUM: usize = 3;

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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PackHeader {
    pub rawdata_en: u32,
    pub phy_value_en: u32,
    pub gs_data_en: u32,
    pub flags_en: u32,
    pub alg_data_en: u32,
    pub agc_info_en: u32,
    pub timestamp_en: u32,
    pub frameid_en: u32,
    pub func_id_en: u32,
    pub slot_cfg_en: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFrame {
    pub pack_header: PackHeader,
    pub slot_cfg: i32,
    pub function_id: i32,
    pub frame_id: i32,
    pub timestamp: i32,
    pub timestamp_high: i32,
    pub agc_info: Vec<i32>,
    pub agc_info_high: Vec<i32>,
    pub algo_data: Vec<i32>,
    pub algo_data_bits: i32,
    pub flags: Vec<i32>,
    pub flag_data_bits: i32,
    pub gs_data: Vec<i32>,
    pub gs_data_size: i32,
    pub phy_value: Vec<i32>,
    pub phy_value_size: i32,
    pub rawdata: Vec<i32>,
    pub rawdata_size: i32,
}

impl Default for DataFrame {
    fn default() -> Self {
        Self {
            pack_header: PackHeader::default(),
            slot_cfg: 0,
            function_id: 0,
            frame_id: 0,
            timestamp: 0,
            timestamp_high: 0,
            agc_info: Vec::new(),
            agc_info_high: Vec::new(),
            algo_data: Vec::new(),
            algo_data_bits: 0,
            flags: Vec::new(),
            flag_data_bits: 0,
            gs_data: Vec::new(),
            gs_data_size: 0,
            phy_value: Vec::new(),
            phy_value_size: 0,
            rawdata: Vec::new(),
            rawdata_size: 0,
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

impl From<&DataFrame> for Gh3036FrameData {
    fn from(frame: &DataFrame) -> Self {
        let func_id = frame.function_id;
        let func_name = GhFuncFixIdx::from_i32(func_id)
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| format!("UNKNOWN_{}", func_id));
        
        let timestamp = ((frame.timestamp_high as u64) << 32) | (frame.timestamp as u64);
        
        Self {
            function_id: func_id,
            function_name: func_name,
            frame_id: frame.frame_id,
            timestamp,
            gs_data: frame.gs_data.clone(),
            rawdata: frame.rawdata.clone(),
            flags: frame.flags.clone(),
            algo_data: frame.algo_data.clone(),
            agc_info: frame.agc_info.clone(),
            phy_value: frame.phy_value.clone(),
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
            name: "GH3X_GetVersion".to_string(),
            description: "获取芯片版本信息".to_string(),
            params: vec![
                RpcParam {
                    name: "verType".to_string(),
                    param_type: "u8".to_string(),
                    description: "版本类型".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "W".to_string(),
            name: "GH3X_RegsWriteCmd".to_string(),
            description: "寄存器写入命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regs".to_string(),
                    param_type: "u16[]".to_string(),
                    description: "寄存器数据数组".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "R".to_string(),
            name: "GH3X_RegsReadCmd".to_string(),
            description: "寄存器读取命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regAddr".to_string(),
                    param_type: "u16".to_string(),
                    description: "寄存器地址".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "readLen".to_string(),
                    param_type: "i32".to_string(),
                    description: "读取长度".to_string(),
                    default_value: Some("1".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "B".to_string(),
            name: "GH3X_RegBitFieldWriteCmd".to_string(),
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
                    description: "最低位".to_string(),
                    default_value: Some("0".to_string()),
                },
                RpcParam {
                    name: "msb".to_string(),
                    param_type: "u8".to_string(),
                    description: "最高位".to_string(),
                    default_value: Some("15".to_string()),
                },
                RpcParam {
                    name: "regVal".to_string(),
                    param_type: "u16".to_string(),
                    description: "寄存器值".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "C".to_string(),
            name: "GH3X_ChipCtrl".to_string(),
            description: "芯片控制命令".to_string(),
            params: vec![
                RpcParam {
                    name: "ctrlType".to_string(),
                    param_type: "u8".to_string(),
                    description: "控制类型".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "D".to_string(),
            name: "download_config".to_string(),
            description: "下载配置".to_string(),
            params: vec![
                RpcParam {
                    name: "stage".to_string(),
                    param_type: "u8".to_string(),
                    description: "阶段".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "L".to_string(),
            name: "GH3X_RegsListWriteCmd".to_string(),
            description: "寄存器列表写入命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regs".to_string(),
                    param_type: "u16[]".to_string(),
                    description: "寄存器列表".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "S".to_string(),
            name: "GH3X_SwFunctionCmd".to_string(),
            description: "软件功能命令".to_string(),
            params: vec![
                RpcParam {
                    name: "targetFuncMode".to_string(),
                    param_type: "u32".to_string(),
                    description: "目标功能模式".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "ctrlType".to_string(),
                    param_type: "u8".to_string(),
                    description: "控制类型".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "P".to_string(),
            name: "gh_low_power_cmd".to_string(),
            description: "低功耗命令".to_string(),
            params: vec![
                RpcParam {
                    name: "targetFuncMode".to_string(),
                    param_type: "u32".to_string(),
                    description: "目标功能模式".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "ctrlType".to_string(),
                    param_type: "u8".to_string(),
                    description: "控制类型".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "FW".to_string(),
            name: "GH3X_FwUpdateCmd".to_string(),
            description: "固件更新命令".to_string(),
            params: vec![
                RpcParam {
                    name: "src".to_string(),
                    param_type: "u8[]".to_string(),
                    description: "固件数据".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "BF".to_string(),
            name: "GH3X_RegsBitFieldWriteCmd".to_string(),
            description: "寄存器位域批量写入命令".to_string(),
            params: vec![
                RpcParam {
                    name: "regBits".to_string(),
                    param_type: "u16[]".to_string(),
                    description: "寄存器位域数据".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "M".to_string(),
            name: "GHSetWorkModeCmd".to_string(),
            description: "设置工作模式".to_string(),
            params: vec![
                RpcParam {
                    name: "workMode".to_string(),
                    param_type: "u8".to_string(),
                    description: "工作模式".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "LS".to_string(),
            name: "get_chip_link_status".to_string(),
            description: "获取芯片链路状态".to_string(),
            params: vec![
                RpcParam {
                    name: "type".to_string(),
                    param_type: "u8".to_string(),
                    description: "状态类型".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "TS".to_string(),
            name: "gh_timestamp_set".to_string(),
            description: "设置时间戳".to_string(),
            params: vec![
                RpcParam {
                    name: "timestamp".to_string(),
                    param_type: "u32".to_string(),
                    description: "时间戳".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "TM".to_string(),
            name: "gh_time_set".to_string(),
            description: "设置时间".to_string(),
            params: vec![
                RpcParam {
                    name: "timestamp".to_string(),
                    param_type: "u32".to_string(),
                    description: "时间戳".to_string(),
                    default_value: None,
                },
                RpcParam {
                    name: "hourOffset".to_string(),
                    param_type: "i8".to_string(),
                    description: "时区偏移".to_string(),
                    default_value: Some("8".to_string()),
                },
            ],
        },
    ]
}
