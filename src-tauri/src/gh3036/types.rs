//! GH3036 协议数据类型定义
//!
//! 本模块定义 GH3036 协议相关的数据类型

use serde::{Deserialize, Serialize};

/// 加速度轴数量
pub const GH_ACC_AXIS_NUM: usize = 3;

/// 陀螺仪轴数量
pub const GH_GYRO_AXIS_NUM: usize = 3;

/// 功能 ID 枚举
///
/// 对应 C 库的 `gh_func_fix_idx_e` 枚举
/// 用于标识不同的数据功能类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum GhFuncFixIdx {
    /// ADT（活动检测）
    Adt = 0,
    /// HR（心率）
    Hr = 1,
    /// SPO2（血氧）
    Spo2 = 2,
    /// HRV（心率变异性）
    Hrv = 3,
    /// GNADT（通用活动检测）
    Gnadt = 4,
    /// IRNADT（红外活动检测）
    Irnadt = 5,
    /// 测试模式 1
    Test1 = 6,
    /// 测试模式 2
    Test2 = 7,
    /// PPG 配置 0
    PpgCfg0 = 8,
    /// PPG 配置 1
    PpgCfg1 = 9,
    /// PPG 配置 2
    PpgCfg2 = 10,
    /// PPG 配置 3
    PpgCfg3 = 11,
    /// PPG 配置 4
    PpgCfg4 = 12,
    /// PPG 配置 5
    PpgCfg5 = 13,
    /// PPG 配置 6
    PpgCfg6 = 14,
    /// PPG 配置 7
    PpgCfg7 = 15,
    /// 容量配置
    CapCfg = 16,
    /// 最大值
    Max = 17,
}

impl GhFuncFixIdx {
    /// 从 i32 值创建枚举
    ///
    /// # 参数
    /// - `value`: 功能 ID 值
    ///
    /// # 返回
    /// - `Some(GhFuncFixIdx)`: 成功
    /// - `None`: 无效值
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
    
    /// 获取功能名称字符串
    ///
    /// # 返回
    /// 功能名称，用于 CSV 文件命名和显示
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

/// GH3036 帧数据结构
///
/// # 功能
/// 存储从 C 库回调返回的帧数据，用于前端显示和 CSV 保存
///
/// # 字段说明
/// - `function_id`: 功能 ID
/// - `function_name`: 功能名称
/// - `frame_id`: 帧 ID（0 表示新序列开始）
/// - `timestamp`: 时间戳（64 位）
/// - `gs_data`: 加速度/陀螺仪数据
/// - `rawdata`: 原始数据
/// - `flags`: 标志位
/// - `algo_data`: 算法结果
/// - `agc_info`: AGC 信息
/// - `phy_value`: 物理值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gh3036FrameData {
    /// 功能 ID
    pub function_id: i32,
    /// 功能名称
    pub function_name: String,
    /// 帧 ID（0 表示新序列开始）
    pub frame_id: i32,
    /// 时间戳（64 位）
    pub timestamp: u64,
    /// 加速度/陀螺仪数据（前 3 个为 ACC，后 3 个为 GYRO）
    pub gs_data: Vec<i32>,
    /// 原始数据
    pub rawdata: Vec<i32>,
    /// 标志位
    pub flags: Vec<i32>,
    /// 算法结果
    pub algo_data: Vec<i32>,
    /// AGC 信息
    pub agc_info: Vec<i32>,
    /// 物理值
    pub phy_value: Vec<i32>,
}

impl Gh3036FrameData {
    /// 从 C 库 DataFrame 创建 Rust 结构
    ///
    /// # 参数
    /// - `frame`: C 库帧数据指针
    ///
    /// # 返回
    /// Rust 帧数据结构
    ///
    /// # 安全性
    /// 需要确保 frame 指针有效
    pub fn from_c_frame(frame: &super::ffi::DataFrame) -> Self {
        let func_id = frame.function_id;
        let func_name = GhFuncFixIdx::from_i32(func_id)
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| format!("UNKNOWN_{}", func_id));
        
        let timestamp = ((frame.timestamp_high as u64) << 32) | (frame.timestamp as u64);
        
        let gs_data = if !frame.p_gs_data.is_null() && frame.gs_data_size > 0 {
            unsafe { std::slice::from_raw_parts(frame.p_gs_data, frame.gs_data_size as usize).to_vec() }
        } else {
            vec![]
        };
        
        let rawdata = if !frame.p_rawdata.is_null() && frame.rawdata_size > 0 {
            unsafe { std::slice::from_raw_parts(frame.p_rawdata, frame.rawdata_size as usize).to_vec() }
        } else {
            vec![]
        };
        
