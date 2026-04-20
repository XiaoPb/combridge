//! # RPC 核心模块
//!
//! 本模块提供 RPC（远程过程调用）核心功能，兼容 C 版本的 `gh_rpccore.c`。
//!
//! ## 主要组件
//!
//! - [`RpcCore`]: RPC 核心结构，管理函数注册和帧处理
//! - [`RpcConfig`]: RPC 配置结构，配置发送回调和同步原语
//! - [`InvokeNode`]: 函数注册节点，存储注册的 RPC 函数信息
//! - [`RpcHandler`]: RPC 处理函数类型
//!
//! ## 使用流程
//!
//! 1. 创建 [`RpcConfig`] 配置发送回调
//! 2. 创建 [`RpcCore`] 实例
//! 3. 使用 [`InvokeNode`] 注册处理函数
//! 4. 调用 `process()` 处理接收数据
//! 5. 调用 `publish()`/`send()`/`call()` 发送数据
//!
//! ## 示例
//!
//! ### 完整示例
//!
//! ```rust
//! use rpc::{InvokeNode, RpcConfig, RpcCore};
//!
//! // 定义发送回调函数
//! fn send_data(data: &[u8]) {
//!     // 实现数据发送逻辑
//! }
//!
//! // 创建 RPC 配置和核心
//! let config = RpcConfig::new(send_data);
//! let mut rpc: RpcCore<16, _> = RpcCore::new(config);
//!
//! // 定义处理函数
//! fn my_handler(data: &[u8], size: usize, ret: Option<&mut [u8]>) -> i32 {
//!     // 处理接收到的数据
//!     0 // 返回状态码
//! }
//!
//! // 注册处理函数
//! let node = InvokeNode::new("my_command", Some("<u8><u16>"), Some(my_handler));
//! rpc.register(node).unwrap();
//!
//! // 发送数据
//! rpc.publish("status", &[1, 2, 3]).unwrap();
//! ```

use crate::package::{FrameParser, ParseResult};
use crate::types::{RpcError, TypeKey, FRAME_HEADER, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE};
use heapless::{String, Vec as HeaplessVec};

#[allow(dead_code)]
const MAX_STATIC_NODES: usize = 16;
const MAX_DYNAMIC_NODES: usize = 3;
const COMM_RETRY_TIME: usize = 500;
const COMM_RETRY_ROUND: usize = 100;

/// 默认最大 payload 大小
pub const DEFAULT_MAX_PAYLOAD_SIZE: usize = 200;
/// 多帧缓冲区大小
pub const MULTI_FRAME_BUFFER_SIZE: usize = 4096;
const MAX_FRAME_BUFFERS: usize = 8;

/// 安全帧回调类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureCallbackType {
    /// 接收帧确认
    ReceiveFrame = 0,
    /// 调用返回
    Return = 1,
    /// 函数不存在
    NoSuchFunction = 2,
    /// 错误
    Error = 3,
}

/// 安全帧返回数据
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecureReturn {
    /// 回调类型
    pub callback_type: SecureCallbackType,
    /// 通信 ID
    pub com_id: u8,
    /// 数据 1（帧索引等）
    pub data1: u8,
    /// 数据 2
    pub data2: u8,
}

impl Default for SecureReturn {
    fn default() -> Self {
        Self {
            callback_type: SecureCallbackType::ReceiveFrame,
            com_id: 0,
            data1: 0,
            data2: 0,
        }
    }
}

/// 帧缓冲区，用于安全帧模式的多帧传输
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    /// 帧索引
    pub frame_idx: u8,
    /// 帧数据
    pub data: HeaplessVec<u8, GHRPC_FRAME_SIZE>,
    /// 链表下一个节点索引
    pub next: Option<usize>,
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self {
            frame_idx: 0,
            data: HeaplessVec::new(),
            next: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 动态节点状态
pub enum DynamicNodeState {
    /// 等待响应
    Waiting,
    /// 已完成
    Completed,
    /// 超时
    Timeout,
}

/// 动态节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicNodeType {
    /// 普通帧模式
    Normal,
    /// 安全帧模式
    Secure,
}

/// 动态节点，用于管理安全帧模式的请求-响应生命周期
#[derive(Debug, Clone)]
pub struct DynamicNode {
    /// 命令键字节数据
    pub key: [u8; MAX_SUPPORT_KEY_SIZE],
    /// 命令键有效长度
    pub key_len: usize,
    /// 通信 ID
    pub com_id: u8,
    /// 调用索引
    pub invoke_idx: u8,
    /// 节点当前状态
    pub state: DynamicNodeState,
    /// 节点类型（普通/安全）
    pub node_type: DynamicNodeType,
    /// 返回数据缓冲区
    pub ret_data: HeaplessVec<u8, GHRPC_FRAME_SIZE>,
    /// 帧缓冲区数组
    pub frame_buffers: [Option<FrameBuffer>; MAX_FRAME_BUFFERS],
    /// 帧缓冲区链表头索引
    pub frame_buffer_head: Option<usize>,
    /// 帧缓冲区链表尾索引
    pub frame_buffer_tail: Option<usize>,
    /// 帧缓冲区计数
    pub frame_buffer_count: usize,
}

impl Default for DynamicNode {
    fn default() -> Self {
        Self {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 0,
            com_id: 0,
            invoke_idx: 0,
            state: DynamicNodeState::Waiting,
            node_type: DynamicNodeType::Normal,
            ret_data: HeaplessVec::new(),
            frame_buffers: core::array::from_fn(|_| None),
            frame_buffer_head: None,
            frame_buffer_tail: None,
            frame_buffer_count: 0,
        }
    }
}

impl DynamicNode {
    /// 创建新的动态节点
    pub fn new(key: &str, node_type: DynamicNodeType) -> Self {
        let mut node = Self::default();
        node.key_len = key.len().min(MAX_SUPPORT_KEY_SIZE);
        node.key[..node.key_len].copy_from_slice(&key.as_bytes()[..node.key_len]);
        node.node_type = node_type;
        node
    }

    /// 获取命令键的字符串表示
    pub fn key_str(&self) -> &str {
        core::str::from_utf8(&self.key[..self.key_len]).unwrap_or("")
    }

    /// 添加帧缓冲区到链表
    pub fn add_frame_buffer(&mut self, frame_idx: u8, frame_data: HeaplessVec<u8, GHRPC_FRAME_SIZE>) -> Result<(), RpcError> {
        if self.frame_buffer_count >= MAX_FRAME_BUFFERS {
            return Err(RpcError::MemoryNotEnough);
        }

        let free_slot = self.frame_buffers.iter().position(|b| b.is_none())
            .ok_or(RpcError::MemoryNotEnough)?;

        let buffer = FrameBuffer {
            frame_idx,
            data: frame_data,
            next: None,
        };

        if let Some(tail_idx) = self.frame_buffer_tail {
            if let Some(ref mut tail) = self.frame_buffers[tail_idx] {
                tail.next = Some(free_slot);
            }
        }

        self.frame_buffers[free_slot] = Some(buffer);
        
        if self.frame_buffer_head.is_none() {
            self.frame_buffer_head = Some(free_slot);
        }
        self.frame_buffer_tail = Some(free_slot);
        self.frame_buffer_count += 1;

        Ok(())
    }

    /// 从链表中移除指定帧索引的缓冲区
    pub fn remove_frame_buffer(&mut self, frame_idx: u8) -> bool {
        let mut prev_idx: Option<usize> = None;
        let mut current_idx = self.frame_buffer_head;

        while let Some(idx) = current_idx {
            if let Some(ref buffer) = self.frame_buffers[idx] {
                if buffer.frame_idx == frame_idx {
                    let next_idx = buffer.next;
                    
                    if let Some(prev) = prev_idx {
                        if let Some(ref mut prev_buffer) = self.frame_buffers[prev] {
                            prev_buffer.next = next_idx;
                        }
                    } else {
                        self.frame_buffer_head = next_idx;
                    }

                    if next_idx.is_none() {
                        self.frame_buffer_tail = prev_idx;
                    }

                    self.frame_buffers[idx] = None;
                    self.frame_buffer_count = self.frame_buffer_count.saturating_sub(1);
                    return true;
                }
                prev_idx = Some(idx);
                current_idx = buffer.next;
            } else {
                break;
            }
        }
        false
    }

    /// 移除所有帧缓冲区
    pub fn remove_all_frame_buffers(&mut self) {
        for i in 0..MAX_FRAME_BUFFERS {
            self.frame_buffers[i] = None;
        }
        self.frame_buffer_head = None;
        self.frame_buffer_tail = None;
        self.frame_buffer_count = 0;
    }

    /// 检查是否有待重传的帧
    pub fn has_pending_frames(&self) -> bool {
        self.frame_buffer_count > 0
    }

    /// 获取所有待重传的帧缓冲区
    pub fn get_frame_buffers_for_retry(&self) -> HeaplessVec<(usize, HeaplessVec<u8, GHRPC_FRAME_SIZE>), MAX_FRAME_BUFFERS> {
        let mut result = HeaplessVec::new();
        let mut current_idx = self.frame_buffer_head;

        while let Some(idx) = current_idx {
            if let Some(ref buffer) = self.frame_buffers[idx] {
                let _ = result.push((idx, buffer.data.clone()));
                current_idx = buffer.next;
            } else {
                break;
            }
        }

        result
    }
}

/// 多帧传输上下文
///
/// 用于管理大数据分帧发送的状态。
///
/// # 字段说明
///
/// - `key`: 命令键
/// - `key_len`: 命令键长度
/// - `invoke_idx`: 调用索引（安全帧模式）
/// - `frame_idx`: 当前帧索引
/// - `secure`: 是否为安全帧
/// - `max_payload_size`: 最大 payload 大小
#[derive(Debug, Clone)]
pub struct MultiFrameContext {
    /// 命令键
    pub key: [u8; MAX_SUPPORT_KEY_SIZE],
    /// 命令键长度
    pub key_len: usize,
    /// 调用索引（安全帧模式）
    pub invoke_idx: u8,
    /// 当前帧索引
    pub frame_idx: u8,
    /// 是否为安全帧
    pub secure: bool,
    /// 最大 payload 大小
    pub max_payload_size: usize,
}

impl Default for MultiFrameContext {
    fn default() -> Self {
        Self {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 0,
            invoke_idx: 0,
            frame_idx: 0,
            secure: false,
            max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
        }
    }
}

