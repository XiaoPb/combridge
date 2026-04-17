//! GH3036 协议模块
//!
//! 本模块提供 GH3036 芯片协议的 Rust 实现，包括：
//! - 协议管理器
//! - 数据类型定义
//! - CSV 写入器

pub mod csv_writer;
pub mod manager;
pub mod types;

pub use manager::{ChannelConfig, ChannelType, CsvConfig, Gh3036Manager};
pub use types::{get_rpc_commands, get_version_types, Gh3036EventData, Gh3036FrameData, RpcCommand, RpcParam, VersionTypeConfig};

pub type Gh3036ManagerRef = std::sync::Arc<Gh3036Manager>;
