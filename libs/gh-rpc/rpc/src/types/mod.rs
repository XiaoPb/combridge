//! # RPC 核心类型定义
//!
//! 本模块定义 RPC 核心的基础类型，支持 `no_std` 环境。
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
//! ## 主要类型
//!
//! - `TypeKey`: 类型键结构体，标识帧属性
//! - `RpcError`: RPC 操作错误类型
//! - `FrameError`: 帧解析错误类型
//!
//! ## 常量
//!
//! - `FRAME_HEADER`: 帧头标识 `[0xAA, 0x11]`
//! - `GHRPC_FRAME_SIZE`: RPC 帧最大大小（256 字节）
//! - `MAX_SUPPORT_KEY_SIZE`: 最大键值大小（64 字节）
//!
//! ## 示例
//!
//! ```rust
//! use rpc::{TypeKey, FRAME_HEADER, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE};
//!
//! // 验证帧头
//! assert_eq!(FRAME_HEADER, [0xAA, 0x11]);
//!
//! // 验证帧大小限制
//! assert_eq!(GHRPC_FRAME_SIZE, 256);
//! assert_eq!(MAX_SUPPORT_KEY_SIZE, 64);
//!
//! // 创建类型键
//! let mut key = TypeKey::new();
//! key.set_pack_type(1);
//! key.set_secure(true);
//!
//! // 序列化和反序列化
//! let byte = key.to_byte();
//! let restored = TypeKey::from_byte(byte);
//! assert_eq!(key, restored);
//! ```

#![allow(dead_code)]

/// 帧头标识字节
///
/// 用于标识 RPC 帧的起始位置，固定为 `[0xAA, 0x11]`。
///
/// # 示例
///
/// ```rust
/// use rpc::FRAME_HEADER;
///
/// assert_eq!(FRAME_HEADER, [0xAA, 0x11]);
/// ```
pub const FRAME_HEADER: [u8; 2] = [0xAA, 0x11];

/// RPC 帧最大大小
///
/// 单个 RPC 帧的最大字节数，默认为 256 字节。
/// 此限制适用于嵌入式设备的内存约束场景。
///
/// # 示例
///
/// ```rust
/// use rpc::GHRPC_FRAME_SIZE;
///
/// assert_eq!(GHRPC_FRAME_SIZE, 256);
/// ```
pub const GHRPC_FRAME_SIZE: usize = 256;

/// 最大支持的键值大小
///
/// 命令名称（Key）的最大长度，默认为 64 字节。
/// 键用于标识 RPC 命令，如 `"get_status"`、`"set_config"` 等。
///
/// # 示例
///
/// ```rust
/// use rpc::MAX_SUPPORT_KEY_SIZE;
///
/// assert_eq!(MAX_SUPPORT_KEY_SIZE, 64);
/// ```
pub const MAX_SUPPORT_KEY_SIZE: usize = 64;

/// 类型键结构体
///
/// 用于标识帧中键数据的类型和属性，是 RPC 帧的重要组成部分。
///
/// ## 位定义
///
/// | 位 | 名称 | 说明 |
/// |----|------|------|
/// | 0-1 | pack_type | 打包类型（0-3） |
/// | 2 | is_array | 是否为数组 |
/// | 3-5 | width | 数据宽度（0-7） |
/// | 6 | secure | 是否为安全帧（需要 ACK） |
/// | 7 | fin | 是否为最后一帧 |
///
/// ## 打包类型
///
/// | 值 | 类型 |
/// |----|------|
/// | 0 | 无符号整数 |
/// | 1 | 有符号整数 |
/// | 2 | 字符串/字节序列 |
/// | 3 | 浮点数 |
///
/// ## 示例
///
/// ```rust
/// use rpc::TypeKey;
///
/// // 创建默认类型键
/// let key = TypeKey::new();
/// assert_eq!(key.pack_type, 2); // 默认为字符串类型
/// assert!(key.fin); // 默认为最后一帧
///
/// // 创建自定义类型键
/// let mut custom = TypeKey::new();
/// custom.set_pack_type(1);      // 有符号整数
/// custom.set_is_array(true);    // 数组
/// custom.set_width(3);          // 4 字节宽度
/// custom.set_secure(true);      // 需要确认
/// custom.set_fin(false);        // 非最后一帧
///
/// // 序列化为字节
/// let byte = custom.to_byte();
///
/// // 从字节反序列化
/// let restored = TypeKey::from_byte(byte);
/// assert_eq!(custom, restored);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeKey {
    /// 打包类型（位 0-1）
    ///
    /// - 0: 无符号整数
    /// - 1: 有符号整数
    /// - 2: 字符串/字节序列
    /// - 3: 浮点数
    pub pack_type: u8,

    /// 是否为数组（位 2）
    ///
    /// 当为 `true` 时，键数据为多个元素的数组。
    pub is_array: bool,

    /// 数据宽度（位 3-5）
    ///
    /// 表示数据宽度编码，实际字节数为 `2^(width+1)`。
    /// - 0: 1 字节
    /// - 1: 2 字节
    /// - 2: 4 字节
    /// - 3: 8 字节
    pub width: u8,

    /// 是否为安全帧（位 6）
    ///
    /// 当为 `true` 时，帧需要接收方确认（ACK）。
    /// 安全帧包含调用索引（ComID）用于确认匹配。
    pub secure: bool,

    /// 是否为最后一帧（位 7）
    ///
    /// 当为 `false` 时，表示还有后续帧。
    /// 用于大数据分帧传输场景。
    pub fin: bool,
}