impl MultiFrameContext {
    /// 创建新的多帧上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置命令键
    pub fn set_key(&mut self, key: &str) {
        self.key_len = key.len().min(MAX_SUPPORT_KEY_SIZE);
        self.key[..self.key_len].copy_from_slice(&key.as_bytes()[..self.key_len]);
    }

    /// 获取命令键字符串
    pub fn key_str(&self) -> &str {
        core::str::from_utf8(&self.key[..self.key_len]).unwrap_or("")
    }
}

/// 多帧重组状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiFrameState {
    /// 空闲，未开始接收
    Idle,
    /// 正在接收
    Receiving,
    /// 接收完成
    Complete,
    /// 接收错误
    Error,
}

/// 多帧重组缓冲区
///
/// 用于接收端重组多帧数据。
///
/// # 字段说明
///
/// - `key`: 命令键
/// - `key_len`: 命令键长度
/// - `invoke_idx`: 调用索引（安全帧模式）
/// - `expected_frame_idx`: 期望的下一帧索引
/// - `state`: 重组状态
/// - `data`: 重组后的数据
#[derive(Debug, Clone)]
pub struct MultiFrameBuffer {
    /// 命令键
    pub key: [u8; MAX_SUPPORT_KEY_SIZE],
    /// 命令键长度
    pub key_len: usize,
    /// 调用索引（安全帧模式）
    pub invoke_idx: u8,
    /// 期望的下一帧索引
    pub expected_frame_idx: u8,
    /// 重组状态
    pub state: MultiFrameState,
    /// 重组后的数据
    pub data: HeaplessVec<u8, MULTI_FRAME_BUFFER_SIZE>,
}

impl Default for MultiFrameBuffer {
    fn default() -> Self {
        Self {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 0,
            invoke_idx: 0,
            expected_frame_idx: 0,
            state: MultiFrameState::Idle,
            data: HeaplessVec::new(),
        }
    }
}

impl MultiFrameBuffer {
    /// 创建新的多帧重组缓冲区
    pub fn new() -> Self {
        Self::default()
    }

    /// 重置缓冲区
    pub fn reset(&mut self) {
        self.key = [0u8; MAX_SUPPORT_KEY_SIZE];
        self.key_len = 0;
        self.invoke_idx = 0;
        self.expected_frame_idx = 0;
        self.state = MultiFrameState::Idle;
        self.data.clear();
    }

    /// 获取命令键字符串
    pub fn key_str(&self) -> &str {
        core::str::from_utf8(&self.key[..self.key_len]).unwrap_or("")
    }

    /// 追加帧数据
    ///
    /// # 参数
    ///
    /// - `result`: 解析结果
    ///
    /// # 返回值
    ///
    /// - `Ok(true)`: 重组完成
    /// - `Ok(false)`: 需要更多帧
    /// - `Err(_)`: 重组错误
    pub fn append_frame(&mut self, result: &ParseResult) -> Result<bool, RpcError> {
        if self.state == MultiFrameState::Idle {
            self.key[..result.key.len()].copy_from_slice(&result.key);
            self.key_len = result.key.len();
            self.invoke_idx = result.frame_index.invoke_idx;
            self.expected_frame_idx = result.frame_index.frame_idx;
            self.state = MultiFrameState::Receiving;
        } else {
            if result.key[..] != self.key[..] {
                self.state = MultiFrameState::Error;
                return Err(RpcError::LoseFrame);
            }
            if result.frame_index.frame_idx != self.expected_frame_idx {
                self.state = MultiFrameState::Error;
                return Err(RpcError::LoseFrame);
            }
        }

        self.data
            .extend_from_slice(&result.param)
            .map_err(|_| RpcError::MemoryNotEnough)?;

        if result.is_fin {
            self.state = MultiFrameState::Complete;
            return Ok(true);
        }

        self.expected_frame_idx = self.expected_frame_idx.wrapping_add(1);
        Ok(false)
    }
}

/// RPC 处理函数类型
///
/// 定义 RPC 函数的处理签名。
///
/// # 参数
///
/// - `&[u8]`: 接收到的参数数据
/// - `usize`: 数据大小（字节）
/// - `Option<&mut [u8]>`: 返回数据缓冲区（可选）
///
/// # 返回值
///
/// 处理结果状态码：
/// - `0`: 成功
/// - 负值: 错误
///
/// # 示例
///
/// ```rust
/// use rpc::RpcHandler;
///
/// fn my_handler(data: &[u8], size: usize, ret: Option<&mut [u8]>) -> i32 {
///     // 处理数据
///     if let Some(buf) = ret {
///         // 写入返回数据
///         buf[0] = 0x00;
///     }
///     0 // 返回成功
/// }
/// ```
pub type RpcHandler = fn(&[u8], usize, Option<&mut [u8]>) -> i32;

/// 函数注册节点
///
/// 用于存储注册的 RPC 函数信息。
///
/// # 字段说明
///
/// - `key`: 函数名称（命令键），用于匹配接收到的帧
/// - `detail`: 格式说明，描述参数类型（如 `"<u8><u16*>"`）
/// - `func`: 处理函数
/// - `header`: 头部偏移（内部使用）
///
/// # 示例
///
/// ```rust
/// use rpc::InvokeNode;
///
/// // 定义处理函数
/// fn my_handler(data: &[u8], size: usize, ret: Option<&mut [u8]>) -> i32 {
///     0
/// }
///
/// // 创建注册节点
/// let node = InvokeNode::new(
///     "get_status",           // 命令名称
///     Some("<u8><u16*>"),     // 格式说明
///     Some(my_handler),       // 处理函数
/// );
///
/// assert_eq!(node.key, "get_status");
/// assert_eq!(node.detail, Some("<u8><u16*>"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct InvokeNode {
    /// 函数名称（键）
    ///
    /// 用于匹配接收到的 RPC 帧。当帧的键与此值匹配时，
    /// 将调用对应的处理函数。
    pub key: &'static str,

    /// 格式说明
    ///
    /// 描述参数的类型和格式，如：
    /// - `"<u8>"`: 单个无符号 8 位整数
    /// - `"<u16*>"`: 无符号 16 位整数数组
    /// - `"<u8><u16>"`: 一个 u8 后跟一个 u16
    pub detail: Option<&'static str>,

    /// 处理函数
    ///
    /// 当匹配到此节点时调用的函数。
    /// 如果为 `None`，则此节点仅用于声明，不执行任何操作。
    pub func: Option<RpcHandler>,

    /// 头部偏移
    ///
    /// 内部使用的偏移值。
    pub header: Option<usize>,
}

impl InvokeNode {
    /// 创建新的注册节点
    ///
    /// # 参数
    ///
    /// - `key`: 函数名称（必须是 `'static` 生命周期）
    /// - `detail`: 格式说明（可选）
    /// - `func`: 处理函数（可选）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::InvokeNode;
    ///
    /// fn handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 { 0 }
    ///
    /// // 创建带处理函数的节点
    /// let node = InvokeNode::new("command", Some("<u8>"), Some(handler));
    ///
    /// // 创建仅声明的节点
    /// let node = InvokeNode::new("notification", None, None);
    /// ```
    pub const fn new(key: &'static str, detail: Option<&'static str>, func: Option<RpcHandler>) -> Self {
        Self {
            key,
            detail,
            func,
            header: None,
        }
    }
}

/// RPC 配置结构
///
/// 用于配置 RPC 核心的行为。
///
/// # 字段说明
///
/// - `send`: 发送数据的回调函数（必需）
/// - `lock`: 获取锁的回调函数（可选，用于多线程环境）
/// - `unlock`: 释放锁的回调函数（可选，用于多线程环境）
/// - `delay`: 延时的回调函数（可选，用于重试机制）
///
/// # 示例
///
/// ```rust
/// use rpc::RpcConfig;
///
/// // 基本配置
/// let config = RpcConfig::new(|data: &[u8]| {
///     // 发送数据到串口或其他接口
/// });
///
/// // 带同步原语的配置
/// let config = RpcConfig::new(|data: &[u8]| {
///     // 发送数据
/// })
/// .with_lock(|| { /* 获取锁 */ })
/// .with_unlock(|| { /* 释放锁 */ });
/// ```
pub struct RpcConfig<F>
where
    F: Fn(&[u8]),
{
    /// 发送数据的回调函数
    ///
    /// 当需要发送 RPC 帧时调用此函数。
    /// 参数是要发送的字节数据。
    pub send: F,

    /// 获取锁的回调函数
    ///
    /// 在多线程环境中，用于保护共享资源。
    pub lock: Option<fn()>,

    /// 释放锁的回调函数
    ///
    /// 在多线程环境中，用于释放锁。
    pub unlock: Option<fn()>,

    /// 延时的回调函数
    ///
    /// 用于重试机制中的延时等待。
    pub delay: Option<fn()>,
}

impl<F: Fn(&[u8])> RpcConfig<F> {
    /// 创建新的 RPC 配置
    ///
    /// # 参数
    ///
    /// - `send`: 发送数据的回调函数
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::RpcConfig;
    ///
    /// let config = RpcConfig::new(|data: &[u8]| {
    ///     // 发送数据
    /// });
    /// ```
    pub fn new(send: F) -> Self {
        Self {
            send,
            lock: None,
            unlock: None,
            delay: None,
        }
    }

    /// 设置获取锁的回调函数
    ///
    /// # 参数
    ///
    /// - `lock`: 获取锁的函数
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::RpcConfig;
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {})
    ///     .with_lock(|| { /* 获取锁 */ });
    /// ```
    pub fn with_lock(mut self, lock: fn()) -> Self {
        self.lock = Some(lock);
        self
    }

    /// 设置释放锁的回调函数
    ///
    /// # 参数
    ///
    /// - `unlock`: 释放锁的函数
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::RpcConfig;
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {})
    ///     .with_unlock(|| { /* 释放锁 */ });
    /// ```
    pub fn with_unlock(mut self, unlock: fn()) -> Self {
        self.unlock = Some(unlock);
        self
    }

    /// 设置延时的回调函数
    ///
    /// # 参数
    ///
    /// - `delay`: 延时函数
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::RpcConfig;
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {})
    ///     .with_delay(|| { /* 延时 */ });
    /// ```
    pub fn with_delay(mut self, delay: fn()) -> Self {
        self.delay = Some(delay);
        self
    }
}

