//! GH Protocol RPC Core Library
//! 
//! 提供RPC核心协议实现，支持异步操作、自动分帧、超时重发等功能。

pub mod error;
pub mod types;
pub mod log;
pub mod frame;
pub mod package;
pub mod unpacker;
pub mod core;

pub use error::RpcError;
pub use types::*;
pub use log::{LogCallback, DefaultLogger, NullLogger};
pub use frame::{FrameParser, ParseResult, ParseState, FrameBuilder, calculate_crc};
pub use package::{Package, Unpackage, TypeHeader, FormatInfo};
pub use unpacker::{
    DataUnpacker, UnpackError, UnpackValue, unpack,
    unpack_u8_array, unpack_u16_array, unpack_u32_array, unpack_u64_array,
    unpack_i8_array, unpack_i16_array, unpack_i32_array, unpack_i64_array,
    unpack_string,
};
pub use core::{RpcCore, RpcConfig, InvokeNode, RpcHandler, InvokeContext, SendFunction, SendFuture};
