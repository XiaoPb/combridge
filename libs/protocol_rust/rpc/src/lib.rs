//! GH Protocol RPC Core Library
//!
//! 提供RPC核心协议实现，支持异步操作、自动分帧、超时重发等功能。

pub mod core;
pub mod error;
pub mod frame;
pub mod log;
pub mod package;
pub mod types;
pub mod unpacker;

pub use core::{
    InvokeContext, InvokeNode, RpcConfig, RpcCore, RpcHandler, SendFunction, SendFuture,
};
pub use error::RpcError;
pub use frame::{
    calculate_crc, FrameBuilder, FrameParser, ParseResult, ParseState, RPC_RECEIVE_BUFFER_SIZE,
};
pub use log::{DefaultLogger, LogCallback, NullLogger};
pub use package::{FormatInfo, Package, TypeHeader, Unpackage};
pub use types::*;
pub use unpacker::{
    unpack, unpack_i16_array, unpack_i32_array, unpack_i64_array, unpack_i8_array, unpack_string,
    unpack_u16_array, unpack_u32_array, unpack_u64_array, unpack_u8_array, DataUnpacker,
    UnpackError, UnpackValue,
};