/// RPC 核心
///
/// 管理 RPC 函数注册、帧解析和数据收发。
///
/// # 类型参数
///
/// - `N`: 最大注册函数数量
/// - `F`: 发送回调函数类型
///
/// # 功能
///
/// - 函数注册和管理
/// - 帧解析和分发
/// - 数据发送（publish/send/call）
/// - 返回结果处理
///
/// # 示例
///
/// ```rust
/// use rpc::{RpcCore, RpcConfig, InvokeNode};
///
/// // 创建配置
/// let config = RpcConfig::new(|data: &[u8]| {
///     // 发送数据
/// });
///
/// // 创建 RPC 核心（最多注册 16 个函数）
/// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
///
/// // 注册函数
/// fn handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 { 0 }
/// let node = InvokeNode::new("test", None, Some(handler));
/// rpc.register(node).unwrap();
///
/// // 处理接收数据
/// rpc.process(&[0xAA, 0x11, /* ... */], false);
///
/// // 发送数据
/// rpc.publish("status", &[1, 2, 3]).unwrap();
/// ```
pub struct RpcCore<const N: usize, F>
where
    F: Fn(&[u8]),
{
    static_nodes: [Option<InvokeNode>; N],
    static_count: usize,
    dynamic_nodes: [Option<DynamicNode>; MAX_DYNAMIC_NODES],
    parser: FrameParser,
    send: F,
    lock: Option<fn()>,
    unlock: Option<fn()>,
    delay: Option<fn()>,
    send_index: u8,
    current_key: [u8; MAX_SUPPORT_KEY_SIZE],
    current_key_len: usize,
    current_com_id: u8,
    multi_frame_buffer: MultiFrameBuffer,
}

