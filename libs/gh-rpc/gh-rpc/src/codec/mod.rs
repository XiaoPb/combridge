//! 序列化和反序列化模块
//!
//! 本模块提供数据打包/解包功能，兼容 C 版本的 `gh_package.c`。
//!
//! ## 主要组件
//!
//! - [`FormatSpec`]: 格式规范解析器
//! - [`DynValue`]: 动态类型值
//! - [`Packer`]: 数据打包器
//! - [`Unpacker`]: 数据解包器
//!
//! ## 示例
//!
//! ```rust
//! use gh_rpc::codec::{Packer, Unpacker, FormatSpec, DynValue};
//!
//! // 打包数据
//! let mut buffer = [0u8; 16];
//! let mut packer = Packer::new(&mut buffer);
//! packer.pack_u8(0x12).unwrap();
//! packer.pack_u16(0x3456).unwrap();
//! let packed = packer.finish();
//!
//! // 解包数据
//! let mut unpacker = Unpacker::new(packed);
//! assert_eq!(unpacker.unpack_u8().unwrap(), 0x12);
//! assert_eq!(unpacker.unpack_u16().unwrap(), 0x3456);
//! ```

use rpc::types::{CodecError, TypeMarker};
use heapless::Vec as HeaplessVec;

const MAX_TYPE_MARKERS: usize = 16;

/// 格式规范解析器
///
/// 解析格式字符串并存储类型信息。
///
/// # 示例
///
/// ```rust
/// use gh_rpc::codec::FormatSpec;
///
/// let spec = FormatSpec::parse("<u8><u16><u32>").unwrap();
/// assert_eq!(spec.types().len(), 3);
/// assert_eq!(spec.data_size(), 7);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatSpec {
    types: HeaplessVec<TypeMarker, MAX_TYPE_MARKERS>,
}

impl FormatSpec {
    /// 解析格式字符串
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
    /// - `<f32>`: 32 位浮点数
    /// - `<f64>`: 64 位浮点数
    /// - `<u8*>`: u8 数组
    /// - `<u16*>`: u16 数组
    /// - `<u32*>`: u32 数组
    ///
    /// # 参数
    ///
    /// * `s` - 格式字符串
    ///
    /// # 返回值
    ///
    /// 成功返回 `FormatSpec`，失败返回错误
    pub fn parse(s: &str) -> Result<Self, CodecError> {
        let mut types = HeaplessVec::new();
        let mut chars = s.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c != '<' {
                chars.next();
                continue;
            }
            chars.next();

            let type_char = chars.next().ok_or(CodecError::InvalidFormat)?;
            let mut size_str = heapless::String::<8>::new();
            let mut is_array = false;

            while let Some(&c) = chars.peek() {
                if c == '>' {
                    chars.next();
                    break;
                }
                if c == '*' {
                    is_array = true;
                    chars.next();
                    continue;
                }
                size_str
                    .push(chars.next().unwrap())
                    .map_err(|_| CodecError::InvalidFormat)?;
            }

            let size: u32 = size_str
                .parse()
                .map_err(|_| CodecError::InvalidFormat)?;

            let base_type = match (type_char, size) {
                ('u', 8) => TypeMarker::U8,
                ('u', 16) => TypeMarker::U16,
                ('u', 32) => TypeMarker::U32,
                ('u', 64) => TypeMarker::U64,
                ('d', 8) => TypeMarker::I8,
                ('d', 16) => TypeMarker::I16,
                ('d', 32) => TypeMarker::I32,
                ('d', 64) => TypeMarker::I64,
                ('f', 32) => TypeMarker::F32,
                ('f', 64) => TypeMarker::F64,
                _ => return Err(CodecError::InvalidFormat),
            };

            let final_type = if is_array {
                static U8_MARKER: TypeMarker = TypeMarker::U8;
                static U16_MARKER: TypeMarker = TypeMarker::U16;
                static U32_MARKER: TypeMarker = TypeMarker::U32;

                let inner: &'static TypeMarker = match base_type {
                    TypeMarker::U8 => &U8_MARKER,
                    TypeMarker::U16 => &U16_MARKER,
                    TypeMarker::U32 => &U32_MARKER,
                    _ => return Err(CodecError::InvalidFormat),
                };
                TypeMarker::Array(inner)
            } else {
                base_type
            };

