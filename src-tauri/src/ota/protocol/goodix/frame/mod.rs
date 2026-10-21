//! Goodix DFU 两种协议的帧编解码。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

pub mod basic;
pub mod fast;

pub use basic::{BasicFrame, HEADER_LEN as BASIC_HEADER_LEN, MAGIC as BASIC_MAGIC};
pub use fast::{FastFrame, MAGIC as FAST_MAGIC};