impl TypeKey {
    /// 创建新的类型键
    ///
    /// 返回具有默认值的类型键：
    /// - `pack_type`: 2（字符串类型）
    /// - `is_array`: false
    /// - `width`: 7
    /// - `secure`: false
    /// - `fin`: true
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let key = TypeKey::new();
    /// assert_eq!(key.pack_type, 2);
    /// assert!(!key.is_array);
    /// assert!(key.fin);
    /// ```
    pub fn new() -> Self {
        Self {
            pack_type: 2,
            is_array: false,
            width: 7,
            secure: false,
            fin: true,
        }
    }

    /// 设置打包类型
    ///
    /// # 参数
    ///
    /// - `pack_type`: 打包类型值（0-3），超出范围会被截断
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let mut key = TypeKey::new();
    /// key.set_pack_type(1);
    /// assert_eq!(key.pack_type, 1);
    ///
    /// // 超出范围会被截断
    /// key.set_pack_type(5);
    /// assert_eq!(key.pack_type, 1); // 5 & 0b11 = 1
    /// ```
    pub fn set_pack_type(&mut self, pack_type: u8) {
        self.pack_type = pack_type & 0b11;
    }

    /// 设置是否为数组
    ///
    /// # 参数
    ///
    /// - `is_array`: 是否为数组
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let mut key = TypeKey::new();
    /// key.set_is_array(true);
    /// assert!(key.is_array);
    /// ```
    pub fn set_is_array(&mut self, is_array: bool) {
        self.is_array = is_array;
    }

    /// 设置数据宽度
    ///
    /// # 参数
    ///
    /// - `width`: 数据宽度值（0-7），超出范围会被截断
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let mut key = TypeKey::new();
    /// key.set_width(3);
    /// assert_eq!(key.width, 3);
    /// ```
    pub fn set_width(&mut self, width: u8) {
        self.width = width & 0b111;
    }

    /// 设置是否为安全帧
    ///
    /// # 参数
    ///
    /// - `secure`: 是否为安全帧（需要确认）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let mut key = TypeKey::new();
    /// key.set_secure(true);
    /// assert!(key.secure);
    /// ```
    pub fn set_secure(&mut self, secure: bool) {
        self.secure = secure;
    }

    /// 设置是否为最后一帧
    ///
    /// # 参数
    ///
    /// - `fin`: 是否为最后一帧
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let mut key = TypeKey::new();
    /// key.set_fin(false);
    /// assert!(!key.fin);
    /// ```
    pub fn set_fin(&mut self, fin: bool) {
        self.fin = fin;
    }