            types
                .push(final_type)
                .map_err(|_| CodecError::InvalidFormat)?;
        }

        Ok(Self { types })
    }

    /// 获取类型列表
    pub fn types(&self) -> &[TypeMarker] {
        &self.types
    }

    /// 计算数据总大小（字节）
    pub fn data_size(&self) -> usize {
        self.types.iter().map(|t| t.size()).sum()
    }
}

/// 动态类型值
///
/// 用于在运行时存储不同类型的值。
#[derive(Debug, Clone, Copy)]
pub enum DynValue {
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
    /// 32 位浮点数
    F32(f32),
    /// 64 位浮点数
    F64(f64),
    /// 数组
    Array {
        /// 数据指针
        ptr: *const u8,
        /// 数据长度
        len: usize,
    },
}

impl Default for DynValue {
    fn default() -> Self {
        DynValue::U8(0)
    }
}

unsafe impl Send for DynValue {}
unsafe impl Sync for DynValue {}

impl DynValue {
    /// 转换为 u8
    pub fn as_u8(&self) -> Option<u8> {
        match self {
            DynValue::U8(v) => Some(*v),
            _ => None,
        }
    }

    /// 转换为 u16
    pub fn as_u16(&self) -> Option<u16> {
        match self {
            DynValue::U16(v) => Some(*v),
            _ => None,
        }
    }

    /// 转换为 u32
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            DynValue::U32(v) => Some(*v),
            _ => None,
        }
    }

    /// 转换为 i32
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            DynValue::I32(v) => Some(*v),
            _ => None,
        }
    }

    /// 转换为数组指针和长度
    pub fn as_array(&self) -> Option<(*const u8, usize)> {
        match self {
            DynValue::Array { ptr, len } => Some((*ptr, *len)),
            _ => None,
        }
    }
}

/// 将 u16 转换为大端字节序
#[inline]
pub fn to_big_endian_16(value: u16) -> [u8; 2] {
    value.to_be_bytes()
}

/// 将 u32 转换为大端字节序
#[inline]
pub fn to_big_endian_32(value: u32) -> [u8; 4] {
    value.to_be_bytes()
}

/// 从大端字节序解析 u16
#[inline]
pub fn from_big_endian_16(bytes: [u8; 2]) -> u16 {
    u16::from_be_bytes(bytes)
}

/// 从大端字节序解析 u32
#[inline]
pub fn from_big_endian_32(bytes: [u8; 4]) -> u32 {
    u32::from_be_bytes(bytes)
}

/// 数据打包器
///
/// 将数据打包到字节缓冲区。
///
/// # 示例
///
/// ```rust
/// use gh_rpc::codec::Packer;
///
/// let mut buffer = [0u8; 16];
/// let mut packer = Packer::new(&mut buffer);
/// packer.pack_u8(0x12).unwrap();
/// packer.pack_u16(0x3456).unwrap();
/// let result = packer.finish();
/// assert_eq!(result, &[0x12, 0x34, 0x56]);
/// ```
pub struct Packer<'a> {
    buffer: &'a mut [u8],
    pos: usize,
}

