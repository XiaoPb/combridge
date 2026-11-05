//! GH3220 UProtocol 领域模块。
//!
//! - `types`：对外 DTO、配置与错误
//! - `link`：BLE / TCP 调试链路
//! - `session`：命令串行化与响应关联
//! - `manager`：生命周期、解码、事件 ACK、采集与 CSV
//! - `csv_writer`：按功能分文件的 CSV 落盘

pub mod csv_writer;
pub mod link;
pub mod manager;
pub mod session;
pub mod types;

pub use manager::Gh3220Manager;
pub use types::{
    Gh3220CaptureConfig, Gh3220CaptureStatus, Gh3220CsvConfig, Gh3220DataEvent, Gh3220DebugStatus,
    Gh3220DeviceInfo, Gh3220Error, Gh3220EventPayload, Gh3220LinkConfig, Gh3220LinkKind,
    Gh3220Record, Gh3220State, Gh3220Stats, Gh3220Status, DEFAULT_CSV_IDLE_CLOSE_SECS,
};

pub type Gh3220ManagerRef = std::sync::Arc<Gh3220Manager>;
