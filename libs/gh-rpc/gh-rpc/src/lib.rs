//! # gh-rpc
//!
//! GH RPC 业务层库，封装 rpc 核心库并提供业务相关功能。
//!
//! ## 模块结构
//!
//! | 模块 | 说明 |
//! |------|------|
//! | [`rpc`] | Re-export rpc crate 的公共 API |
//! | [`cmd`] | 命令处理模块 |
//! | [`codec`] | 编解码模块 |
//! | [`data_package`] | 数据打包模块 |
//! | [`pool`] | Re-export rpc::poll |
//! | [`types`] | Re-export rpc::types |

#![no_std]
#![warn(missing_docs)]

pub mod rpc;
pub mod cmd;
pub mod codec;
pub mod data_package;
pub mod pool;
pub mod types;
