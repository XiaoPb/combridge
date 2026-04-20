//! # 命令处理模块
//!
//! 本模块提供命令处理功能，兼容 C 版本的 `gh_protocol_cmd.c`。
//!
//! ## 主要组件
//!
//! - [`Command`]: 命令枚举，定义所有支持的命令
//! - [`Response`]: 响应枚举，定义所有响应类型
//! - [`CommandRegistry`]: 命令注册表，管理命令处理器
//!
//! ## 示例
//!
//! ```rust
//! use gh_rpc::cmd::{Command, CommandRegistry, Response, CommandError, CMD_GET_VERSION};
//!
//! let mut registry: CommandRegistry<16> = CommandRegistry::new();
//!
//! fn version_handler(_cmd: Command) -> Result<Response, CommandError> {
//!     Ok(Response::Empty)
//! }
//!
//! registry.register(CMD_GET_VERSION, version_handler).unwrap();
//! ```

use heapless::Vec as HeaplessVec;

/// 获取版本命令键
pub const CMD_GET_VERSION: &str = "GH3X_GetVersion";
/// 寄存器写入命令键
pub const CMD_REGS_WRITE: &str = "GH3X_RegsWriteCmd";
/// 寄存器读取命令键
pub const CMD_REGS_READ: &str = "GH3X_RegsReadCmd";
/// 寄存器位域写入命令键
pub const CMD_REG_BIT_FIELD_WRITE: &str = "GH3X_RegBitFieldWriteCmd";
/// 芯片控制命令键
pub const CMD_CHIP_CTRL: &str = "GH3X_ChipCtrl";
/// 软件功能命令键
pub const CMD_SW_FUNCTION: &str = "GH3X_SwFunctionCmd";
/// 下载配置命令键
pub const CMD_DOWNLOAD_CONFIG: &str = "download_config";
/// 寄存器列表写入命令键
pub const CMD_REGS_LIST_WRITE: &str = "GH3X_RegsListWriteCmd";
/// 固件更新命令键
pub const CMD_FW_UPDATE: &str = "FW";
/// 获取芯片链路状态命令键
pub const CMD_GET_CHIP_LINK_STATUS: &str = "get_chip_link_status";
/// 时间戳设置命令键
pub const CMD_TIMESTAMP_SET: &str = "gh_timestamp_set";
/// 时间设置命令键
pub const CMD_TIME_SET: &str = "gh_time_set";
/// 设置工作模式命令键
pub const CMD_SET_WORK_MODE: &str = "GHSetWorkModeCmd";
/// 低功耗命令键
pub const CMD_LOW_POWER: &str = "gh_low_power_cmd";
/// 寄存器位域写入命令键（别名）
pub const CMD_REGS_BIT_FIELD_WRITE: &str = "GH3X_RegsBitFieldWriteCmd";
/// 产测模式设置命令键
pub const CMD_FACTORY_SET_MODE: &str = "F_SetMode";
/// 产测模式获取命令键
pub const CMD_FACTORY_GET_MODE: &str = "F_GetMode";

/// 硬复位控制类型
pub const CHIP_CTRL_HARD_RESET: u8 = 0x5A;
/// RX 复位控制类型
pub const CHIP_CTRL_RX_RESET: u8 = 0x5B;
/// 软复位控制类型
pub const CHIP_CTRL_SOFT_RESET: u8 = 0xC2;
/// 唤醒控制类型
pub const CHIP_CTRL_WAKEUP: u8 = 0xC3;
/// 睡眠控制类型
pub const CHIP_CTRL_SLEEP: u8 = 0xC4;

/// 固件版本类型
pub const VER_TYPE_FW: u8 = 0x01;
/// 虚拟寄存器版本类型
pub const VER_TYPE_VIRTUAL_REG: u8 = 0x03;
/// Bootloader 版本类型
pub const VER_TYPE_BOOTLOADER: u8 = 0x04;
/// 协议版本类型
pub const VER_TYPE_PROTOCOL: u8 = 0x05;
/// 功能支持版本类型
pub const VER_TYPE_FUNC_SUPPORT: u8 = 0x06;
/// 驱动版本类型
pub const VER_TYPE_DRV: u8 = 0x07;
/// 芯片版本类型
pub const VER_TYPE_CHIP: u8 = 0x08;
/// BLE 版本类型
pub const VER_TYPE_BLE: u8 = 0x09;
/// Demo 版本类型
pub const VER_TYPE_DEMO: u8 = 0x0A;
/// 算法版本类型
pub const VER_TYPE_ALGO: u8 = 0x20;