impl<'a> Packer<'a> {
    /// 创建新的打包器
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer, pos: 0 }
    }

    /// 打包 u8
    pub fn pack_u8(&mut self, value: u8) -> Result<(), CodecError> {
        if self.pos >= self.buffer.len() {
            return Err(CodecError::BufferOverflow);
        }
        self.buffer[self.pos] = value;
        self.pos += 1;
        Ok(())
    }

    /// 打包 u16（大端字节序）
    pub fn pack_u16(&mut self, value: u16) -> Result<(), CodecError> {
        if self.pos + 2 > self.buffer.len() {
            return Err(CodecError::BufferOverflow);
        }
        let bytes = to_big_endian_16(value);
        self.buffer[self.pos..self.pos + 2].copy_from_slice(&bytes);
        self.pos += 2;
        Ok(())
    }

    /// 打包 u32（大端字节序）
    pub fn pack_u32(&mut self, value: u32) -> Result<(), CodecError> {
        if self.pos + 4 > self.buffer.len() {
            return Err(CodecError::BufferOverflow);
        }
        let bytes = to_big_endian_32(value);
        self.buffer[self.pos..self.pos + 4].copy_from_slice(&bytes);
        self.pos += 4;
        Ok(())
    }

    /// 打包 u64（大端字节序）
    pub fn pack_u64(&mut self, value: u64) -> Result<(), CodecError> {
        if self.pos + 8 > self.buffer.len() {
            return Err(CodecError::BufferOverflow);
        }
        let bytes = value.to_be_bytes();
        self.buffer[self.pos..self.pos + 8].copy_from_slice(&bytes);
        self.pos += 8;
        Ok(())
    }

    /// 打包 i8
    pub fn pack_i8(&mut self, value: i8) -> Result<(), CodecError> {
        self.pack_u8(value as u8)
    }

    /// 打包 i16（大端字节序）
    pub fn pack_i16(&mut self, value: i16) -> Result<(), CodecError> {
        self.pack_u16(value as u16)
    }

    /// 打包 i32（大端字节序）
    pub fn pack_i32(&mut self, value: i32) -> Result<(), CodecError> {
        self.pack_u32(value as u32)
    }

    /// 打包 i64（大端字节序）
    pub fn pack_i64(&mut self, value: i64) -> Result<(), CodecError> {
        self.pack_u64(value as u64)
    }

    /// 打包 f32
    pub fn pack_f32(&mut self, value: f32) -> Result<(), CodecError> {
        self.pack_u32(value.to_bits())
    }

    /// 打包 f64
    pub fn pack_f64(&mut self, value: f64) -> Result<(), CodecError> {
        self.pack_u64(value.to_bits())
    }

    /// 打包字节数组
    pub fn pack_array(&mut self, data: &[u8]) -> Result<(), CodecError> {
        if self.pos + data.len() > self.buffer.len() {
            return Err(CodecError::BufferOverflow);
        }
        self.buffer[self.pos..self.pos + data.len()].copy_from_slice(data);
        self.pos += data.len();
        Ok(())
    }

    /// 按格式规范打包数据
    pub fn pack_with_format(
        &mut self,
        format: &FormatSpec,
        values: &[DynValue],
    ) -> Result<(), CodecError> {
        if format.types().len() != values.len() {
            return Err(CodecError::InvalidFormat);
        }

        for (type_marker, value) in format.types().iter().zip(values.iter()) {
            match (type_marker, value) {
                (TypeMarker::U8, DynValue::U8(v)) => self.pack_u8(*v)?,
                (TypeMarker::U16, DynValue::U16(v)) => self.pack_u16(*v)?,
                (TypeMarker::U32, DynValue::U32(v)) => self.pack_u32(*v)?,
                (TypeMarker::U64, DynValue::U64(v)) => self.pack_u64(*v)?,
                (TypeMarker::I8, DynValue::I8(v)) => self.pack_i8(*v)?,
                (TypeMarker::I16, DynValue::I16(v)) => self.pack_i16(*v)?,
                (TypeMarker::I32, DynValue::I32(v)) => self.pack_i32(*v)?,
                (TypeMarker::I64, DynValue::I64(v)) => self.pack_i64(*v)?,
                (TypeMarker::F32, DynValue::F32(v)) => self.pack_f32(*v)?,
                (TypeMarker::F64, DynValue::F64(v)) => self.pack_f64(*v)?,
                (TypeMarker::Array(inner), DynValue::Array { ptr, len }) => {
                    let elem_size = inner.size();
                    if ptr.is_null() || *len == 0 {
                        continue;
                    }
                    let data = unsafe { core::slice::from_raw_parts(*ptr, *len * elem_size) };
                    self.pack_array(data)?;
                }
                _ => return Err(CodecError::InvalidType),
            }
        }

        Ok(())
    }

    /// 获取已打包的数据长度
    pub fn len(&self) -> usize {
        self.pos
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.pos == 0
    }

    /// 完成打包并返回数据切片
    pub fn finish(self) -> &'a [u8] {
        &self.buffer[..self.pos]
    }

    /// 获取当前已打包的数据切片
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.pos]
    }
}