    /// 转换为字节
    ///
    /// 将类型键编码为单字节表示。
    ///
    /// # 返回值
    ///
    /// 编码后的字节值
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let key = TypeKey::new();
    /// let byte = key.to_byte();
    ///
    /// // 可以反序列化回原值
    /// let restored = TypeKey::from_byte(byte);
    /// assert_eq!(key, restored);
    /// ```
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0;
        byte |= self.pack_type & 0b11;
        byte |= (self.is_array as u8) << 2;
        byte |= (self.width & 0b111) << 3;
        byte |= (self.secure as u8) << 6;
        byte |= (self.fin as u8) << 7;
        byte
    }

    /// 从字节创建
    ///
    /// 从单字节解码类型键。
    ///
    /// # 参数
    ///
    /// - `byte`: 编码的字节值
    ///
    /// # 返回值
    ///
    /// 解码后的类型键
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeKey;
    ///
    /// let mut original = TypeKey::new();
    /// original.set_pack_type(1);
    /// original.set_secure(true);
    ///
    /// let byte = original.to_byte();
    /// let restored = TypeKey::from_byte(byte);
    ///
    /// assert_eq!(original, restored);
    /// ```
    pub fn from_byte(byte: u8) -> Self {
        Self {
            pack_type: byte & 0b11,
            is_array: (byte & 0b100) != 0,
            width: (byte >> 3) & 0b111,
            secure: (byte & 0b1000000) != 0,
            fin: (byte & 0b10000000) != 0,
        }
    }
}

impl Default for TypeKey {
    fn default() -> Self {
        Self::new()
    }
}

/// RPC 错误类型
///
/// 定义 RPC 操作可能返回的错误，用于指示操作失败原因。
///
/// # 错误码映射
///
/// | 错误码 | 错误类型 | 说明 |
/// |--------|----------|------|
/// | 1 | FormatError | 数据格式错误 |
/// | 2 | KeyOverMaxSize | 键值超过最大长度 |
/// | 3 | NotUnderInvoke | 不在调用上下文中 |
/// | 4 | SendFail | 发送失败 |
/// | 5 | MemoryNotEnough | 内存不足 |
/// | 6 | LoseFrame | 丢帧 |
/// | 7 | CrcError | CRC 校验错误 |
///
/// # 示例
///
/// ```rust
/// use rpc::RpcError;
///
/// let error = RpcError::MemoryNotEnough;
/// assert_eq!(error as i32, 5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcError {
    /// 格式错误
    ///
    /// 数据格式不符合 RPC 协议规范。
    FormatError = 1,

    /// 键值超过最大长度
    ///
    /// 命令名称超过了 [`MAX_SUPPORT_KEY_SIZE`] 限制。
    KeyOverMaxSize = 2,

    /// 不在调用上下文中
    ///
    /// 在非 RPC 调用上下文中执行了需要上下文的操作，
    /// 如在非处理函数中调用 `return_result`。
    NotUnderInvoke = 3,

    /// 发送失败
    ///
    /// 数据发送操作失败。
    SendFail = 4,

    /// 内存不足
    ///
    /// 内存分配失败或缓冲区已满。
    MemoryNotEnough = 5,

    /// 丢帧
    ///
    /// 多帧传输时丢失了部分帧。
    LoseFrame = 6,

    /// CRC 校验错误
    ///
    /// 接收到的帧 CRC 校验失败。
    CrcError = 7,

    /// 发送状态错误
    ///
    /// 动态节点插入重试次数耗尽。
    SendStatus = 8,

    /// 键未找到
    ///
    /// 指定的键在动态节点中不存在。
    KeyNotFound = 9,
}

/// 帧错误类型
///
/// 定义帧解析过程中可能返回的错误。
///
/// # 示例
///
/// ```rust
/// use rpc::FrameError;
///
/// let error = FrameError::CrcMismatch;
/// println!("Frame error: {:?}", error);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// 无效的帧头
    ///
    /// 帧头不是 `[0xAA, 0x11]`。
    InvalidHeader,

    /// 无效的长度
    ///
    /// 帧长度字段值无效。
    InvalidLength,

    /// CRC 校验不匹配
    ///
    /// 计算的 CRC 与帧中的 CRC 不一致。
    CrcMismatch,

    /// 无效的键
    ///
    /// 键数据无效或无法解析。
    InvalidKey,

    /// 无效的格式
    ///
    /// 帧格式不符合规范。
    InvalidFormat,
}

