//! # GH Protocol Command Library
//!
//! 提供GH协议命令的高级封装和G协议解析功能。
//!
//! ## 模块结构
//!
//! - [`types`] - G协议数据类型定义（GhFuncFrame、GhFrameData等）
//! - [`commands`] - 命令定义（命令键常量、格式字符串、Command枚举等）
//! - [`frame_decoder`] - G协议帧解码器
//! - [`executor`] - 命令执行器
//!
//! ## 主要类型
//!
//! ### 数据类型
//!
//! - [`GhFuncFrame`] - 功能帧结构，包含帧计数、时间戳、G传感器数据和通道数据
//! - [`GhFrameData`] - 帧数据结构，包含IPD、原始数据和AGC信息
//! - [`GhAgcInfo`] - AGC信息结构
//! - [`GhGsensorData`] - G传感器数据
//! - [`GhFuncFixIdx`] - 功能固定索引枚举
//!
//! ### 命令类型
//!
//! - [`Command`] - 命令枚举，包含所有20个命令类型
//! - [`Response`] - 响应枚举
//! - 命令键常量：`KEY_GH3X_GET_VERSION`、`KEY_GH3X_REGS_WRITE_CMD`等
//! - 格式字符串常量：`FMT_GH3X_GET_VERSION`、`FMT_GH3X_REGS_WRITE_CMD`等
//!
//! ### 解码器
//!
//! - [`FrameDecoder`] - G协议帧解码器
//! - [`DataUnpacker`] - 通用数据解码器（来自rpc模块）
//!
//! ### 执行器
//!
//! - [`CommandExecutor`] - 命令执行器
//! - [`FrameCallback`] - 帧数据回调类型
//!
//! ## 示例
//!
//! ```ignore
//! use gh_rpc::{FrameDecoder, GhFuncFrame, DataUnpacker, unpack_string};
//!
//! // G协议帧解码
//! let decoder = FrameDecoder::new();
//! let frames: Vec<GhFuncFrame> = decoder.decode_frames(&raw_data)?;
//!
//! // 通用数据解码（来自rpc模块）
//! let unpacker = DataUnpacker::new();
//! let value = unpacker.unpack(&data, "<u16>")?;
//!
//! // 便捷函数
//! let version = unpack_string(&response);
//! ```

pub mod types;
pub mod commands;
pub mod frame_decoder;
pub mod executor;

pub use types::{
    DecodeError, GhAgcInfo, GhFrameData, GhFrameDataFlag, GhFuncFixIdx, GhFuncFrame, GhGsensorData,
};
pub use commands::{
    Command, DownloadConfigParams, EventParams, FGetModeParams, FParams, FSetModeParams,
    FwParams, GParams, Gh3xChipCtrlParams, Gh3xGetVersionParams, Gh3xRegBitFieldWriteCmdParams,
    Gh3xRegsBitFieldWriteCmdParams, Gh3xRegsListWriteCmdParams, Gh3xRegsReadCmdParams,
    Gh3xRegsWriteCmdParams, Gh3xSwFunctionCmdParams, GhLowPowerCmdParams, GhSetWorkModeCmdParams,
    GhTimeSetParams, GhTimestampSetParams, GetChipLinkStatusParams, Response,
    FMT_EVENT, FMT_F, FMT_FW, FMT_F_GET_MODE, FMT_F_SET_MODE, FMT_G, FMT_GH3X_CHIP_CTRL,
    FMT_GH3X_GET_VERSION, FMT_GH3X_REG_BIT_FIELD_WRITE_CMD, FMT_GH3X_REGS_BIT_FIELD_WRITE_CMD,
    FMT_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REGS_READ_CMD, FMT_GH3X_REGS_WRITE_CMD,
    FMT_GH3X_SW_FUNCTION_CMD, FMT_GH_SET_WORK_MODE_CMD, FMT_DOWNLOAD_CONFIG,
    FMT_GET_CHIP_LINK_STATUS, FMT_GH_LOW_POWER_CMD, FMT_GH_TIME_SET, FMT_GH_TIMESTAMP_SET,
    KEY_EVENT, KEY_F, KEY_FW, KEY_F_GET_MODE, KEY_F_SET_MODE, KEY_G, KEY_GH3X_CHIP_CTRL,
    KEY_GH3X_GET_VERSION, KEY_GH3X_REG_BIT_FIELD_WRITE_CMD, KEY_GH3X_REGS_BIT_FIELD_WRITE_CMD,
    KEY_GH3X_REGS_LIST_WRITE_CMD, KEY_GH3X_REGS_READ_CMD, KEY_GH3X_REGS_WRITE_CMD,
    KEY_GH3X_SW_FUNCTION_CMD, KEY_GH_SET_WORK_MODE_CMD, KEY_DOWNLOAD_CONFIG,
    KEY_GET_CHIP_LINK_STATUS, KEY_GH_LOW_POWER_CMD, KEY_GH_TIME_SET, KEY_GH_TIMESTAMP_SET,
    RET_F_GET_MODE, RET_FW, RET_GET_CHIP_LINK_STATUS, RET_GH3X_GET_VERSION, RET_GH3X_REGS_READ_CMD,
};
pub use frame_decoder::FrameDecoder;
pub use executor::{CommandExecutor, FrameCallback};

pub use rpc::{
    DataUnpacker, UnpackError, UnpackValue, unpack,
    unpack_u8_array, unpack_u16_array, unpack_u32_array, unpack_u64_array,
    unpack_i8_array, unpack_i16_array, unpack_i32_array, unpack_i64_array,
    unpack_string,
};
