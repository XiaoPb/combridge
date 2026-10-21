//! 会话描述：传输什么、如何传输。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::firmware::DfuImage;

/// 使用哪个协议服务。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Route {
    /// 优先基础 DFU 服务，回退到快速服务。
    #[default]
    Auto,
    /// 基础 DFU 服务（`a6ed0401`），DFU V1 或 V2。
    Normal,
    /// 专用快速 DFU 服务（`A6ED0701`）。
    FastService,
}

/// 固件数据如何推送到基础 DFU 服务。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferMode {
    /// 设备为 DFU V2 时快速模式，否则普通模式。
    #[default]
    Auto,
    /// 始终使用 `PROGRAM_FLASH` 分段。
    Normal,
    /// 始终使用原始快速流（仅 DFU V2）。
    Fast,
}

/// 镜像写入位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TargetAddress {
    /// 写入镜像中记录的加载地址（单 Bank / 原地）。
    #[default]
    LoadAddress,
    /// 写入显式地址（后台 / 拷贝模式）。
    Explicit(u32),
    /// 由 App Bootloader 推荐地址（仅 DFU V2）。
    Recommended,
}

/// 固件与资源更新共用的传输选项。
#[derive(Debug, Clone, Default)]
pub struct TransferOptions {
    /// 服务选择。
    pub route: Route,
    /// 基础服务上的数据推送模式。
    pub mode: TransferMode,
    /// 目标地址选择。
    pub target: TargetAddress,
    /// `PROGRAM_END` 后立即复位并启动新固件。
    pub run_immediately: bool,
    /// DFU V1 前写入控制点的可选厂商命令。
    pub control_command: Option<Vec<u8>>,
}

impl TransferOptions {
    /// 普通原地更新完成后立即运行新固件。
    pub fn in_place() -> Self {
        Self {
            run_immediately: true,
            ..Self::default()
        }
    }
}

/// 原始资源镜像（字体、表、外部 Flash 资源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceImage {
    /// 资源字节。
    pub data: Vec<u8>,
    /// 内部或外部 Flash 的目标地址。
    pub address: u32,
    /// 写入外部 QSPI Flash。
    pub ext_flash: bool,
}

impl ResourceImage {
    /// 构造资源镜像。
    pub fn new(data: Vec<u8>, address: u32, ext_flash: bool) -> Self {
        Self {
            data,
            address,
            ext_flash,
        }
    }

    /// 字节数。
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 是否无数据。
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// 待执行的操作。
#[derive(Debug, Clone)]
pub enum Operation {
    /// 更新应用固件。
    Firmware(DfuImage),
    /// 更新资源镜像。
    Resource(ResourceImage),
}

impl Operation {
    /// 将要传输的载荷长度。
    pub fn data_len(&self) -> usize {
        match self {
            Self::Firmware(image) => image.len(),
            Self::Resource(resource) => resource.len(),
        }
    }
}

/// 完整的 DFU 会话描述。
#[derive(Debug, Clone)]
pub struct DfuPlan {
    /// 传输内容。
    pub operation: Operation,
    /// 传输方式。
    pub options: TransferOptions,
}

impl DfuPlan {
    /// 固件更新计划。
    pub fn firmware(image: DfuImage, options: TransferOptions) -> Self {
        Self {
            operation: Operation::Firmware(image),
            options,
        }
    }

    /// 资源更新计划。
    pub fn resource(resource: ResourceImage, options: TransferOptions) -> Self {
        Self {
            operation: Operation::Resource(resource),
            options,
        }
    }
}