/// 类型标记枚举
///
/// 用于标识数据序列化时的数据类型。
///
/// ## 支持的类型
///
/// | 类型 | 大小 | 说明 |
/// |------|------|------|
/// | U8 | 1 字节 | 无符号 8 位整数 |
/// | U16 | 2 字节 | 无符号 16 位整数 |
/// | U32 | 4 字节 | 无符号 32 位整数 |
/// | U64 | 8 字节 | 无符号 64 位整数 |
/// | I8 | 1 字节 | 有符号 8 位整数 |
/// | I16 | 2 字节 | 有符号 16 位整数 |
/// | I32 | 4 字节 | 有符号 32 位整数 |
/// | I64 | 8 字节 | 有符号 64 位整数 |
/// | F32 | 4 字节 | 32 位浮点数 |
/// | F64 | 8 字节 | 64 位浮点数 |
/// | Array | N 字节 | 数组类型 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeMarker {
    /// 无符号 8 位整数
    U8,
    /// 无符号 16 位整数
    U16,
    /// 无符号 32 位整数
    U32,
    /// 无符号 64 位整数
    U64,
    /// 有符号 8 位整数
    I8,
    /// 有符号 16 位整数
    I16,
    /// 有符号 32 位整数
    I32,
    /// 有符号 64 位整数
    I64,
    /// 32 位浮点数
    F32,
    /// 64 位浮点数
    F64,
    /// 数组类型，包含元素类型
    Array(&'static TypeMarker),
}

impl TypeMarker {
    /// 获取类型的大小（字节数）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeMarker;
    ///
    /// assert_eq!(TypeMarker::U8.size(), 1);
    /// assert_eq!(TypeMarker::U32.size(), 4);
    /// assert_eq!(TypeMarker::F64.size(), 8);
    /// ```
    pub fn size(&self) -> usize {
        match self {
            TypeMarker::U8 | TypeMarker::I8 => 1,
            TypeMarker::U16 | TypeMarker::I16 => 2,
            TypeMarker::U32 | TypeMarker::I32 | TypeMarker::F32 => 4,
            TypeMarker::U64 | TypeMarker::I64 | TypeMarker::F64 => 8,
            TypeMarker::Array(inner) => inner.size(),
        }
    }

    /// 判断是否为有符号类型
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeMarker;
    ///
    /// assert!(TypeMarker::I32.is_signed());
    /// assert!(!TypeMarker::U32.is_signed());
    /// assert!(TypeMarker::F32.is_signed());
    /// ```
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            TypeMarker::I8
                | TypeMarker::I16
                | TypeMarker::I32
                | TypeMarker::I64
                | TypeMarker::F32
                | TypeMarker::F64
        )
    }

    /// 判断是否为浮点类型
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::TypeMarker;
    ///
    /// assert!(TypeMarker::F32.is_float());
    /// assert!(!TypeMarker::I32.is_float());
    /// ```
    pub fn is_float(&self) -> bool {
        matches!(self, TypeMarker::F32 | TypeMarker::F64)
    }

    /// 判断是否为数组类型
    pub fn is_array(&self) -> bool {
        matches!(self, TypeMarker::Array(_))
    }
}

/// 编解码错误类型
///
/// 定义数据序列化和反序列化可能返回的错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    /// 无效的格式字符串
    InvalidFormat,
    /// 无效的类型
    InvalidType,
    /// 缓冲区溢出
    BufferOverflow,
    /// 意外的数据结束
    UnexpectedEnd,
}

/// RPC 数据指针
///
/// 封装原始指针和大小，用于表示帧中的变长数据。
///
/// # 安全性
///
/// 本结构体包含原始指针，使用时需确保：
/// - 指针指向有效的内存区域
/// - 内存区域的大小至少为 `size` 字节
/// - 在使用期间内存不会被释放或移动
///
/// # 示例
///
/// ```rust
/// use rpc::RpcPoint;
///
/// // 创建空指针
/// let empty = RpcPoint::empty();
/// assert!(empty.is_empty());
///
/// // 从缓冲区创建
/// let mut buffer = [1u8, 2, 3, 4];
/// let point = RpcPoint::new(buffer.as_mut_ptr(), buffer.len());
/// assert_eq!(point.as_slice(), &[1, 2, 3, 4]);
/// ```
#[derive(Debug, Clone)]
pub struct RpcPoint {
    /// 数据指针
    pub point: *mut u8,
    /// 数据大小
    pub size: usize,
}