impl<const N: usize, F> RpcCore<N, F>
where
    F: Fn(&[u8]),
{
    /// 创建新的 RPC 核心
    ///
    /// # 参数
    ///
    /// - `config`: RPC 配置
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let rpc: RpcCore<16, _> = RpcCore::new(config);
    /// ```
    pub fn new(config: RpcConfig<F>) -> Self {
        Self {
            static_nodes: [None; N],
            static_count: 0,
            dynamic_nodes: core::array::from_fn(|_| None),
            parser: FrameParser::new(),
            send: config.send,
            lock: config.lock,
            unlock: config.unlock,
            delay: config.delay,
            send_index: 0,
            current_key: [0u8; MAX_SUPPORT_KEY_SIZE],
            current_key_len: 0,
            current_com_id: 0,
            multi_frame_buffer: MultiFrameBuffer::new(),
        }
    }

    /// 注册函数节点
    ///
    /// # 参数
    ///
    /// - `node`: 函数注册节点
    ///
    /// # 错误
    ///
    /// - `RpcError::MemoryNotEnough`: 注册表已满
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig, InvokeNode};
    ///
    /// fn handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 { 0 }
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// let node = InvokeNode::new("test", None, Some(handler));
    /// assert!(rpc.register(node).is_ok());
    /// ```
    pub fn register(&mut self, node: InvokeNode) -> Result<(), RpcError> {
        if self.static_count >= N {
            return Err(RpcError::MemoryNotEnough);
        }
        self.static_nodes[self.static_count] = Some(node);
        self.static_count += 1;
        Ok(())
    }

    /// 处理接收到的数据
    ///
    /// 逐字节解析接收到的数据，当解析完成时调用对应的处理函数。
    ///
    /// # 参数
    ///
    /// - `data`: 接收到的字节数据
    /// - `restart`: 是否重置解析器状态
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// // 处理接收数据（不重置解析器）
    /// rpc.process(&[0xAA, 0x11, /* ... */], false);
    ///
    /// // 处理接收数据（重置解析器）
    /// rpc.process(&[0xAA, 0x11, /* ... */], true);
    /// ```
    pub fn process(&mut self, data: &[u8], restart: bool) {
        log::debug!("[RPC] process() 开始处理数据: len={}, restart={}", data.len(), restart);
        if restart {
            self.parser.reset();
        }
        
        for &byte in data {
            match self.parser.process_byte(byte) {
                Ok(Some(result)) => {
                    log::debug!("[RPC] process() 解析到完整帧: key={}, is_secure={}, is_fin={}", 
                        result.key_str(), result.is_secure, result.is_fin);
                    self.handle_parse_result(result);
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("[RPC] process() 解析错误: {:?}", e);
                    self.parser.reset();
                }
            }
        }
        log::debug!("[RPC] process() 处理完成");
    }

    /// 处理接收到的数据，支持多帧重组
    ///
    /// 与 `process()` 类似，但会自动处理多帧传输，当收到完整的多帧数据时返回重组后的数据。
    ///
    /// # 参数
    ///
    /// - `data`: 接收到的字节数据
    /// - `restart`: 是否重置解析器状态
    ///
    /// # 返回值
    ///
    /// 如果收到完整的多帧数据（fin=1），返回重组后的数据。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// if let Some(data) = rpc.process_multi_frame(&[0xAA, 0x11, /* ... */], false) {
    ///     println!("Received {} bytes", data.len());
    /// }
    /// ```
    pub fn process_multi_frame(&mut self, data: &[u8], restart: bool) -> Option<HeaplessVec<u8, MULTI_FRAME_BUFFER_SIZE>> {
        if restart {
            self.parser.reset();
            self.multi_frame_buffer.reset();
        }
        
        for &byte in data {
            match self.parser.process_byte(byte) {
                Ok(Some(result)) => {
                    if !result.is_fin {
                        match self.multi_frame_buffer.append_frame(&result) {
                            Ok(true) => {
                                let complete_data = self.multi_frame_buffer.data.clone();
                                self.multi_frame_buffer.reset();
                                return Some(complete_data);
                            }
                            Ok(false) => {}
                            Err(_) => {
                                self.multi_frame_buffer.reset();
                            }
                        }
                    } else {
                        if self.multi_frame_buffer.state == MultiFrameState::Receiving {
                            match self.multi_frame_buffer.append_frame(&result) {
                                Ok(true) => {
                                    let complete_data = self.multi_frame_buffer.data.clone();
                                    self.multi_frame_buffer.reset();
                                    return Some(complete_data);
                                }
                                Ok(false) => {}
                                Err(_) => {
                                    self.multi_frame_buffer.reset();
                                }
                            }
                        } else {
                            self.handle_parse_result(result);
                        }
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    self.parser.reset();
                }
            }
        }
        
        None
    }

    /// 获取多帧缓冲区状态
    ///
    /// # 返回值
    ///
    /// 返回当前多帧缓冲区的状态。
    pub fn multi_frame_state(&self) -> MultiFrameState {
        self.multi_frame_buffer.state
    }

    /// 获取多帧缓冲区中已接收的数据
    ///
    /// # 返回值
    ///
    /// 返回已接收的数据切片。
    pub fn multi_frame_data(&self) -> &[u8] {
        &self.multi_frame_buffer.data
    }

    /// 重置多帧缓冲区
    ///
    /// 清空多帧缓冲区，准备接收新的多帧数据。
    pub fn reset_multi_frame(&mut self) {
        self.multi_frame_buffer.reset();
    }

    /// 处理接收到的数据，返回解析到的参数数据
    ///
    /// 与 `process()` 类似，但会返回最后一个完整帧的参数数据。
    ///
    /// # 参数
    ///
    /// - `data`: 接收到的字节数据
    /// - `restart`: 是否重置解析器状态
    ///
    /// # 返回值
    ///
    /// 如果解析到完整的帧且为最后一帧，返回参数数据。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// if let Some(param) = rpc.process_and_get_param(&[0xAA, 0x11, /* ... */], false) {
    ///     println!("Received {} bytes", param.len());
    /// }
    /// ```
    pub fn process_and_get_param(&mut self, data: &[u8], restart: bool) -> Option<HeaplessVec<u8, GHRPC_FRAME_SIZE>> {
        if restart {
            self.parser.reset();
        }
        
        let mut result_param: Option<HeaplessVec<u8, GHRPC_FRAME_SIZE>> = None;
        
        for &byte in data {
            match self.parser.process_byte(byte) {
                Ok(Some(result)) => {
                    if result.is_fin {
                        result_param = Some(result.param.clone());
                    }
                    self.handle_parse_result(result);
                }
                Ok(None) => {}
                Err(_) => {
                    self.parser.reset();
                }
            }
        }
        
        result_param
    }

    fn handle_parse_result(&mut self, result: ParseResult) {
        let key_str = result.key_str();
        log::debug!("[RPC] handle_parse_result() 开始: key={}, is_secure={}, param_len={}", 
            key_str, result.is_secure, result.param.len());
        
        if self.process_ack_frame(&result) {
            log::debug!("[RPC] handle_parse_result() 已处理ACK帧");
            return;
        }
        
        if result.is_secure {
            log::debug!("[RPC] handle_parse_result() 发送ACK: invoke_idx={}, frame_idx={}", 
                result.frame_index.invoke_idx, result.frame_index.frame_idx);
            let _ = self.send_ack(result.frame_index.invoke_idx, result.frame_index.frame_idx);
        }
        
        let param_slice = result.param.as_slice();
        
        let is_dynamic = self.has_dynamic_node(key_str);
        let is_secure = result.is_secure;
        
        log::debug!("[RPC] handle_parse_result() 检查动态节点: key={}, is_dynamic={}", key_str, is_dynamic);
        
        if is_dynamic {
            log::info!("[RPC] handle_parse_result() 匹配到动态节点: key={}", key_str);
            if is_secure {
                let secure_return = SecureReturn {
                    callback_type: SecureCallbackType::Return,
                    com_id: result.frame_index.invoke_idx,
                    data1: result.frame_index.frame_idx,
                    data2: 0,
                };
                log::debug!("[RPC] handle_parse_result() 处理安全帧回调: com_id={}", secure_return.com_id);
                self.handle_secure_callback(&secure_return);
            }
            
            if let Some(dynamic_node) = self.find_dynamic_node_mut(key_str) {
                dynamic_node.ret_data.clear();
                let _ = dynamic_node.ret_data.extend_from_slice(param_slice);
                dynamic_node.state = DynamicNodeState::Completed;
                log::info!("[RPC] handle_parse_result() 动态节点状态更新为Completed: key={}, ret_len={}", 
                    key_str, dynamic_node.ret_data.len());
            }
            return;
        }
        
        let node_info = self.find_node(key_str).map(|n| (n.func, n.detail));
        
        if let Some((Some(func), detail_opt)) = node_info {
            log::debug!("[RPC] handle_parse_result() 匹配到静态节点: key={}", key_str);
            
            self.current_key_len = key_str.len();
            self.current_key[..self.current_key_len].copy_from_slice(key_str.as_bytes());
            self.current_com_id = result.frame_index.invoke_idx;
            
            if let Some(detail) = detail_opt {
                if let Ok(unpack_result) = unpack(param_slice, detail) {
                    if let Some(arr) = unpack_result.get_u8_array(0) {
                        log::debug!("[RPC] handle_parse_result() 解包成功: key={}, unpacked_len={}", key_str, arr.len());
                        func(arr, arr.len(), None);
                        return;
                    }
                }
                log::warn!("[RPC] handle_parse_result() 解包失败，使用原始数据: key={}", key_str);
            }
            
            log::debug!("[RPC] handle_parse_result() 调用处理函数: key={}", key_str);
            func(param_slice, param_slice.len(), None);
        } else {
            log::debug!("[RPC] handle_parse_result() 未找到匹配节点: key={}", key_str);
        }
    }

    fn find_node(&self, key: &str) -> Option<&InvokeNode> {
        for i in 0..self.static_count {
            if let Some(ref node) = self.static_nodes[i] {
                if node.key == key {
                    return Some(node);
                }
            }
        }
        None
    }

    /// 构建并返回帧数据（非阻塞发送）
    ///
    /// 用于发布数据，不需要接收方确认。适用于状态通知等场景。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    /// - `data`: 要发送的数据
    ///
    /// # 返回值
    ///
    /// 返回构建的帧数据。
    ///
    /// # 错误
    ///
    /// - `RpcError::MemoryNotEnough`: 缓冲区不足
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// // 发布状态数据
    /// let frame = rpc.publish("status", &[1, 2, 3]).unwrap();
    /// println!("Sent {} bytes", frame.len());
    /// ```
    pub fn publish(&mut self, key: &str, data: &[u8]) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::debug!("[RPC] publish() 开始: key={}, data_len={}", key, data.len());
        let frame = self.build_frame(key, data, false, true)?;
        log::debug!("[RPC] publish() 构建帧完成, len={}", frame.len());
        (self.send)(&frame);
        log::info!("[RPC] publish() 发送完成: key={}", key);
        Ok(frame)
    }

    /// 构建并返回帧数据（可靠传输）
    ///
    /// 用于需要确认的命令，帧会被标记为安全帧。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    /// - `data`: 要发送的数据
    ///
    /// # 返回值
    ///
    /// 返回构建的帧数据。
    ///
    /// # 错误
    ///
    /// - `RpcError::MemoryNotEnough`: 缓冲区不足
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// // 发送需要确认的命令
    /// let frame = rpc.send("command", &[4, 5, 6]).unwrap();
    /// ```
    pub fn send(&mut self, key: &str, data: &[u8]) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::debug!("[RPC] send() 开始: key={}, data_len={}", key, data.len());
        let frame = self.build_frame(key, data, true, true)?;
        log::debug!("[RPC] send() 构建安全帧完成, len={}", frame.len());
        (self.send)(&frame);
        log::info!("[RPC] send() 发送完成: key={}", key);
        Ok(frame)
    }

    /// 计算指定键的最大 payload 大小
    ///
    /// 根据键长度和帧模式计算单帧可承载的最大数据量。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键
    /// - `secure`: 是否为安全帧
    /// - `is_fin`: 是否为最后一帧
    ///
    /// # 返回值
    ///
    /// 返回最大 payload 大小。
    pub fn calculate_max_payload(&self, key: &str, secure: bool, is_fin: bool) -> usize {
        let key_len = key.len();
        let mut overhead = FRAME_HEADER.len() + 1 + 1;
        
        if key_len != 1 {
            overhead += 1;
        }
        overhead += key_len;
        
        if secure {
            overhead += 1;
        }
        
        if !is_fin {
            overhead += 1;
        }
        
        overhead += 1;
        
        GHRPC_FRAME_SIZE.saturating_sub(overhead)
    }

    /// 多帧发布数据（非阻塞发送）
    ///
    /// 用于发布大数据，自动分帧发送。适用于大数据传输场景。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    /// - `data`: 要发送的数据
    ///
    /// # 返回值
    ///
    /// 返回发送的帧数量。
    ///
    /// # 错误
    ///
    /// - `RpcError::MemoryNotEnough`: 缓冲区不足
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// // 发布大数据（自动分帧）
    /// let large_data = [0u8; 500];
    /// let frame_count = rpc.publish_multi("data", &large_data).unwrap();
    /// println!("Sent {} frames", frame_count);
    /// ```
    pub fn publish_multi(&mut self, key: &str, data: &[u8]) -> Result<usize, RpcError> {
        let max_payload = self.calculate_max_payload(key, false, false);
        
        if data.len() <= max_payload {
            self.publish(key, data)?;
            return Ok(1);
        }
        
        let mut offset = 0;
        let mut frame_count = 0;
        let mut frame_idx: u8 = 0;
        
        while offset < data.len() {
            let remaining = data.len() - offset;
            let is_fin = remaining <= max_payload;
            let chunk_size = if is_fin { remaining } else { max_payload };
            
            let frame = self.build_frame_multi(key, &data[offset..offset + chunk_size], false, is_fin, frame_idx)?;
            (self.send)(&frame);
            
            frame_count += 1;
            offset += chunk_size;
            
            if !is_fin {
                frame_idx = frame_idx.wrapping_add(1);
            }
        }
        
        Ok(frame_count)
    }

    /// 多帧安全发送（可靠传输）
    ///
    /// 用于发送大数据，自动分帧发送，每帧标记为安全帧。
    /// 适用于需要确认的大数据传输场景。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    /// - `data`: 要发送的数据
    ///
    /// # 返回值
    ///
    /// 返回发送的帧数量。
    ///
    /// # 错误
    ///
    /// - `RpcError::MemoryNotEnough`: 缓冲区不足
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// // 安全发送大数据（自动分帧）
    /// let large_data = [0u8; 500];
    /// let frame_count = rpc.send_multi("command", &large_data).unwrap();
    /// println!("Sent {} frames", frame_count);
    /// ```
    pub fn send_multi(&mut self, key: &str, data: &[u8]) -> Result<usize, RpcError> {
        let max_payload = self.calculate_max_payload(key, true, false);
        
        if data.len() <= max_payload {
            self.send(key, data)?;
            return Ok(1);
        }
        
        self.send_index = self.send_index.wrapping_add(1);
        if self.send_index == 0 {
            self.send_index = 1;
        }
        let invoke_idx = self.send_index;
        
        let mut offset = 0;
        let mut frame_count = 0;
        let mut frame_idx: u8 = 0;
        
        while offset < data.len() {
            let remaining = data.len() - offset;
            let is_fin = remaining <= max_payload;
            let chunk_size = if is_fin { remaining } else { max_payload };
            
            let frame = self.build_frame_multi_secure(key, &data[offset..offset + chunk_size], invoke_idx, is_fin, frame_idx)?;
            (self.send)(&frame);
            
            frame_count += 1;
            offset += chunk_size;
            
            if !is_fin {
                frame_idx = frame_idx.wrapping_add(1);
            }
        }
        
        Ok(frame_count)
    }

    /// 构建多帧传输帧（非安全模式）
    fn build_frame_multi(&mut self, key: &str, data: &[u8], _secure: bool, fin: bool, frame_idx: u8) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        let mut frame: HeaplessVec<u8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
        
        frame.extend_from_slice(&FRAME_HEADER).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut type_key = TypeKey::new();
        type_key.pack_type = 2;
        type_key.is_array = key.len() != 1;
        type_key.width = 3;
        type_key.secure = false;
        type_key.fin = fin;
        
        let length_pos = frame.len();
        frame.push(0).map_err(|_| RpcError::MemoryNotEnough)?;
        
        frame.push(type_key.to_byte()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if key.len() != 1 {
            frame.push(key.len() as u8).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        frame.extend_from_slice(key.as_bytes()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if !fin {
            frame.push(frame_idx).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        
        frame.extend_from_slice(data).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut crc: u8 = 0;
        for i in FRAME_HEADER.len() + 1..frame.len() {
            crc = crc.wrapping_add(frame[i]);
        }
        
        let length = (frame.len() - FRAME_HEADER.len() - 1) as u8;
        frame[length_pos] = length;
        
        frame.push(crc).map_err(|_| RpcError::MemoryNotEnough)?;
        
        Ok(frame)
    }

    /// 构建多帧传输帧（安全模式）
    fn build_frame_multi_secure(&mut self, key: &str, data: &[u8], invoke_idx: u8, fin: bool, frame_idx: u8) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        let mut frame: HeaplessVec<u8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
        
        frame.extend_from_slice(&FRAME_HEADER).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut type_key = TypeKey::new();
        type_key.pack_type = 2;
        type_key.is_array = key.len() != 1;
        type_key.width = 3;
        type_key.secure = true;
        type_key.fin = fin;
        
        let length_pos = frame.len();
        frame.push(0).map_err(|_| RpcError::MemoryNotEnough)?;
        
        frame.push(type_key.to_byte()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if key.len() != 1 {
            frame.push(key.len() as u8).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        frame.extend_from_slice(key.as_bytes()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        frame.push(invoke_idx).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if !fin {
            frame.push(frame_idx).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        
        frame.extend_from_slice(data).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut crc: u8 = 0;
        for i in FRAME_HEADER.len() + 1..frame.len() {
            crc = crc.wrapping_add(frame[i]);
        }
        
        let length = (frame.len() - FRAME_HEADER.len() - 1) as u8;
        frame[length_pos] = length;
        
        frame.push(crc).map_err(|_| RpcError::MemoryNotEnough)?;
        
        Ok(frame)
    }

    /// 构建并返回帧数据（需要返回值）
    ///
    /// 用于调用需要返回结果的命令。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    /// - `data`: 要发送的数据
    /// - `_ret`: 返回数据缓冲区（当前未使用）
    ///
    /// # 返回值
    ///
    /// 返回构建的帧数据。
    ///
    /// # 错误
    ///
    /// - `RpcError::MemoryNotEnough`: 缓冲区不足
    /// - `RpcError::SendStatus`: 等待返回值超时
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {})
    ///     .with_delay(|| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// let mut ret_buf = [0u8; 64];
    /// let result = rpc.call("query", &[1, 2], &mut ret_buf);
    /// assert!(result.is_ok() || result.is_err());
    /// ```
    pub fn call(&mut self, key: &str, data: &[u8], _ret: &mut [u8]) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::info!("[RPC] call() 开始: key={}", key);
        
        let node = DynamicNode::new(key, DynamicNodeType::Normal);
        
        let frame = self.build_frame(key, data, false, true)?;
        log::debug!("[RPC] call() 构建帧完成, 长度={}", frame.len());
        
        self.insert_dynamic_node(node)?;
        log::debug!("[RPC] call() 插入动态节点完成");
        
        (self.send)(&frame);
        log::debug!("[RPC] call() 发送帧完成");
        
        log::info!("[RPC] call() 开始等待返回值...");
        let result = self.wait_send_complete(key);
        log::info!("[RPC] call() 等待返回值完成: {:?}", result.as_ref().map(|r| r.len()));
        
        self.remove_dynamic_node(key);
        log::debug!("[RPC] call() 移除动态节点完成");
        
        result
    }

    /// 开始调用（发送帧并插入动态节点）
    ///
    /// 用于异步场景，发送帧后立即返回，不等待返回值。
    /// 需要配合 `wait_call_result()` 使用。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    /// - `data`: 要发送的数据
    ///
    /// # 返回值
    ///
    /// 返回构建的帧数据。
    pub fn call_start(&mut self, key: &str, data: &[u8]) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::info!("[RPC] call_start() 开始: key={}", key);
        
        let node = DynamicNode::new(key, DynamicNodeType::Normal);
        
        let frame = self.build_frame(key, data, false, true)?;
        log::debug!("[RPC] call_start() 构建帧完成, 长度={}", frame.len());
        
        self.insert_dynamic_node(node)?;
        log::debug!("[RPC] call_start() 插入动态节点完成");
        
        (self.send)(&frame);
        log::debug!("[RPC] call_start() 发送帧完成");
        
        Ok(frame)
    }

    /// 等待调用结果
    ///
    /// 等待之前通过 `call_start()` 发送的命令返回结果。
    /// 在等待期间会释放锁，允许其他线程调用 `process()`。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    ///
    /// # 返回值
    ///
    /// 返回结果数据。
    pub fn wait_call_result(&mut self, key: &str) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::info!("[RPC] wait_call_result() 开始等待: key={}", key);
        
        let result = self.wait_send_complete(key);
        log::info!("[RPC] wait_call_result() 等待完成: {:?}", result.as_ref().map(|r| r.len()));
        
        self.remove_dynamic_node(key);
        log::debug!("[RPC] wait_call_result() 移除动态节点完成");
        
        result
    }

    /// 检查调用结果（非阻塞）
    ///
    /// 检查之前通过 `call_start()` 发送的命令是否已完成。
    /// 如果已完成，返回结果并移除动态节点；否则返回 None。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    ///
    /// # 返回值
    ///
    /// - `Some(Ok(data))`: 调用已完成，返回结果数据
    /// - `Some(Err(e))`: 调用失败
    /// - `None`: 调用尚未完成，需要继续等待
    pub fn check_call_result(&mut self, key: &str) -> Option<Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError>> {
        if let Some(node) = self.find_dynamic_node_mut(key) {
            if node.state == DynamicNodeState::Completed {
                let ret_data = node.ret_data.clone();
                log::info!("[RPC] check_call_result() 节点已完成: key={}, ret_len={}", key, ret_data.len());
                self.remove_dynamic_node(key);
                return Some(Ok(ret_data));
            }
        } else {
            log::debug!("[RPC] check_call_result() 节点不存在: key={}", key);
            return Some(Err(RpcError::SendStatus));
        }
        None
    }

    /// 返回结果
    ///
    /// 在处理函数中调用，用于返回结果数据给调用方。
    ///
    /// # 参数
    ///
    /// - `data`: 结果数据
    ///
    /// # 错误
    ///
    /// - `RpcError::NotUnderInvoke`: 不在调用上下文中（未在处理函数内调用）
    /// - `RpcError::MemoryNotEnough`: 缓冲区不足
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig, InvokeNode};
    ///
    /// fn handler(data: &[u8], size: usize, ret: Option<&mut [u8]>) -> i32 {
    ///     // 注意：在实际使用中，需要通过 RpcCore 实例调用 return_result
    ///     0
    /// }
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// // 在处理函数中：
    /// // rpc.return_result(&[1, 2, 3]).unwrap();
    /// ```
    pub fn return_result(&mut self, data: &[u8]) -> Result<(), RpcError> {
        if self.current_key_len == 0 {
            return Err(RpcError::NotUnderInvoke);
        }
        
        let mut key_buf: heapless::Vec<u8, MAX_SUPPORT_KEY_SIZE> = heapless::Vec::new();
        key_buf.extend_from_slice(&self.current_key[..self.current_key_len])
            .map_err(|_| RpcError::MemoryNotEnough)?;
        
        let key = core::str::from_utf8(&key_buf)
            .map_err(|_| RpcError::FormatError)?;
        
        let frame = self.build_frame(key, data, false, true)?;
        (self.send)(&frame);
        Ok(())
    }

    fn build_frame(&mut self, key: &str, data: &[u8], secure: bool, fin: bool) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        let mut frame: HeaplessVec<u8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
        
        frame.extend_from_slice(&FRAME_HEADER).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut type_key = TypeKey::new();
        type_key.pack_type = 2; // GH_PRO_TYPE_SIGNED = 2
        type_key.is_array = key.len() != 1;
        type_key.width = 3;
        type_key.secure = secure;
        type_key.fin = fin;
        
        let length_pos = frame.len();
        frame.push(0).map_err(|_| RpcError::MemoryNotEnough)?;
        
        frame.push(type_key.to_byte()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if key.len() != 1 {
            frame.push(key.len() as u8).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        frame.extend_from_slice(key.as_bytes()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if secure {
            self.send_index = self.send_index.wrapping_add(1);
            if self.send_index == 0 {
                self.send_index = 1;
            }
            frame.push(self.send_index).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        
        if !fin {
            frame.push(self.send_index).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        
        frame.extend_from_slice(data).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut crc: u8 = 0;
        for i in FRAME_HEADER.len() + 1..frame.len() {
            crc = crc.wrapping_add(frame[i]);
        }
        
        let length = (frame.len() - FRAME_HEADER.len() - 1) as u8;
        frame[length_pos] = length;
        
        frame.push(crc).map_err(|_| RpcError::MemoryNotEnough)?;
        
        Ok(frame)
    }

    /// 安全帧发送并等待返回值
    ///
    /// 发送安全帧（secure=true, fin=false），并等待接收方返回结果。
    /// 适用于需要确认和返回值的 RPC 调用场景。
    ///
    /// # 参数
    ///
    /// - `key`: 命令键（命令名称）
    /// - `data`: 要发送的数据
    /// - `ret`: 返回数据缓冲区（用于存储返回值）
    ///
    /// # 返回值
    ///
    /// 返回接收到的返回数据。
    ///
    /// # 错误
    ///
    /// - `RpcError::MemoryNotEnough`: 缓冲区不足或动态节点已满
    /// - `RpcError::LoseFrame`: 等待超时
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{RpcCore, RpcConfig};
    ///
    /// let config = RpcConfig::new(|_data: &[u8]| {});
    /// let mut rpc: RpcCore<16, _> = RpcCore::new(config);
    ///
    /// let mut ret_buf = [0u8; 64];
    /// let result = rpc.sall("query", &[1, 2], &mut ret_buf);
    /// ```
    pub fn sall(&mut self, key: &str, data: &[u8], _ret: &mut [u8]) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::info!("[RPC] sall() 开始: key={}, data_len={}", key, data.len());
        
        self.send_index = self.send_index.wrapping_add(1);
        if self.send_index == 0 {
            self.send_index = 1;
        }
        let com_id = self.send_index;
        log::debug!("[RPC] sall() 分配com_id={}", com_id);
        
        let mut node = DynamicNode::new(key, DynamicNodeType::Secure);
        node.com_id = com_id;
        
        let frame = self.build_frame_with_com_id(key, data, true, false, com_id)?;
        log::debug!("[RPC] sall() 构建安全帧完成, len={}", frame.len());
        
        node.add_frame_buffer(com_id, frame.clone())?;
        
        self.insert_dynamic_node(node)?;
        log::debug!("[RPC] sall() 插入动态节点完成");
        
        (self.send)(&frame);
        log::debug!("[RPC] sall() 发送帧完成");
        
        log::info!("[RPC] sall() 开始等待安全帧确认...");
        let result = self.secure_type_send_process(key, com_id);
        log::info!("[RPC] sall() 等待完成: {:?}", result.as_ref().map(|r| r.len()));
        
        self.remove_dynamic_node(key);
        
        result
    }

    /// 插入动态节点
    fn insert_dynamic_node(&mut self, node: DynamicNode) -> Result<(), RpcError> {
        log::debug!("[RPC] insert_dynamic_node() 尝试插入动态节点: key={}", node.key_str());
        for i in 0..MAX_DYNAMIC_NODES {
            if self.dynamic_nodes[i].is_none() {
                let mut key_buf: heapless::String<MAX_SUPPORT_KEY_SIZE> = heapless::String::new();
                let _ = key_buf.push_str(node.key_str());
                self.dynamic_nodes[i] = Some(node);
                log::info!("[RPC] insert_dynamic_node() 成功插入动态节点: key={}, slot={}", key_buf, i);
                return Ok(());
            }
        }
        log::error!("[RPC] insert_dynamic_node() 动态节点已满: key={}", node.key_str());
        Err(RpcError::MemoryNotEnough)
    }

    fn remove_dynamic_node(&mut self, key: &str) {
        log::debug!("[RPC] remove_dynamic_node() 尝试移除动态节点: key={}", key);
        for i in 0..MAX_DYNAMIC_NODES {
            if let Some(ref node) = self.dynamic_nodes[i] {
                if node.key_str() == key {
                    self.dynamic_nodes[i] = None;
                    log::info!("[RPC] remove_dynamic_node() 成功移除动态节点: key={}, slot={}", key, i);
                    return;
                }
            }
        }
        log::debug!("[RPC] remove_dynamic_node() 未找到动态节点: key={}", key);
    }

    fn find_dynamic_node_mut(&mut self, key: &str) -> Option<&mut DynamicNode> {
        for i in 0..MAX_DYNAMIC_NODES {
            if let Some(ref node) = self.dynamic_nodes[i] {
                if node.key_str() == key {
                    return self.dynamic_nodes[i].as_mut();
                }
            }
        }
        None
    }

    fn has_dynamic_node(&self, key: &str) -> bool {
        for i in 0..MAX_DYNAMIC_NODES {
            if let Some(ref node) = self.dynamic_nodes[i] {
                if node.key_str() == key {
                    return true;
                }
            }
        }
        false
    }

    fn find_dynamic_node_by_com_id_mut(&mut self, com_id: u8) -> Option<&mut DynamicNode> {
        for i in 0..MAX_DYNAMIC_NODES {
            if let Some(ref node) = self.dynamic_nodes[i] {
                if node.com_id == com_id {
                    return self.dynamic_nodes[i].as_mut();
                }
            }
        }
        None
    }

    /// 等待发送完成
    fn wait_send_complete(&mut self, key: &str) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::debug!("[RPC] wait_send_complete() 开始等待: key={}", key);
        let mut retry_time = COMM_RETRY_TIME + 1;
        
        while retry_time > 0 {
            retry_time -= 1;
            
            let node_exists = self.find_dynamic_node_mut(key).is_some();
            log::trace!("[RPC] wait_send_complete() 检查节点: key={}, exists={}, retry={}", key, node_exists, retry_time);
            
            if !node_exists {
                log::debug!("[RPC] wait_send_complete() 节点不存在，退出等待");
                break;
            }
            
            if let Some(node) = self.find_dynamic_node_mut(key) {
                if node.state == DynamicNodeState::Completed {
                    let ret_data = node.ret_data.clone();
                    log::info!("[RPC] wait_send_complete() 节点已完成，返回数据长度={}", ret_data.len());
                    return Ok(ret_data);
                }
            }
            
            if retry_time > 0 {
                if let Some(delay) = self.delay {
                    delay();
                }
            }
        }
        
        if retry_time == 0 {
            return Err(RpcError::SendStatus);
        }
        
        if let Some(node) = self.find_dynamic_node_mut(key) {
            if node.state == DynamicNodeState::Completed {
                return Ok(node.ret_data.clone());
            }
        }
        
        Err(RpcError::LoseFrame)
    }

    fn secure_type_send_process(&mut self, key: &str, com_id: u8) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        log::debug!("[RPC] secure_type_send_process() 开始: key={}, com_id={}", key, com_id);
        let mut retry_time = COMM_RETRY_TIME + 1;
        
        while retry_time > 0 {
            retry_time -= 1;
            
            if let Some(lock) = self.lock {
                lock();
            }
            
            let all_frames_confirmed = if let Some(node) = self.find_dynamic_node_mut(key) {
                let pending = node.has_pending_frames();
                log::trace!("[RPC] secure_type_send_process() 检查待确认帧: key={}, has_pending={}, retry={}", 
                    key, pending, retry_time);
                !pending
            } else {
                if let Some(unlock) = self.unlock {
                    unlock();
                }
                log::debug!("[RPC] secure_type_send_process() 节点不存在，退出: key={}", key);
                break;
            };
            
            if all_frames_confirmed {
                if let Some(unlock) = self.unlock {
                    unlock();
                }
                log::debug!("[RPC] secure_type_send_process() 所有帧已确认: key={}", key);
                break;
            }
            
            if (COMM_RETRY_TIME - retry_time) % COMM_RETRY_ROUND == 0 {
                if let Some(node) = self.find_dynamic_node_mut(key) {
                    let frames = node.get_frame_buffers_for_retry();
                    log::debug!("[RPC] secure_type_send_process() 重传帧: key={}, frame_count={}", key, frames.len());
                    for (_, frame_data) in frames {
                        (self.send)(&frame_data);
                    }
                }
            }
            
            if let Some(unlock) = self.unlock {
                unlock();
            }
            
            if retry_time > 0 {
                if let Some(delay) = self.delay {
                    delay();
                }
            }
        }
        
        if retry_time == 0 {
            log::warn!("[RPC] secure_type_send_process() 超时: key={}", key);
            if let Some(node) = self.find_dynamic_node_mut(key) {
                node.remove_all_frame_buffers();
            }
            return Err(RpcError::LoseFrame);
        }
        
        log::debug!("[RPC] secure_type_send_process() 开始等待返回值: key={}", key);
        self.wait_send_complete(key)
    }

    /// 构建带指定调用索引的帧
    fn build_frame_with_com_id(&mut self, key: &str, data: &[u8], secure: bool, fin: bool, com_id: u8) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        let mut frame: HeaplessVec<u8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
        
        frame.extend_from_slice(&FRAME_HEADER).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut type_key = TypeKey::new();
        type_key.pack_type = 2;
        type_key.is_array = key.len() != 1;
        type_key.width = 3;
        type_key.secure = secure;
        type_key.fin = fin;
        
        let length_pos = frame.len();
        frame.push(0).map_err(|_| RpcError::MemoryNotEnough)?;
        
        frame.push(type_key.to_byte()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if key.len() != 1 {
            frame.push(key.len() as u8).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        frame.extend_from_slice(key.as_bytes()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        if secure {
            frame.push(com_id).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        
        if !fin {
            frame.push(com_id).map_err(|_| RpcError::MemoryNotEnough)?;
        }
        
        frame.extend_from_slice(data).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut crc: u8 = 0;
        for i in FRAME_HEADER.len() + 1..frame.len() {
            crc = crc.wrapping_add(frame[i]);
        }
        
        let length = (frame.len() - FRAME_HEADER.len() - 1) as u8;
        frame[length_pos] = length;
        
        frame.push(crc).map_err(|_| RpcError::MemoryNotEnough)?;
        
        Ok(frame)
    }

    /// 更新动态节点的返回数据
    ///
    /// 当收到响应帧时调用此方法，更新对应动态节点的状态和数据。
    pub fn update_dynamic_node(&mut self, key: &str, data: &[u8]) -> bool {
        if let Some(node) = self.find_dynamic_node_mut(key) {
            node.ret_data.clear();
            let _ = node.ret_data.extend_from_slice(data);
            node.state = DynamicNodeState::Completed;
            return true;
        }
        false
    }

    /// 处理普通帧回调，更新动态节点的返回数据
    pub fn handle_normal_callback(&mut self, key: &str, data: &[u8]) {
        log::debug!("[RPC] handle_normal_callback() 处理普通回调: key={}, data_len={}", key, data.len());
        if let Some(node) = self.find_dynamic_node_mut(key) {
            node.ret_data.clear();
            let _ = node.ret_data.extend_from_slice(data);
            node.state = DynamicNodeState::Completed;
            log::info!("[RPC] handle_normal_callback() 动态节点更新完成: key={}", key);
        } else {
            log::debug!("[RPC] handle_normal_callback() 未找到动态节点: key={}", key);
        }
    }

    /// 处理安全帧回调，根据回调类型更新动态节点状态
    pub fn handle_secure_callback(&mut self, secure_return: &SecureReturn) {
        log::debug!("[RPC] handle_secure_callback() 处理安全帧回调: com_id={}, callback_type={:?}", 
            secure_return.com_id, secure_return.callback_type);
        if let Some(node) = self.find_dynamic_node_by_com_id_mut(secure_return.com_id) {
            if node.com_id != secure_return.com_id {
                log::warn!("[RPC] handle_secure_callback() com_id不匹配: expected={}, got={}", 
                    node.com_id, secure_return.com_id);
                return;
            }
            
            match secure_return.callback_type {
                SecureCallbackType::ReceiveFrame => {
                    log::debug!("[RPC] handle_secure_callback() ReceiveFrame: frame_idx={}", secure_return.data1);
                    node.remove_frame_buffer(secure_return.data1);
                }
                SecureCallbackType::Return => {
                    log::info!("[RPC] handle_secure_callback() Return: com_id={}", secure_return.com_id);
                    node.remove_all_frame_buffers();
                    node.state = DynamicNodeState::Completed;
                }
                SecureCallbackType::NoSuchFunction | SecureCallbackType::Error => {
                    log::warn!("[RPC] handle_secure_callback() 错误响应: {:?}", secure_return.callback_type);
                    node.remove_all_frame_buffers();
                    node.state = DynamicNodeState::Completed;
                }
            }
        } else {
            log::debug!("[RPC] handle_secure_callback() 未找到动态节点: com_id={}", secure_return.com_id);
        }
    }

    /// 发送 ACK 确认帧
    pub fn send_ack(&mut self, com_id: u8, frame_idx: u8) -> Result<(), RpcError> {
        log::debug!("[RPC] send_ack() 发送ACK: com_id={}, frame_idx={}", com_id, frame_idx);
        let ack_data = [com_id, frame_idx];
        let frame = self.build_ack_frame(&ack_data)?;
        (self.send)(&frame);
        log::debug!("[RPC] send_ack() ACK发送完成");
        Ok(())
    }

    fn build_ack_frame(&mut self, data: &[u8]) -> Result<HeaplessVec<u8, GHRPC_FRAME_SIZE>, RpcError> {
        let mut frame: HeaplessVec<u8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
        
        frame.extend_from_slice(&FRAME_HEADER).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut type_key = TypeKey::new();
        type_key.pack_type = 2;
        type_key.is_array = false;
        type_key.width = 3;
        type_key.secure = false;
        type_key.fin = true;
        
        let length_pos = frame.len();
        frame.push(0).map_err(|_| RpcError::MemoryNotEnough)?;
        
        frame.push(type_key.to_byte()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let ack_key = "ACK";
        frame.push(ack_key.len() as u8).map_err(|_| RpcError::MemoryNotEnough)?;
        frame.extend_from_slice(ack_key.as_bytes()).map_err(|_| RpcError::MemoryNotEnough)?;
        
        frame.extend_from_slice(data).map_err(|_| RpcError::MemoryNotEnough)?;
        
        let mut crc: u8 = 0;
        for i in FRAME_HEADER.len() + 1..frame.len() {
            crc = crc.wrapping_add(frame[i]);
        }
        
        let length = (frame.len() - FRAME_HEADER.len() - 1) as u8;
        frame[length_pos] = length;
        
        frame.push(crc).map_err(|_| RpcError::MemoryNotEnough)?;
        
        Ok(frame)
    }

    /// 处理接收到的 ACK 帧
    pub fn process_ack_frame(&mut self, result: &ParseResult) -> bool {
        let key_str = result.key_str();
        if key_str != "ACK" {
            return false;
        }
        
        if result.param.len() >= 2 {
            let com_id = result.param[0];
            let frame_idx = result.param[1];
            
            log::debug!("[RPC] process_ack_frame() 收到ACK: com_id={}, frame_idx={}", com_id, frame_idx);
            
            self.handle_secure_callback(&SecureReturn {
                callback_type: SecureCallbackType::ReceiveFrame,
                com_id,
                data1: frame_idx,
                data2: 0,
            });
            
            return true;
        }
        
        log::warn!("[RPC] process_ack_frame() ACK参数长度不足: len={}", result.param.len());
        false
    }
}

/// 格式标记类型
///
/// 表示格式字符串中的一个标记，如 `<u8>`, `<u16*>` 等。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatToken {
    /// 是否为有符号类型
    pub is_signed: bool,
    /// 数据宽度（字节数的 log2）
    pub width: u8,
    /// 是否为数组
    pub is_array: bool,
}

impl FormatToken {
    /// 获取数据大小（字节数）
    pub fn size(&self) -> usize {
        1 << self.width
    }

    /// 从格式字符串片段解析
    ///
    /// 支持的格式：
    /// - `<u8>`: 无符号 8 位整数
    /// - `<u16>`: 无符号 16 位整数
    /// - `<u32>`: 无符号 32 位整数
    /// - `<u64>`: 无符号 64 位整数
    /// - `<d8>`: 有符号 8 位整数
    /// - `<d16>`: 有符号 16 位整数
    /// - `<d32>`: 有符号 32 位整数
    /// - `<d64>`: 有符号 64 位整数
    /// - `<u8*>`: u8 数组（带长度前缀）
    /// - `<u16*>`: u16 数组（带长度前缀）
    pub fn parse(s: &str) -> Option<Self> {
        if s.len() < 2 {
            return None;
        }

        let is_signed = match s.chars().next()? {
            'u' => false,
            'd' => true,
            _ => return None,
        };

        let rest = &s[1..];
        let (num_str, is_array) = if rest.ends_with('*') {
            (&rest[..rest.len() - 1], true)
        } else {
            (rest, false)
        };

        let width = match num_str {
            "8" => 0,
            "16" => 1,
            "32" => 2,
            "64" => 3,
            _ => return None,
        };

        Some(Self {
            is_signed,
            width,
            is_array,
        })
    }
}

/// 解包后的值
///
/// 表示从帧数据中解包出的单个值。
#[derive(Debug, Clone, PartialEq)]
pub enum UnpackValue {
    /// 无符号 8 位整数
    U8(u8),
    /// 无符号 16 位整数
    U16(u16),
    /// 无符号 32 位整数
    U32(u32),
    /// 无符号 64 位整数
    U64(u64),
    /// 有符号 8 位整数
    I8(i8),
    /// 有符号 16 位整数
    I16(i16),
    /// 有符号 32 位整数
    I32(i32),
    /// 有符号 64 位整数
    I64(i64),
    /// u8 数组
    U8Array(HeaplessVec<u8, GHRPC_FRAME_SIZE>),
    /// u16 数组
    U16Array(HeaplessVec<u16, GHRPC_FRAME_SIZE>),
    /// u32 数组
    U32Array(HeaplessVec<u32, GHRPC_FRAME_SIZE>),
    /// u64 数组
    U64Array(HeaplessVec<u64, GHRPC_FRAME_SIZE>),
    /// i8 数组
    I8Array(HeaplessVec<i8, GHRPC_FRAME_SIZE>),
    /// i16 数组
    I16Array(HeaplessVec<i16, GHRPC_FRAME_SIZE>),
    /// i32 数组
    I32Array(HeaplessVec<i32, GHRPC_FRAME_SIZE>),
    /// i64 数组
    I64Array(HeaplessVec<i64, GHRPC_FRAME_SIZE>),
}

/// 解包结果
///
/// 包含解包后的所有值。
#[derive(Debug, Clone)]
pub struct UnpackResult {
    /// 解包出的值列表
    pub values: HeaplessVec<UnpackValue, 16>,
}

impl Default for UnpackResult {
    fn default() -> Self {
        Self::new()
    }
}

impl UnpackResult {
    /// 创建新的解包结果
    pub fn new() -> Self {
        Self {
            values: HeaplessVec::new(),
        }
    }

    /// 获取指定索引的 u8 值
    pub fn get_u8(&self, index: usize) -> Option<u8> {
        match self.values.get(index)? {
            UnpackValue::U8(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 u16 值
    pub fn get_u16(&self, index: usize) -> Option<u16> {
        match self.values.get(index)? {
            UnpackValue::U16(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 u32 值
    pub fn get_u32(&self, index: usize) -> Option<u32> {
        match self.values.get(index)? {
            UnpackValue::U32(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 u64 值
    pub fn get_u64(&self, index: usize) -> Option<u64> {
        match self.values.get(index)? {
            UnpackValue::U64(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 i8 值
    pub fn get_i8(&self, index: usize) -> Option<i8> {
        match self.values.get(index)? {
            UnpackValue::I8(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 i16 值
    pub fn get_i16(&self, index: usize) -> Option<i16> {
        match self.values.get(index)? {
            UnpackValue::I16(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 i32 值
    pub fn get_i32(&self, index: usize) -> Option<i32> {
        match self.values.get(index)? {
            UnpackValue::I32(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 i64 值
    pub fn get_i64(&self, index: usize) -> Option<i64> {
        match self.values.get(index)? {
            UnpackValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// 获取指定索引的 u8 数组
    pub fn get_u8_array(&self, index: usize) -> Option<&[u8]> {
        match self.values.get(index)? {
            UnpackValue::U8Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// 获取指定索引的 u16 数组
    pub fn get_u16_array(&self, index: usize) -> Option<&[u16]> {
        match self.values.get(index)? {
            UnpackValue::U16Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// 获取指定索引的 u32 数组
    pub fn get_u32_array(&self, index: usize) -> Option<&[u32]> {
        match self.values.get(index)? {
            UnpackValue::U32Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// 获取指定索引的 u64 数组
    pub fn get_u64_array(&self, index: usize) -> Option<&[u64]> {
        match self.values.get(index)? {
            UnpackValue::U64Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }
}

/// 解包读取器
///
/// 用于从已打包的数据中按格式字符串读取值。
///
/// # 示例
///
/// ```rust
/// use rpc::UnpackReader;
///
/// // 数据格式：<u8 类型头><u8 值><u16 类型头><u16 值>
/// let data: &[u8] = &[
///     0x00, 0x42,       // u8 类型头 + 值 0x42
///     0x08, 0x34, 0x12, // u16 类型头 + 值 0x1234
/// ];
/// let mut reader = UnpackReader::new(data);
/// let result = reader.unpack("<u8><u16>").unwrap();
///
/// let value_u8 = result.get_u8(0).unwrap();
/// let value_u16 = result.get_u16(1).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct UnpackReader<'a> {
    /// 数据引用
    data: &'a [u8],
    /// 当前读取位置
    pos: usize,
}

impl<'a> UnpackReader<'a> {
    /// 创建新的解包读取器
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// 获取当前读取位置
    pub fn position(&self) -> usize {
        self.pos
    }

    /// 获取剩余数据长度
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// 读取一个字节
    fn read_u8(&mut self) -> Option<u8> {
        if self.pos >= self.data.len() {
            return None;
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Some(value)
    }

    /// 读取两个字节（小端序）
    fn read_u16_le(&mut self) -> Option<u16> {
        if self.pos + 2 > self.data.len() {
            return None;
        }
        let value = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Some(value)
    }

    /// 读取四个字节（小端序）
    fn read_u32_le(&mut self) -> Option<u32> {
        if self.pos + 4 > self.data.len() {
            return None;
        }
        let value = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Some(value)
    }

    /// 读取八个字节（小端序）
    fn read_u64_le(&mut self) -> Option<u64> {
        if self.pos + 8 > self.data.len() {
            return None;
        }
        let value = u64::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ]);
        self.pos += 8;
        Some(value)
    }

    /// 读取指定长度的字节
    #[allow(dead_code)]
    fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return None;
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Some(slice)
    }

    /// 解析格式字符串
    fn parse_format(format: &str) -> Option<HeaplessVec<FormatToken, 16>> {
        let mut tokens: HeaplessVec<FormatToken, 16> = HeaplessVec::new();
        let mut chars = format.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '<' {
                let mut token_str = String::<16>::new();
                while let Some(&next) = chars.peek() {
                    if next == '>' {
                        chars.next();
                        break;
                    }
                    let _ = token_str.push(chars.next()?);
                }

                let token = FormatToken::parse(&token_str)?;
                tokens.push(token).ok()?;
            }
        }

        Some(tokens)
    }

    /// 按格式字符串解包数据
    ///
    /// # 参数
    ///
    /// - `format`: 格式字符串，如 `"<u8><u16><u32>"`
    ///
    /// # 返回值
    ///
    /// 返回解包后的结果，包含所有解析出的值。
    ///
    /// # 错误
    ///
    /// - `RpcError::FormatError`: 格式字符串无效
    /// - `RpcError::MemoryNotEnough`: 数据不足
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::UnpackReader;
    ///
    /// // 数据格式：<u8 类型头><u8 值><u16 类型头><u16 值>
    /// let data: &[u8] = &[
///     0x00, 0x42,       // u8 类型头 + 值 0x42
///     0x08, 0x34, 0x12, // u16 类型头 + 值 0x1234
/// ];
/// let mut reader = UnpackReader::new(data);
/// let result = reader.unpack("<u8><u16>").unwrap();
    /// ```
    pub fn unpack(&mut self, format: &str) -> Result<UnpackResult, RpcError> {
        let tokens = Self::parse_format(format).ok_or(RpcError::FormatError)?;
        let mut result = UnpackResult::new();

        for token in tokens.iter() {
            let type_header = self.read_u8().ok_or(RpcError::FormatError)?;

            if token.is_array {
                let value = self.read_array_value(token, type_header)?;
                result.values.push(value).map_err(|_| RpcError::MemoryNotEnough)?;
            } else {
                let value = self.read_scalar_value(token)?;
                result.values.push(value).map_err(|_| RpcError::MemoryNotEnough)?;
            }
        }

        Ok(result)
    }

    /// 读取标量值
    fn read_scalar_value(&mut self, token: &FormatToken) -> Result<UnpackValue, RpcError> {
        let value = match (token.is_signed, token.width) {
            (false, 0) => UnpackValue::U8(self.read_u8().ok_or(RpcError::FormatError)?),
            (false, 1) => UnpackValue::U16(self.read_u16_le().ok_or(RpcError::FormatError)?),
            (false, 2) => UnpackValue::U32(self.read_u32_le().ok_or(RpcError::FormatError)?),
            (false, 3) => UnpackValue::U64(self.read_u64_le().ok_or(RpcError::FormatError)?),
            (true, 0) => UnpackValue::I8(self.read_u8().ok_or(RpcError::FormatError)? as i8),
            (true, 1) => UnpackValue::I16(self.read_u16_le().ok_or(RpcError::FormatError)? as i16),
            (true, 2) => UnpackValue::I32(self.read_u32_le().ok_or(RpcError::FormatError)? as i32),
            (true, 3) => UnpackValue::I64(self.read_u64_le().ok_or(RpcError::FormatError)? as i64),
            _ => return Err(RpcError::FormatError),
        };
        Ok(value)
    }

    /// 读取数组值
    fn read_array_value(&mut self, token: &FormatToken, _type_header: u8) -> Result<UnpackValue, RpcError> {
        let length = self.read_u8().ok_or(RpcError::FormatError)? as usize;

        match (token.is_signed, token.width) {
            (false, 0) => {
                let mut arr: HeaplessVec<u8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u8().ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::U8Array(arr))
            }
            (false, 1) => {
                let mut arr: HeaplessVec<u16, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u16_le().ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::U16Array(arr))
            }
            (false, 2) => {
                let mut arr: HeaplessVec<u32, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u32_le().ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::U32Array(arr))
            }
            (false, 3) => {
                let mut arr: HeaplessVec<u64, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u64_le().ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::U64Array(arr))
            }
            (true, 0) => {
                let mut arr: HeaplessVec<i8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u8().ok_or(RpcError::FormatError)? as i8)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::I8Array(arr))
            }
            (true, 1) => {
                let mut arr: HeaplessVec<i16, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u16_le().ok_or(RpcError::FormatError)? as i16)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::I16Array(arr))
            }
            (true, 2) => {
                let mut arr: HeaplessVec<i32, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u32_le().ok_or(RpcError::FormatError)? as i32)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::I32Array(arr))
            }
            (true, 3) => {
                let mut arr: HeaplessVec<i64, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..length {
                    arr.push(self.read_u64_le().ok_or(RpcError::FormatError)? as i64)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                Ok(UnpackValue::I64Array(arr))
            }
            _ => Err(RpcError::FormatError),
        }
    }
}

/// 从帧数据中解包参数（简化版）
///
/// 直接从帧参数数据中解包，不包含类型头。
/// 适用于简单的参数解析场景。
///
/// # 参数
///
/// - `data`: 帧参数数据
/// - `format`: 格式字符串，如 `"<u8><u16><u32>"`
///
/// # 返回值
///
/// 返回解包后的结果。
///
/// # 示例
///
/// ```rust
/// use rpc::unpack;
///
/// // 简单的参数数据（无类型头）
/// let data: &[u8] = &[0x42, 0x34, 0x12];
/// let result = unpack(data, "<u8><u16>").unwrap();
///
/// assert_eq!(result.get_u8(0), Some(0x42));
/// assert_eq!(result.get_u16(1), Some(0x1234));
/// ```
pub fn unpack(data: &[u8], format: &str) -> Result<UnpackResult, RpcError> {
    let tokens = UnpackReader::parse_format(format).ok_or(RpcError::FormatError)?;
    let mut result = UnpackResult::new();
    let mut pos = 0;

    fn read_u8_at(data: &[u8], pos: &mut usize) -> Option<u8> {
        if *pos >= data.len() {
            return None;
        }
        let value = data[*pos];
        *pos += 1;
        Some(value)
    }

    fn read_u16_le_at(data: &[u8], pos: &mut usize) -> Option<u16> {
        if *pos + 2 > data.len() {
            return None;
        }
        let value = u16::from_le_bytes([data[*pos], data[*pos + 1]]);
        *pos += 2;
        Some(value)
    }

    fn read_u32_le_at(data: &[u8], pos: &mut usize) -> Option<u32> {
        if *pos + 4 > data.len() {
            return None;
        }
        let value = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
        *pos += 4;
        Some(value)
    }

    fn read_u64_le_at(data: &[u8], pos: &mut usize) -> Option<u64> {
        if *pos + 8 > data.len() {
            return None;
        }
        let value = u64::from_le_bytes([
            data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3],
            data[*pos + 4], data[*pos + 5], data[*pos + 6], data[*pos + 7],
        ]);
        *pos += 8;
        Some(value)
    }

    for token in tokens.iter() {
        let value = match (token.is_signed, token.is_array, token.width) {
            (false, false, 0) => UnpackValue::U8(read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?),
            (false, false, 1) => UnpackValue::U16(read_u16_le_at(data, &mut pos).ok_or(RpcError::FormatError)?),
            (false, false, 2) => UnpackValue::U32(read_u32_le_at(data, &mut pos).ok_or(RpcError::FormatError)?),
            (false, false, 3) => UnpackValue::U64(read_u64_le_at(data, &mut pos).ok_or(RpcError::FormatError)?),
            (true, false, 0) => UnpackValue::I8(read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as i8),
            (true, false, 1) => UnpackValue::I16(read_u16_le_at(data, &mut pos).ok_or(RpcError::FormatError)? as i16),
            (true, false, 2) => UnpackValue::I32(read_u32_le_at(data, &mut pos).ok_or(RpcError::FormatError)? as i32),
            (true, false, 3) => UnpackValue::I64(read_u64_le_at(data, &mut pos).ok_or(RpcError::FormatError)? as i64),
            (false, true, 0) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<u8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::U8Array(arr)
            }
            (false, true, 1) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<u16, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u16_le_at(data, &mut pos).ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::U16Array(arr)
            }
            (false, true, 2) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<u32, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u32_le_at(data, &mut pos).ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::U32Array(arr)
            }
            (false, true, 3) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<u64, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u64_le_at(data, &mut pos).ok_or(RpcError::FormatError)?)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::U64Array(arr)
            }
            (true, true, 0) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<i8, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as i8)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::I8Array(arr)
            }
            (true, true, 1) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<i16, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u16_le_at(data, &mut pos).ok_or(RpcError::FormatError)? as i16)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::I16Array(arr)
            }
            (true, true, 2) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<i32, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u32_le_at(data, &mut pos).ok_or(RpcError::FormatError)? as i32)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::I32Array(arr)
            }
            (true, true, 3) => {
                let _type_header = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)?;
                let len = read_u8_at(data, &mut pos).ok_or(RpcError::FormatError)? as usize;
                let mut arr: HeaplessVec<i64, GHRPC_FRAME_SIZE> = HeaplessVec::new();
                for _ in 0..len {
                    arr.push(read_u64_le_at(data, &mut pos).ok_or(RpcError::FormatError)? as i64)
                        .map_err(|_| RpcError::MemoryNotEnough)?;
                }
                UnpackValue::I64Array(arr)
            }
            _ => return Err(RpcError::FormatError),
        };
        result.values.push(value).map_err(|_| RpcError::MemoryNotEnough)?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoke_node_creation() {
        fn test_handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 { 0 }
        
        let node = InvokeNode::new("test", Some("<u8>"), Some(test_handler));
        assert_eq!(node.key, "test");
        assert_eq!(node.detail, Some("<u8>"));
        assert!(node.func.is_some());
    }

    #[test]
    fn test_rpc_core_registration() {
        fn test_handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 { 0 }
        
        let config = RpcConfig::new(|_data: &[u8]| {});
        
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);
        
        let node = InvokeNode::new("test_cmd", None, Some(test_handler));
        assert!(rpc.register(node).is_ok());
        assert_eq!(rpc.static_count, 1);
    }

    #[test]
    fn test_build_frame() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);
        
        let frame = rpc.build_frame("G", &[1, 2, 3], false, true).unwrap();
        
        assert!(frame.starts_with(&FRAME_HEADER));
        assert!(frame.len() > FRAME_HEADER.len() + 2);
    }

    #[test]
    fn test_publish() {
        use core::sync::atomic::{AtomicUsize, Ordering};
        static SENT_COUNT: AtomicUsize = AtomicUsize::new(0);
        SENT_COUNT.store(0, Ordering::SeqCst);
        
        let config = RpcConfig::new(|_data: &[u8]| {
            SENT_COUNT.fetch_add(1, Ordering::SeqCst);
        });
        
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);
        
        assert!(rpc.publish("test", &[1, 2, 3]).is_ok());
        
        assert_eq!(SENT_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_format_token_parse() {
        let token = FormatToken::parse("u8").unwrap();
        assert!(!token.is_signed);
        assert_eq!(token.width, 0);
        assert!(!token.is_array);

        let token = FormatToken::parse("u16").unwrap();
        assert!(!token.is_signed);
        assert_eq!(token.width, 1);

        let token = FormatToken::parse("d32").unwrap();
        assert!(token.is_signed);
        assert_eq!(token.width, 2);

        let token = FormatToken::parse("u8*").unwrap();
        assert!(token.is_array);
        assert_eq!(token.width, 0);
    }

    #[test]
    fn test_unpack_simple() {
        let data: &[u8] = &[0x42, 0x34, 0x12];
        let result = unpack(data, "<u8><u16>").unwrap();

        assert_eq!(result.get_u8(0), Some(0x42));
        assert_eq!(result.get_u16(1), Some(0x1234));
    }

    #[test]
    fn test_unpack_u32() {
        let data: &[u8] = &[0x78, 0x56, 0x34, 0x12];
        let result = unpack(data, "<u32>").unwrap();

        assert_eq!(result.get_u32(0), Some(0x12345678));
    }

    #[test]
    fn test_unpack_u64() {
        let data: &[u8] = &[0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01];
        let result = unpack(data, "<u64>").unwrap();

        assert_eq!(result.get_u64(0), Some(0x0123456789ABCDEF));
    }

    #[test]
    fn test_unpack_signed() {
        let data: &[u8] = &[0xFF];
        let result = unpack(data, "<d8>").unwrap();

        assert_eq!(result.get_i8(0), Some(-1));
    }

    #[test]
    fn test_unpack_array() {
        let data: &[u8] = &[3, 1, 2, 3];
        let result = unpack(data, "<u8*>").unwrap();

        let arr = result.get_u8_array(0).unwrap();
        assert_eq!(arr, &[1, 2, 3]);
    }

    #[test]
    fn test_unpack_u16_array() {
        let data: &[u8] = &[2, 0x34, 0x12, 0x78, 0x56];
        let result = unpack(data, "<u16*>").unwrap();

        let arr = result.get_u16_array(0).unwrap();
        assert_eq!(arr, &[0x1234, 0x5678]);
    }

    #[test]
    fn test_unpack_mixed() {
        let data: &[u8] = &[0x42, 0x34, 0x12, 2, 0x01, 0x00, 0x02, 0x00];
        let result = unpack(data, "<u8><u16><u16*>").unwrap();

        assert_eq!(result.get_u8(0), Some(0x42));
        assert_eq!(result.get_u16(1), Some(0x1234));
        let arr = result.get_u16_array(2).unwrap();
        assert_eq!(arr, &[1, 2]);
    }
}