/// 心率版本偏移
pub const HR_VERSION_OFFSET: u8 = 0x00;
/// HRV 版本偏移
pub const HRV_VERSION_OFFSET: u8 = 0x01;
/// SpO2 版本偏移
pub const SPO2_VERSION_OFFSET: u8 = 0x02;
/// NADT 版本偏移
pub const ADT_VERSION_OFFSET: u8 = 0x03;
/// NADT 版本偏移
pub const NADT_VERSION_OFFSET: u8 = 0x04;

const MAX_REGS_LIST: usize = 300;
const MAX_VERSION_LEN: usize = 150;

/// 命令错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// 无效的命令
    InvalidCommand,
    /// 无效的参数
    InvalidParameter,
    /// 不支持的操作
    NotSupported,
    /// 缓冲区溢出
    BufferOverflow,
    /// 处理器未找到
    HandlerNotFound,
}

/// 命令枚举
///
/// 定义所有支持的命令类型。
#[derive(Debug, Clone)]
pub enum Command {
    /// 获取版本
    GetVersion {
        /// 版本类型
        ver_type: u8,
    },
    /// 寄存器写入
    RegsWrite {
        /// 寄存器地址
        addr: u16,
        /// 写入值
        value: u16,
    },
    /// 寄存器读取
    RegsRead {
        /// 寄存器地址
        addr: u16,
        /// 读取长度
        len: i32,
    },
    /// 寄存器位域写入
    RegBitFieldWrite {
        /// 寄存器地址
        addr: u16,
        /// 最低位
        lsb: u8,
        /// 最高位
        msb: u8,
        /// 写入值
        value: u16,
    },
    /// 芯片控制
    ChipCtrl {
        /// 控制类型
        ctrl_type: u8,
    },
    /// 软件功能
    SwFunction {
        /// 功能模式
        func_mode: u32,
        /// 控制类型
        ctrl_type: u8,
    },
    /// 下载配置
    DownloadConfig {
        /// 阶段
        stage: u8,
    },
    /// 寄存器列表写入
    RegsListWrite {
        /// 寄存器列表
        regs: HeaplessVec<u16, MAX_REGS_LIST>,
    },
    /// 固件更新
    FwUpdate {
        /// 固件数据
        data: HeaplessVec<u8, 256>,
    },
    /// 获取芯片链路状态
    GetChipLinkStatus {
        /// 命令类型
        cmd_type: u8,
    },
    /// 时间戳设置
    TimestampSet {
        /// 时间戳
        ts: u32,
    },
    /// 时间设置
    TimeSet {
        /// 时间戳
        ts: u32,
        /// 小时偏移
        hour_offset: i8,
    },
    /// 设置工作模式
    SetWorkMode {
        /// 工作模式
        mode: u8,
    },
    /// 低功耗
    LowPower {
        /// 功能模式
        func_mode: u32,
        /// 控制类型
        ctrl_type: u8,
    },
    /// 寄存器位域写入（别名）
    RegsBitFieldWrite {
        /// 寄存器列表
        regs: HeaplessVec<u16, MAX_REGS_LIST>,
    },
}

/// 响应枚举
///
/// 定义所有响应类型。
#[derive(Debug, Clone)]
pub enum Response {
    /// 版本响应
    Version {
        /// 版本数据
        data: HeaplessVec<u8, MAX_VERSION_LEN>,
    },
    /// 寄存器响应
    Regs {
        /// 寄存器数据
        data: HeaplessVec<u16, 200>,
    },
    /// 状态响应
    Status {
        /// 状态码
        status: i8,
    },
    /// 固件更新响应
    FwUpdate {
        /// 固件数据
        data: HeaplessVec<u8, 100>,
    },
    /// 空响应
    Empty,
}

/// 命令处理器 trait
pub trait CommandHandler {
    /// 处理命令
    fn handle(&self, cmd: Command) -> Result<Response, CommandError>;
}

/// 命令处理函数类型
pub type CommandHandlerFn = fn(Command) -> Result<Response, CommandError>;