impl RpcPoint {
    /// 创建新的 RPC 指针
    ///
    /// # 参数
    ///
    /// - `point`: 数据起始指针
    /// - `size`: 数据大小
    ///
    /// # 安全性
    ///
    /// 调用者需确保指针有效且内存区域大小正确。
    pub fn new(point: *mut u8, size: usize) -> Self {
        Self { point, size }
    }

    /// 创建空的 RPC 指针
    pub fn empty() -> Self {
        Self {
            point: core::ptr::null_mut(),
            size: 0,
        }
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.point.is_null() || self.size == 0
    }

    /// 获取数据的不可变切片
    ///
    /// # 安全性
    ///
    /// 如果指针为空或大小为 0，返回空切片。
    /// 否则，返回从指针开始的切片。
    pub fn as_slice(&self) -> &[u8] {
        if self.point.is_null() || self.size == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.point, self.size) }
        }
    }

    /// 获取数据的可变切片
    ///
    /// # 安全性
    ///
    /// 如果指针为空或大小为 0，返回空切片。
    /// 否则，返回从指针开始的可变切片。
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        if self.point.is_null() || self.size == 0 {
            &mut []
        } else {
            unsafe { core::slice::from_raw_parts_mut(self.point, self.size) }
        }
    }
}

impl Default for RpcPoint {
    fn default() -> Self {
        Self::empty()
    }
}

unsafe impl Send for RpcPoint {}
unsafe impl Sync for RpcPoint {}

/// 帧数据结构
///
/// 表示一个完整的 RPC 帧，包含所有字段。
///
/// ## 字段说明
///
/// | 字段 | 大小 | 说明 |
/// |------|------|------|
/// | header | 2 bytes | 帧头标识 `[0xAA, 0x11]` |
/// | length | 1 byte | 帧长度（不含帧头和长度字段） |
/// | key_header | 1 byte | 类型键标志 |
/// | key_data | N bytes | 命令名称 |
/// | com_id | 1 byte | 通信 ID（安全帧模式） |
/// | frame_id | 1 byte | 帧 ID（非 FIN 模式） |
/// | param_data | N bytes | 参数数据 |
/// | crc | 1 byte | CRC 校验和 |
#[derive(Debug, Clone)]
pub struct FrameData {
    /// 帧头标识
    pub header: [u8; 2],
    /// 帧长度
    pub length: u8,
    /// 类型键标志
    pub key_header: TypeKey,
    /// 键数据（命令名称）
    pub key_data: RpcPoint,
    /// 通信 ID
    pub com_id: u8,
    /// 帧 ID
    pub frame_id: u8,
    /// 参数数据
    pub param_data: RpcPoint,
    /// CRC 校验和
    pub crc: u8,
}

impl FrameData {
    /// 创建新的帧数据
    pub fn new() -> Self {
        Self {
            header: FRAME_HEADER,
            length: 0,
            key_header: TypeKey::new(),
            key_data: RpcPoint::empty(),
            com_id: 0,
            frame_id: 0,
            param_data: RpcPoint::empty(),
            crc: 0,
        }
    }

    /// 验证帧头是否正确
    pub fn validate_header(&self) -> bool {
        self.header == FRAME_HEADER
    }

    /// 计算帧的总大小
    pub fn total_size(&self) -> usize {
        2 + 1 + 1 + self.key_data.size + 1 + 1 + self.param_data.size + 1
    }
}