        let flags = if !frame.p_flags.is_null() && frame.flag_data_bits > 0 {
            let len = ((frame.flag_data_bits + 31) / 32) as usize;
            unsafe { std::slice::from_raw_parts(frame.p_flags, len).to_vec() }
        } else {
            vec![]
        };
        
        let algo_data = if !frame.p_algo_data.is_null() && frame.algo_data_bits > 0 {
            let len = ((frame.algo_data_bits + 31) / 32) as usize;
            unsafe { std::slice::from_raw_parts(frame.p_algo_data, len).to_vec() }
        } else {
            vec![]
        };
        
        let agc_info = if !frame.p_agc_info.is_null() && frame.agc_info_size > 0 {
            unsafe { std::slice::from_raw_parts(frame.p_agc_info, frame.agc_info_size as usize).to_vec() }
        } else {
            vec![]
        };
        
        let phy_value = if !frame.p_phy_value.is_null() && frame.phy_value_size > 0 {
            unsafe { std::slice::from_raw_parts(frame.p_phy_value, frame.phy_value_size as usize).to_vec() }
        } else {
            vec![]
        };
        
        Self {
            function_id: func_id,
            function_name: func_name,
            frame_id: frame.frame_id,
            timestamp,
            gs_data,
            rawdata,
            flags,
            algo_data,
            agc_info,
            phy_value,
        }
    }
}

/// RPC 命令定义
///
/// # 功能
/// 定义 RPC 指令的结构，用于前端显示和执行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCommand {
    /// 命令键（如 "V" 表示获取版本）
    pub key: String,
    /// 命令名称
    pub name: String,
    /// 命令描述
    pub description: String,
    /// 参数列表
    pub params: Vec<RpcParam>,
}

/// RPC 参数定义
///
/// # 功能
/// 定义 RPC 指令参数的结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcParam {
    /// 参数名称
    pub name: String,
    /// 参数类型（如 "u8", "u16[]", "u32"）
    pub param_type: String,
    /// 参数描述
    pub description: String,
    /// 默认值
    pub default_value: Option<String>,
}

/// 获取 RPC 命令列表
///
/// # 功能
/// 返回所有支持的 RPC 命令
///
/// # 返回
/// RPC 命令列表
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
                    description: "版本类型（0: 固件版本, 1: 协议版本）".to_string(),
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
                    description: "寄存器数据数组（地址和值交替，如 [0x1000, 0x1234] 表示写入 0x1234 到地址 0x1000）".to_string(),
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
            name: "GH3X_ChipCtrl".to_string(),
            description: "芯片控制命令（复位、休眠等）".to_string(),
            params: vec![
                RpcParam {
                    name: "ctrlType".to_string(),
                    param_type: "u8".to_string(),
                    description: "控制类型（0: 复位, 1: 休眠, 2: 唤醒）".to_string(),
                    default_value: None,
                },
            ],
        },
        RpcCommand {
            key: "D".to_string(),
            name: "download_config".to_string(),
            description: "下载配置到芯片".to_string(),
            params: vec![
                RpcParam {
                    name: "stage".to_string(),
                    param_type: "u8".to_string(),
                    description: "下载阶段（多阶段下载时使用）".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "L".to_string(),
            name: "GH3X_RegsListWriteCmd".to_string(),
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
            name: "GH3X_SwFunctionCmd".to_string(),
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
            name: "gh_low_power_cmd".to_string(),
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
                    description: "控制类型（0: 进入低功耗, 1: 退出低功耗）".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "FW".to_string(),
            name: "GH3X_FwUpdateCmd".to_string(),
            description: "固件更新命令".to_string(),
            params: vec![
                RpcParam {
                    name: "firmwarePath".to_string(),
                    param_type: "string".to_string(),
                    description: "固件文件路径".to_string(),
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
                    description: "寄存器位域数据（地址、LSB、MSB、值交替）".to_string(),
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
                    description: "工作模式（0: 正常, 1: 测试, 2: 校准）".to_string(),
                    default_value: Some("0".to_string()),
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
                    description: "状态类型（0: 连接状态, 1: 通信质量）".to_string(),
                    default_value: Some("0".to_string()),
                },
            ],
        },
        RpcCommand {
            key: "TS".to_string(),
            name: "gh_timestamp_set".to_string(),
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
            name: "gh_time_set".to_string(),
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
                    description: "时区偏移（小时，如东八区为 8）".to_string(),
                    default_value: Some("8".to_string()),
                },
            ],
        },
    ]
}

/// GH3036 事件数据
///
/// # 功能
/// 存储从 C 库 event_callback 返回的事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gh3036EventData {
    /// 事件类型
    pub event_type: u8,
    /// 事件数据
    pub data: Vec<u8>,
    /// 时间戳
    pub timestamp: u64,
}

impl Gh3036EventData {
    /// 创建新的事件数据
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
