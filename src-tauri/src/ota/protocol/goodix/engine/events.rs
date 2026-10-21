//! 引擎与调用方之间的进度上报。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use tokio::sync::mpsc;

use super::plan::Route;

/// 会话的高层阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// 建立连接。
    Connect,
    /// 发现服务与协议版本。
    Probe,
    /// 读取芯片信息。
    LoadChipInfo,
    /// 读取 Bootloader 信息。
    LoadBootInfo,
    /// 读取 App Bootloader 附加信息。
    LoadExtraInfo,
    /// 校验布局。
    CheckOverlap,
    /// 切换 DFU 模式。
    SetDfuMode,
    /// 等待设备重启进入 Bootloader。
    Reconnect,
    /// 准备 Flash 传输。
    Prepare,
    /// 擦除。
    Erase,
    /// 传输字节。
    Program,
    /// 校验校验和。
    Verify,
    /// 重启。
    Reboot,
    /// 会话完成。
    Complete,
}

/// 会话运行期间发出的事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent {
    /// 阶段开始。
    Stage(Stage),
    /// 人类可读消息。
    Message(String),
    /// 擦除进度（扇区）。
    Erase {
        /// 已擦除扇区。
        erased: u16,
        /// 总扇区数。
        total: u16,
    },
    /// 数据传输入度（字节）。
    Transfer {
        /// 已写入字节。
        written: u64,
        /// 总字节。
        total: u64,
    },
    /// 最终报告。
    Completed(DfuReport),
}

impl ProgressEvent {
    /// 进度型事件的完成百分比。
    pub fn percent(&self) -> Option<u8> {
        match self {
            Self::Erase { erased, total } if *total > 0 => {
                Some(((u32::from(*erased) * 100) / u32::from(*total)) as u8)
            }
            Self::Transfer { written, total } if *total > 0 => {
                Some(((*written * 100) / *total) as u8)
            }
            _ => None,
        }
    }
}

/// 接收端消失时静默丢弃事件的进度汇。
#[derive(Debug, Clone, Default)]
pub struct ProgressSink {
    sender: Option<mpsc::UnboundedSender<ProgressEvent>>,
}

impl ProgressSink {
    /// 由通道发送端创建汇。
    pub fn new(sender: mpsc::UnboundedSender<ProgressEvent>) -> Self {
        Self {
            sender: Some(sender),
        }
    }

    /// 创建通道并返回接收端与汇。
    pub fn channel() -> (Self, mpsc::UnboundedReceiver<ProgressEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self::new(tx), rx)
    }

    /// 丢弃所有事件的汇。
    pub fn none() -> Self {
        Self { sender: None }
    }

    /// 发出事件。
    pub fn emit(&self, event: ProgressEvent) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(event);
        }
    }

    /// 发出阶段事件。
    pub fn stage(&self, stage: Stage) {
        self.emit(ProgressEvent::Stage(stage));
    }

    /// 发出消息。
    pub fn message(&self, message: impl Into<String>) {
        self.emit(ProgressEvent::Message(message.into()));
    }

    /// 发出擦除进度。
    pub fn erase(&self, erased: u16, total: u16) {
        self.emit(ProgressEvent::Erase { erased, total });
    }

    /// 发出传输进度。
    pub fn transfer(&self, written: u64, total: u64) {
        self.emit(ProgressEvent::Transfer { written, total });
    }
}

/// 已完成会话的摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DfuReport {
    /// 实际使用的路由。
    pub route: Route,
    /// 设备上报的协议版本。
    pub protocol_version: u8,
    /// 已传输字节数。
    pub bytes_transferred: u64,
    /// 写入设备的校验和。
    pub checksum: u32,
}