impl Default for FrameData {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// 数据包头标志
    ///
    /// 用于标识帧数据中包含的字段。
    ///
    /// ## 位定义
    ///
    /// | 位 | 名称 | 说明 |
    /// |----|------|------|
    /// | 0 | RAWDATA_EN | 原始数据使能 |
    /// | 1 | PHY_VALUE_EN | 物理值使能 |
    /// | 2 | GS_DATA_EN | GS 数据使能 |
    /// | 3 | FLAGS_EN | 标志使能 |
    /// | 4 | ALG_DATA_EN | 算法数据使能 |
    /// | 5 | AGC_INFO_EN | AGC 信息使能 |
    /// | 6 | TIMESTAMP_EN | 时间戳使能 |
    /// | 7 | FRAMEID_EN | 帧 ID 使能 |
    /// | 8 | FUNC_ID_EN | 功能 ID 使能 |
    /// | 9 | SLOT_CFG_EN | 槽配置使能 |
    ///
    /// ## 示例
    ///
    /// ```rust
    /// use rpc::PackHeader;
    ///
    /// // 启用原始数据和时间戳
    /// let header = PackHeader::RAWDATA_EN | PackHeader::TIMESTAMP_EN;
    ///
    /// assert!(header.contains(PackHeader::RAWDATA_EN));
    /// assert!(header.contains(PackHeader::TIMESTAMP_EN));
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PackHeader: u32 {
        /// 原始数据使能
        const RAWDATA_EN = 1 << 0;
        /// 物理值使能
        const PHY_VALUE_EN = 1 << 1;
        /// GS 数据使能
        const GS_DATA_EN = 1 << 2;
        /// 标志使能
        const FLAGS_EN = 1 << 3;
        /// 算法数据使能
        const ALG_DATA_EN = 1 << 4;
        /// AGC 信息使能
        const AGC_INFO_EN = 1 << 5;
        /// 时间戳使能
        const TIMESTAMP_EN = 1 << 6;
        /// 帧 ID 使能
        const FRAMEID_EN = 1 << 7;
        /// 功能 ID 使能
        const FUNC_ID_EN = 1 << 8;
        /// 槽配置使能
        const SLOT_CFG_EN = 1 << 9;
    }
}

impl Default for PackHeader {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header() {
        assert_eq!(FRAME_HEADER, [0xAA, 0x11]);
    }

    #[test]
    fn test_ghrpc_frame_size() {
        assert_eq!(GHRPC_FRAME_SIZE, 256);
    }

    #[test]
    fn test_max_support_key_size() {
        assert_eq!(MAX_SUPPORT_KEY_SIZE, 64);
    }

    #[test]
    fn test_type_key_flags() {
        let key = TypeKey::new();
        let byte = key.to_byte();
        assert_eq!(byte & 0b11, key.pack_type);

        let mut key_secure = TypeKey::new();
        key_secure.set_secure(true);
        key_secure.set_fin(false);
        assert!(key_secure.secure);
        assert!(!key_secure.fin);

        let mut key_fin = TypeKey::new();
        key_fin.set_fin(true);
        assert!(key_fin.fin);
    }

    #[test]
    fn test_type_key_roundtrip() {
        let mut original = TypeKey::new();
        original.set_pack_type(1);
        original.set_is_array(true);
        original.set_width(5);
        original.set_secure(true);
        original.set_fin(false);

        let byte = original.to_byte();
        let restored = TypeKey::from_byte(byte);

        assert_eq!(original, restored);
    }

    #[test]
    fn test_type_marker_size() {
        assert_eq!(TypeMarker::U8.size(), 1);
        assert_eq!(TypeMarker::U16.size(), 2);
        assert_eq!(TypeMarker::U32.size(), 4);
        assert_eq!(TypeMarker::U64.size(), 8);
        assert_eq!(TypeMarker::F32.size(), 4);
        assert_eq!(TypeMarker::F64.size(), 8);
    }

    #[test]
    fn test_type_marker_properties() {
        assert!(TypeMarker::I32.is_signed());
        assert!(!TypeMarker::U32.is_signed());
        assert!(TypeMarker::F32.is_float());
        assert!(!TypeMarker::I32.is_float());
    }

    #[test]
    fn test_rpc_point() {
        let point = RpcPoint::empty();
        assert!(point.is_empty());
        assert!(point.as_slice().is_empty());
    }

    #[test]
    fn test_frame_data() {
        let frame = FrameData::new();
        assert!(frame.validate_header());
        assert_eq!(frame.length, 0);
        assert_eq!(frame.com_id, 0);
        assert_eq!(frame.frame_id, 0);
    }

    #[test]
    fn test_pack_header() {
        let header = PackHeader::RAWDATA_EN | PackHeader::TIMESTAMP_EN;
        assert!(header.contains(PackHeader::RAWDATA_EN));
        assert!(header.contains(PackHeader::TIMESTAMP_EN));
        assert!(!header.contains(PackHeader::GS_DATA_EN));
    }
}
