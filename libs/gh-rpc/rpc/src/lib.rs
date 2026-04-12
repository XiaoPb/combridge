//! # rpc
//!
//! Goodix RPC 协议核心库，提供嵌入式设备间的远程过程调用功能。
//!
//! ## 特性
//!
//! - **`no_std` 兼容**：适用于嵌入式环境
//! - **帧解析状态机**：逐字节解析，低内存占用
//! - **CRC 校验**：确保数据完整性
//! - **安全帧模式**：支持需要确认的可靠传输
//!
//! ## 帧格式
//!
//! RPC 帧格式如下：
//!
//! ```text
//! +----------+--------+---------+----------+-------+----------+--------+-----+
//! | Header   | Length | TypeKey | KeyData  | ComID | FrameID  | Param  | CRC |
//! | 2 bytes  | 1 byte | 1 byte  | N bytes  | 1 byte| 1 byte   | N bytes|1byte|
//! +----------+--------+---------+----------+-------+----------+--------+-----+
//! ```
//!
//! - **Header**: 帧头标识 `[0xAA, 0x11]`
//! - **Length**: 帧长度（不含帧头和长度字段）
//! - **TypeKey**: 类型键，标识键数据类型和帧属性
//! - **KeyData**: 命令名称
//! - **ComID**: 调用索引（仅安全帧模式）
//! - **FrameID**: 帧索引（多帧传输时使用）
//! - **Param**: 参数数据
//! - **CRC**: 校验和
//!
//! ## 快速开始
//!
//! ### 创建 RPC 核心
//!
//! ```rust
//! use rpc::{RpcCore, RpcConfig, InvokeNode};
//!
//! // 定义发送回调函数
//! fn send_data(data: &[u8]) {
//!     // 实现数据发送逻辑
//! }
//!
//! // 创建 RPC 配置
//! let config = RpcConfig::new(send_data);
//!
//! // 创建 RPC 核心（最多注册 16 个函数）
//! let mut rpc: RpcCore<16, _> = RpcCore::new(config);
//! ```
//!
//! ### 注册处理函数
//!
//! ```rust
//! use rpc::{RpcCore, RpcConfig, InvokeNode};
//!
//! // 定义处理函数
//! fn my_handler(data: &[u8], size: usize, ret: Option<&mut [u8]>) -> i32 {
//!     // 处理接收到的数据
//!     println!("Received {} bytes", size);
//!     0 // 返回状态码
//! }
//!
//! # fn send_data(_data: &[u8]) {}
//! # let config = RpcConfig::new(send_data);
//! # let mut rpc: RpcCore<16, _> = RpcCore::new(config);
//!
//! // 创建并注册节点
//! let node = InvokeNode::new("my_command", Some("<u8><u16>"), Some(my_handler));
//! rpc.register(node).unwrap();
//! ```
//!
//! ### 处理接收数据
//!
//! ```rust
//! use rpc::{RpcCore, RpcConfig};
//!
//! # fn send_data(_data: &[u8]) {}
//! # let config = RpcConfig::new(send_data);
//! # let mut rpc: RpcCore<16, _> = RpcCore::new(config);
//!
//! // 处理接收到的字节数据
//! let received_data: &[u8] = &[0xAA, 0x11, /* ... */];
//! rpc.process(received_data, false);
//! ```
//!
//! ### 发送数据
//!
//! ```rust
//! use rpc::{RpcCore, RpcConfig};
//!
//! # fn send_data(_data: &[u8]) {}
//! # let config = RpcConfig::new(send_data);
//! # let mut rpc: RpcCore<16, _> = RpcCore::new(config);
//!
//! // 发布数据（非阻塞，无需确认）
//! rpc.publish("status", &[1, 2, 3]).unwrap();
//!
//! // 发送数据（需要确认）
//! rpc.send("command", &[4, 5, 6]).unwrap();
//! ```
//!
//! ### 使用帧解析器
//!
//! ```rust
//! use rpc::{FrameParser, ParseState};
//!
//! let mut parser = FrameParser::new();
//!
//! // 逐字节处理数据
//! for byte in &[0xAA, 0x11, /* ... */] {
//!     match parser.process_byte(*byte) {
//!         Ok(Some(result)) => {
//!             // 帧解析完成
//!             println!("Key: {}", result.key_str());
//!         }
//!         Ok(None) => {
//!             // 需要更多数据
//!         }
//!         Err(e) => {
//!             // 解析错误
//!             println!("Error: {:?}", e);
//!         }
//!     }
//! }
//! ```
//!
//! ## 模块
//!
//! - [`types`] - 核心类型定义（TypeKey、错误类型、常量）
//! - [`package`] - 帧解析器（FrameParser、ParseResult）
//! - [`core`] - RPC 核心（RpcCore、RpcConfig、InvokeNode）
//! - [`poll`] - 连接池管理（待实现）
//!
//! ## 导出的核心类型
//!
//! | 类型 | 说明 |
//! |------|------|
//! | [`TypeKey`] | 类型键结构体，标识帧属性 |
//! | [`RpcError`] | RPC 错误类型 |
//! | [`FrameError`] | 帧解析错误类型 |
//! | [`FRAME_HEADER`] | 帧头标识 `[0xAA, 0x11]` |
//! | [`GHRPC_FRAME_SIZE`] | RPC 帧最大大小（256 字节） |
//! | [`MAX_SUPPORT_KEY_SIZE`] | 最大键值大小（64 字节） |
//!
//! ## 导出的解析器类型
//!
//! | 类型 | 说明 |
//! |------|------|
//! | [`FrameParser`] | 帧解析状态机 |
//! | [`ParseResult`] | 解析结果 |
//! | [`ParseState`] | 解析状态枚举 |
//! | [`FrameIndex`] | 帧索引信息 |
//!
//! ## 导出的 RPC 核心类型
//!
//! | 类型 | 说明 |
//! |------|------|
//! | [`RpcCore`] | RPC 核心结构 |
//! | [`RpcConfig`] | RPC 配置结构 |
//! | [`InvokeNode`] | 函数注册节点 |
//! | [`RpcHandler`] | RPC 处理函数类型 |

#![no_std]
#![warn(missing_docs)]
#![warn(unsafe_code)]

pub mod core;
pub mod package;
pub mod poll;
pub mod types;

pub use types::{CodecError, FrameData, FrameError, PackHeader, RpcError, RpcPoint, TypeKey, TypeMarker, FRAME_HEADER, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE};

pub use package::{FrameIndex, FrameParser, ParseResult, ParseState};

pub use core::{DynamicNodeType, FormatToken, FrameBuffer, InvokeNode, RpcConfig, RpcCore, RpcHandler, SecureCallbackType, SecureReturn, UnpackReader, UnpackResult, UnpackValue, unpack, DynamicNode, DynamicNodeState};

pub use poll::{BufferIndex, BufferPool, LinkedBuffer, PoolError, SlabMemory};