/// 命令注册表
///
/// 管理命令处理器的注册和分发。
///
/// # 类型参数
///
/// - `N`: 最大注册命令数量
pub struct CommandRegistry<const N: usize> {
    handlers: [Option<(&'static str, CommandHandlerFn)>; N],
    count: usize,
}

impl<const N: usize> CommandRegistry<N> {
    /// 创建新的命令注册表
    pub fn new() -> Self {
        Self {
            handlers: [None; N],
            count: 0,
        }
    }

    /// 注册命令处理器
    ///
    /// # 参数
    ///
    /// - `key`: 命令键
    /// - `handler`: 处理函数
    ///
    /// # 错误
    ///
    /// - `CommandError::BufferOverflow`: 注册表已满
    pub fn register(&mut self, key: &'static str, handler: CommandHandlerFn) -> Result<(), CommandError> {
        if self.count >= N {
            return Err(CommandError::BufferOverflow);
        }
        self.handlers[self.count] = Some((key, handler));
        self.count += 1;
        Ok(())
    }

    /// 分发命令
    ///
    /// # 参数
    ///
    /// - `key`: 命令键
    /// - `cmd`: 命令对象
    ///
    /// # 错误
    ///
    /// - `CommandError::HandlerNotFound`: 处理器未找到
    pub fn dispatch(&self, key: &str, cmd: Command) -> Result<Response, CommandError> {
        for i in 0..self.count {
            if let Some((k, handler)) = self.handlers[i] {
                if k == key {
                    return handler(cmd);
                }
            }
        }
        Err(CommandError::HandlerNotFound)
    }

    /// 获取处理器
    ///
    /// # 参数
    ///
    /// - `key`: 命令键
    ///
    /// # 返回值
    ///
    /// 处理函数，如果未找到则返回 `None`
    pub fn get_handler(&self, key: &str) -> Option<CommandHandlerFn> {
        for i in 0..self.count {
            if let Some((k, handler)) = self.handlers[i] {
                if k == key {
                    return Some(handler);
                }
            }
        }
        None
    }
}

impl<const N: usize> Default for CommandRegistry<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析命令
///
/// 从命令键和数据解析命令对象。
///
/// # 参数
///
/// - `key`: 命令键
/// - `data`: 命令数据
///
/// # 错误
///
/// - `CommandError::InvalidCommand`: 无效的命令键
/// - `CommandError::InvalidParameter`: 无效的参数
pub fn parse_command(key: &str, data: &[u8]) -> Result<Command, CommandError> {
    match key {
        CMD_GET_VERSION => {
            if data.is_empty() {
                return Err(CommandError::InvalidParameter);
            }
            Ok(Command::GetVersion { ver_type: data[0] })
        }
        CMD_REGS_WRITE => {
            if data.len() < 4 {
                return Err(CommandError::InvalidParameter);
            }
            let addr = u16::from_le_bytes([data[0], data[1]]);
            let value = u16::from_le_bytes([data[2], data[3]]);
            Ok(Command::RegsWrite { addr, value })
        }
        CMD_REGS_READ => {
            if data.len() < 6 {
                return Err(CommandError::InvalidParameter);
            }
            let addr = u16::from_le_bytes([data[0], data[1]]);
            let len = i32::from_le_bytes([data[2], data[3], data[4], data[5]]);
            Ok(Command::RegsRead { addr, len })
        }
        CMD_CHIP_CTRL => {
            if data.is_empty() {
                return Err(CommandError::InvalidParameter);
            }
            Ok(Command::ChipCtrl { ctrl_type: data[0] })
        }
        CMD_SW_FUNCTION => {
            if data.len() < 5 {
                return Err(CommandError::InvalidParameter);
            }
            let func_mode = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            let ctrl_type = data[4];
            Ok(Command::SwFunction { func_mode, ctrl_type })
        }
        CMD_DOWNLOAD_CONFIG => {
            if data.is_empty() {
                return Err(CommandError::InvalidParameter);
            }
            Ok(Command::DownloadConfig { stage: data[0] })
        }
        CMD_TIMESTAMP_SET => {
            if data.len() < 4 {
                return Err(CommandError::InvalidParameter);
            }
            let ts = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            Ok(Command::TimestampSet { ts })
        }
        CMD_TIME_SET => {
            if data.len() < 5 {
                return Err(CommandError::InvalidParameter);
            }
            let ts = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            let hour_offset = data[4] as i8;
            Ok(Command::TimeSet { ts, hour_offset })
        }
        CMD_SET_WORK_MODE => {
            if data.is_empty() {
                return Err(CommandError::InvalidParameter);
            }
            Ok(Command::SetWorkMode { mode: data[0] })
        }
        CMD_LOW_POWER => {
            if data.len() < 5 {
                return Err(CommandError::InvalidParameter);
            }
            let func_mode = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            let ctrl_type = data[4];
            Ok(Command::LowPower { func_mode, ctrl_type })
        }
        CMD_GET_CHIP_LINK_STATUS => {
            if data.is_empty() {
                return Err(CommandError::InvalidParameter);
            }
            Ok(Command::GetChipLinkStatus { cmd_type: data[0] })
        }
        CMD_REG_BIT_FIELD_WRITE => {
            if data.len() < 6 {
                return Err(CommandError::InvalidParameter);
            }
            let addr = u16::from_le_bytes([data[0], data[1]]);
            let lsb = data[2];
            let msb = data[3];
            let value = u16::from_le_bytes([data[4], data[5]]);
            Ok(Command::RegBitFieldWrite { addr, lsb, msb, value })
        }
        _ => Err(CommandError::InvalidCommand),
    }
}

/// 编码响应
///
/// 将响应对象编码为字节流。
///
/// # 参数
///
/// - `response`: 响应对象
///
/// # 返回值
///
/// 编码后的字节流
pub fn encode_response(response: &Response) -> HeaplessVec<u8, 256> {
    let mut result: HeaplessVec<u8, 256> = HeaplessVec::new();
    
    match response {
        Response::Version { data } => {
            let _ = result.extend_from_slice(data);
        }
        Response::Regs { data } => {
            for &val in data.iter() {
                let _ = result.extend_from_slice(&val.to_le_bytes());
            }
        }
        Response::Status { status } => {
            let _ = result.push(*status as u8);
        }
        Response::FwUpdate { data } => {
            let _ = result.extend_from_slice(data);
        }
        Response::Empty => {}
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_registry() {
        let mut registry: CommandRegistry<16> = CommandRegistry::new();
        
        fn version_handler(_cmd: Command) -> Result<Response, CommandError> {
            let mut data: HeaplessVec<u8, MAX_VERSION_LEN> = HeaplessVec::new();
            let _ = data.extend_from_slice(b"1.0.0");
            Ok(Response::Version { data })
        }
        
        assert!(registry.register(CMD_GET_VERSION, version_handler).is_ok());
        assert_eq!(registry.count, 1);
    }

    #[test]
    fn test_parse_get_version() {
        let cmd = parse_command(CMD_GET_VERSION, &[VER_TYPE_FW]).unwrap();
        match cmd {
            Command::GetVersion { ver_type } => assert_eq!(ver_type, VER_TYPE_FW),
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_regs_write() {
        let data: [u8; 4] = [0x01, 0x00, 0x34, 0x12];
        let cmd = parse_command(CMD_REGS_WRITE, &data).unwrap();
        match cmd {
            Command::RegsWrite { addr, value } => {
                assert_eq!(addr, 0x0001);
                assert_eq!(value, 0x1234);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_chip_ctrl() {
        let cmd = parse_command(CMD_CHIP_CTRL, &[CHIP_CTRL_SOFT_RESET]).unwrap();
        match cmd {
            Command::ChipCtrl { ctrl_type } => assert_eq!(ctrl_type, CHIP_CTRL_SOFT_RESET),
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_sw_function() {
        let data: [u8; 5] = [0x01, 0x00, 0x00, 0x00, 0x00];
        let cmd = parse_command(CMD_SW_FUNCTION, &data).unwrap();
        match cmd {
            Command::SwFunction { func_mode, ctrl_type } => {
                assert_eq!(func_mode, 1);
                assert_eq!(ctrl_type, 0);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_encode_response() {
        let mut data: HeaplessVec<u8, MAX_VERSION_LEN> = HeaplessVec::new();
        let _ = data.extend_from_slice(b"1.0.0");
        
        let response = Response::Version { data };
        let encoded = encode_response(&response);
        
        assert_eq!(&encoded[..5], b"1.0.0");
    }

    #[test]
    fn test_dispatch() {
        let mut registry: CommandRegistry<16> = CommandRegistry::new();
        
        fn version_handler(_cmd: Command) -> Result<Response, CommandError> {
            let mut data: HeaplessVec<u8, MAX_VERSION_LEN> = HeaplessVec::new();
            let _ = data.extend_from_slice(b"1.0.0");
            Ok(Response::Version { data })
        }
        
        registry.register(CMD_GET_VERSION, version_handler).unwrap();
        
        let cmd = Command::GetVersion { ver_type: VER_TYPE_FW };
        let result = registry.dispatch(CMD_GET_VERSION, cmd);
        
        assert!(result.is_ok());
    }
}