/// 数据解包器
///
/// 从字节缓冲区解包数据。
///
/// # 示例
///
/// ```rust
/// use gh_rpc::codec::Unpacker;
///
/// let buffer = [0x12, 0x34, 0x56];
/// let mut unpacker = Unpacker::new(&buffer);
/// assert_eq!(unpacker.unpack_u8().unwrap(), 0x12);
/// assert_eq!(unpacker.unpack_u16().unwrap(), 0x3456);
/// ```
pub struct Unpacker<'a> {
    buffer: &'a [u8],
    pos: usize,
}

impl<'a> Unpacker<'a> {
    /// 创建新的解包器
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, pos: 0 }
    }

    /// 解包 u8
    pub fn unpack_u8(&mut self) -> Result<u8, CodecError> {
        if self.pos >= self.buffer.len() {
            return Err(CodecError::UnexpectedEnd);
        }
        let value = self.buffer[self.pos];
        self.pos += 1;
        Ok(value)
    }

    /// 解包 u16（大端字节序）
    pub fn unpack_u16(&mut self) -> Result<u16, CodecError> {
        if self.pos + 2 > self.buffer.len() {
            return Err(CodecError::UnexpectedEnd);
        }
        let bytes: [u8; 2] = self.buffer[self.pos..self.pos + 2]
            .try_into()
            .map_err(|_| CodecError::UnexpectedEnd)?;
        self.pos += 2;
        Ok(from_big_endian_16(bytes))
    }

    /// 解包 u32（大端字节序）
    pub fn unpack_u32(&mut self) -> Result<u32, CodecError> {
        if self.pos + 4 > self.buffer.len() {
            return Err(CodecError::UnexpectedEnd);
        }
        let bytes: [u8; 4] = self.buffer[self.pos..self.pos + 4]
            .try_into()
            .map_err(|_| CodecError::UnexpectedEnd)?;
        self.pos += 4;
        Ok(from_big_endian_32(bytes))
    }

    /// 解包 u64（大端字节序）
    pub fn unpack_u64(&mut self) -> Result<u64, CodecError> {
        if self.pos + 8 > self.buffer.len() {
            return Err(CodecError::UnexpectedEnd);
        }
        let bytes: [u8; 8] = self.buffer[self.pos..self.pos + 8]
            .try_into()
            .map_err(|_| CodecError::UnexpectedEnd)?;
        self.pos += 8;
        Ok(u64::from_be_bytes(bytes))
    }

    /// 解包 i8
    pub fn unpack_i8(&mut self) -> Result<i8, CodecError> {
        Ok(self.unpack_u8()? as i8)
    }

    /// 解包 i16（大端字节序）
    pub fn unpack_i16(&mut self) -> Result<i16, CodecError> {
        Ok(self.unpack_u16()? as i16)
    }

    /// 解包 i32（大端字节序）
    pub fn unpack_i32(&mut self) -> Result<i32, CodecError> {
        Ok(self.unpack_u32()? as i32)
    }

    /// 解包 i64（大端字节序）
    pub fn unpack_i64(&mut self) -> Result<i64, CodecError> {
        Ok(self.unpack_u64()? as i64)
    }

    /// 解包 f32
    pub fn unpack_f32(&mut self) -> Result<f32, CodecError> {
        Ok(f32::from_bits(self.unpack_u32()?))
    }

    /// 解包 f64
    pub fn unpack_f64(&mut self) -> Result<f64, CodecError> {
        Ok(f64::from_bits(self.unpack_u64()?))
    }

    /// 解包指定长度的字节数组
    pub fn unpack_array(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        if self.pos + len > self.buffer.len() {
            return Err(CodecError::UnexpectedEnd);
        }
        let slice = &self.buffer[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    /// 获取剩余未解包的字节数
    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.pos)
    }

    /// 检查是否已解包完毕
    pub fn is_empty(&self) -> bool {
        self.pos >= self.buffer.len()
    }

    /// 获取当前位置
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// 重置到起始位置
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// 查看下一个 u8 但不移动位置
    pub fn peek_u8(&self) -> Result<u8, CodecError> {
        if self.pos >= self.buffer.len() {
            return Err(CodecError::UnexpectedEnd);
        }
        Ok(self.buffer[self.pos])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_spec_parse() {
        let spec = FormatSpec::parse("<u8><u16><u32>").unwrap();
        assert_eq!(spec.types().len(), 3);
        assert_eq!(spec.types()[0], TypeMarker::U8);
        assert_eq!(spec.types()[1], TypeMarker::U16);
        assert_eq!(spec.types()[2], TypeMarker::U32);
    }

    #[test]
    fn test_format_spec_signed() {
        let spec = FormatSpec::parse("<d8><d16><d32>").unwrap();
        assert_eq!(spec.types().len(), 3);
        assert_eq!(spec.types()[0], TypeMarker::I8);
        assert_eq!(spec.types()[1], TypeMarker::I16);
        assert_eq!(spec.types()[2], TypeMarker::I32);
    }

    #[test]
    fn test_format_spec_float() {
        let spec = FormatSpec::parse("<f32><f64>").unwrap();
        assert_eq!(spec.types().len(), 2);
        assert_eq!(spec.types()[0], TypeMarker::F32);
        assert_eq!(spec.types()[1], TypeMarker::F64);
    }

    #[test]
    fn test_format_spec_array() {
        let spec = FormatSpec::parse("<u8*><u16*>").unwrap();
        assert_eq!(spec.types().len(), 2);
        assert!(spec.types()[0].is_array());
        assert!(spec.types()[1].is_array());
    }

    #[test]
    fn test_format_spec_data_size() {
        let spec = FormatSpec::parse("<u8><u16><u32>").unwrap();
        assert_eq!(spec.data_size(), 7);
    }

    #[test]
    fn test_format_spec_invalid() {
        assert!(FormatSpec::parse("<invalid>").is_err());
        assert!(FormatSpec::parse("<u33>").is_err());
    }

    #[test]
    fn test_endian_conversion() {
        assert_eq!(to_big_endian_16(0x1234), [0x12, 0x34]);
        assert_eq!(from_big_endian_16([0x12, 0x34]), 0x1234);

        assert_eq!(to_big_endian_32(0x12345678), [0x12, 0x34, 0x56, 0x78]);
        assert_eq!(from_big_endian_32([0x12, 0x34, 0x56, 0x78]), 0x12345678);
    }

    #[test]
    fn test_packer_u8() {
        let mut buffer = [0u8; 16];
        let mut packer = Packer::new(&mut buffer);
        packer.pack_u8(0x12).unwrap();
        packer.pack_u8(0x34).unwrap();
        assert_eq!(packer.len(), 2);
        let result = packer.finish();
        assert_eq!(result, &[0x12, 0x34]);
    }

    #[test]
    fn test_packer_u16() {
        let mut buffer = [0u8; 16];
        let mut packer = Packer::new(&mut buffer);
        packer.pack_u16(0x1234).unwrap();
        assert_eq!(packer.len(), 2);
        let result = packer.finish();
        assert_eq!(result, &[0x12, 0x34]);
    }

    #[test]
    fn test_packer_u32() {
        let mut buffer = [0u8; 16];
        let mut packer = Packer::new(&mut buffer);
        packer.pack_u32(0x12345678).unwrap();
        assert_eq!(packer.len(), 4);
        let result = packer.finish();
        assert_eq!(result, &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_packer_i32() {
        let mut buffer = [0u8; 16];
        let mut packer = Packer::new(&mut buffer);
        packer.pack_i32(-1).unwrap();
        assert_eq!(packer.len(), 4);
        let result = packer.finish();
        assert_eq!(result, &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_packer_array() {
        let mut buffer = [0u8; 16];
        let mut packer = Packer::new(&mut buffer);
        packer.pack_array(&[1, 2, 3, 4]).unwrap();
        assert_eq!(packer.len(), 4);
        let result = packer.finish();
        assert_eq!(result, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_packer_overflow() {
        let mut buffer = [0u8; 2];
        let mut packer = Packer::new(&mut buffer);
        packer.pack_u8(1).unwrap();
        packer.pack_u8(2).unwrap();
        assert!(packer.pack_u8(3).is_err());
    }

    #[test]
    fn test_packer_with_format() {
        let mut buffer = [0u8; 16];
        let mut packer = Packer::new(&mut buffer);
        let spec = FormatSpec::parse("<u8><u16><u32>").unwrap();
        let values = [
            DynValue::U8(0x12),
            DynValue::U16(0x3456),
            DynValue::U32(0x789ABCDE),
        ];
        packer.pack_with_format(&spec, &values).unwrap();
        assert_eq!(packer.len(), 7);
        let result = packer.finish();
        assert_eq!(result, &[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE]);
    }

    #[test]
    fn test_unpacker_u8() {
        let buffer = [0x12, 0x34];
        let mut unpacker = Unpacker::new(&buffer);
        assert_eq!(unpacker.unpack_u8().unwrap(), 0x12);
        assert_eq!(unpacker.unpack_u8().unwrap(), 0x34);
        assert!(unpacker.unpack_u8().is_err());
    }

    #[test]
    fn test_unpacker_u16() {
        let buffer = [0x12, 0x34];
        let mut unpacker = Unpacker::new(&buffer);
        assert_eq!(unpacker.unpack_u16().unwrap(), 0x1234);
        assert_eq!(unpacker.remaining(), 0);
    }

    #[test]
    fn test_unpacker_u32() {
        let buffer = [0x12, 0x34, 0x56, 0x78];
        let mut unpacker = Unpacker::new(&buffer);
        assert_eq!(unpacker.unpack_u32().unwrap(), 0x12345678);
        assert_eq!(unpacker.remaining(), 0);
    }

    #[test]
    fn test_unpacker_i32() {
        let buffer = [0xFF, 0xFF, 0xFF, 0xFF];
        let mut unpacker = Unpacker::new(&buffer);
        assert_eq!(unpacker.unpack_i32().unwrap(), -1);
    }

    #[test]
    fn test_unpacker_array() {
        let buffer = [1, 2, 3, 4, 5];
        let mut unpacker = Unpacker::new(&buffer);
        let arr = unpacker.unpack_array(3).unwrap();
        assert_eq!(arr, &[1, 2, 3]);
        assert_eq!(unpacker.remaining(), 2);
    }

    #[test]
    fn test_unpacker_remaining() {
        let buffer = [1, 2, 3, 4];
        let mut unpacker = Unpacker::new(&buffer);
        assert_eq!(unpacker.remaining(), 4);
        unpacker.unpack_u8().unwrap();
        assert_eq!(unpacker.remaining(), 3);
        unpacker.unpack_u16().unwrap();
        assert_eq!(unpacker.remaining(), 1);
    }

    #[test]
    fn test_roundtrip() {
        let mut buffer = [0u8; 16];
        {
            let mut packer = Packer::new(&mut buffer);
            packer.pack_u8(0x12).unwrap();
            packer.pack_u16(0x3456).unwrap();
            packer.pack_u32(0x789ABCDE).unwrap();
        }

        let mut unpacker = Unpacker::new(&buffer[..7]);
        assert_eq!(unpacker.unpack_u8().unwrap(), 0x12);
        assert_eq!(unpacker.unpack_u16().unwrap(), 0x3456);
        assert_eq!(unpacker.unpack_u32().unwrap(), 0x789ABCDE);
    }

    #[test]
    fn test_dyn_value() {
        let v = DynValue::U8(42);
        assert_eq!(v.as_u8(), Some(42));
        assert_eq!(v.as_u16(), None);

        let v = DynValue::I32(-100);
        assert_eq!(v.as_i32(), Some(-100));

        let data = [1u8, 2, 3];
        let v = DynValue::Array {
            ptr: data.as_ptr(),
            len: 3,
        };
        let (ptr, len) = v.as_array().unwrap();
        assert_eq!(len, 3);
        assert_eq!(unsafe { core::slice::from_raw_parts(ptr, len) }, &[1, 2, 3]);
    }

    #[test]
    fn test_packer_f32() {
        let mut buffer = [0u8; 16];
        let mut packer = Packer::new(&mut buffer);
        packer.pack_f32(3.14).unwrap();
        assert_eq!(packer.len(), 4);
    }

    #[test]
    fn test_unpacker_f32() {
        let mut buffer = [0u8; 16];
        {
            let mut packer = Packer::new(&mut buffer);
            packer.pack_f32(3.14).unwrap();
        }
        let mut unpacker = Unpacker::new(&buffer[..4]);
        let val = unpacker.unpack_f32().unwrap();
        assert!((val - 3.14).abs() < 0.0001);
    }
}
