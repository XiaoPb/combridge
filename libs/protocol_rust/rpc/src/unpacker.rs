//! 通用数据解码模块
//!
//! 提供统一的数据解码接口，支持各类指令数据的解码处理。
//! 参考 C 语言中的 GHRPC_unpack 函数实现模式。
//!
//! ## 格式字符串
//!
//! 使用尖括号包围类型名，与 C 端保持一致：
//! - `<u8>` / `<u16>` / `<u32>` / `<u64>` - 无符号整数
//! - `<i8>` / `<i16>` / `<i32>` / `<i64>` - 有符号整数
//! - `<d8>` / `<d16>` / `<d32>` / `<d64>` - 有符号整数（别名）
//! - `<u8*>` / `<u16*>` / `<u32*>` / `<u64*>` - 无符号整数数组
//! - `<i8*>` / `<i16*>` / `<i32*>` / `<i64*>` - 有符号整数数组
//! - `<s>` - 字符串
//!
//! ## 使用示例
//!
//! ```ignore
//! use rpc::unpack;
//!
//! // 解码单个值
//! let value = unpack(&data, "<u8>")?;
//! let value = unpack(&data, "<u16>")?;
//! let value = unpack(&data, "<i64>")?;
//!
//! // 解码数组
//! let arr = unpack(&data, "<u16*>")?;
//! let arr = unpack(&data, "<i32*>")?;
//!
//! // 解码字符串
//! let s = unpack_string(&data);
//! ```

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum UnpackError {
    InsufficientData,
    InvalidHeader,
    InvalidFormat,
    UnsupportedType,
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnpackError::InsufficientData => write!(f, "数据不足"),
            UnpackError::InvalidHeader => write!(f, "无效的头部"),
            UnpackError::InvalidFormat => write!(f, "无效的格式"),
            UnpackError::UnsupportedType => write!(f, "不支持的类型"),
        }
    }
}

impl std::error::Error for UnpackError {}

#[derive(Debug, Clone, PartialEq)]
pub enum UnpackValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    U8Array(Vec<u8>),
    I8Array(Vec<i8>),
    U16Array(Vec<u16>),
    I16Array(Vec<i16>),
    U32Array(Vec<u32>),
    I32Array(Vec<i32>),
    U64Array(Vec<u64>),
    I64Array(Vec<i64>),
    String(String),
}

#[derive(Debug, Clone, Default)]
pub struct DataUnpacker;

impl DataUnpacker {
    pub fn new() -> Self {
        Self
    }

    fn get_element_size(header: u8) -> usize {
        let width = (header >> 3) & 0x07;
        (1 << width) / 8
    }

    fn is_array(header: u8) -> bool {
        (header & 0x04) != 0
    }

    fn parse_format(format: &str) -> Option<(String, bool)> {
        let format = format.trim();
        if !format.starts_with('<') || !format.ends_with('>') {
            return None;
        }

        let inner = &format[1..format.len() - 1];
        let is_array = inner.ends_with('*');
        let type_name = if is_array {
            &inner[..inner.len() - 1]
        } else {
            inner
        };

        Some((type_name.to_lowercase(), is_array))
    }

    pub fn unpack(&self, data: &[u8], format: &str) -> Result<UnpackValue, UnpackError> {
        if data.is_empty() {
            return Err(UnpackError::InsufficientData);
        }

        let (type_name, is_format_array) =
            Self::parse_format(format).ok_or(UnpackError::InvalidFormat)?;

        let header = data[0];
        let is_data_array = Self::is_array(header);
        let element_size = Self::get_element_size(header);
        let is_array = is_data_array || is_format_array;

        match type_name.as_str() {
            "u8" => {
                if is_array {
                    self.unpack_u8_array_internal(data, element_size)
                } else {
                    self.unpack_u8_internal(data, element_size)
                }
            }
            "i8" | "d8" => {
                if is_array {
                    self.unpack_i8_array_internal(data, element_size)
                } else {
                    self.unpack_i8_internal(data, element_size)
                }
            }
            "u16" => {
                if is_array {
                    self.unpack_u16_array_internal(data, element_size)
                } else {
                    self.unpack_u16_internal(data, element_size)
                }
            }
            "i16" | "d16" => {
                if is_array {
                    self.unpack_i16_array_internal(data, element_size)
                } else {
                    self.unpack_i16_internal(data, element_size)
                }
            }
            "u32" => {
                if is_array {
                    self.unpack_u32_array_internal(data, element_size)
                } else {
                    self.unpack_u32_internal(data, element_size)
                }
            }
            "i32" | "d32" => {
                if is_array {
                    self.unpack_i32_array_internal(data, element_size)
                } else {
                    self.unpack_i32_internal(data, element_size)
                }
            }
            "u64" => {
                if is_array {
                    self.unpack_u64_array_internal(data, element_size)
                } else {
                    self.unpack_u64_internal(data, element_size)
                }
            }
            "i64" | "d64" => {
                if is_array {
                    self.unpack_i64_array_internal(data, element_size)
                } else {
                    self.unpack_i64_internal(data, element_size)
                }
            }
            "s" | "string" => {
                let result = self.unpack_u8_array_internal(data, element_size)?;
                if let UnpackValue::U8Array(arr) = result {
                    Ok(UnpackValue::String(
                        String::from_utf8_lossy(&arr)
                            .trim_matches(char::from(0))
                            .to_string(),
                    ))
                } else {
                    Err(UnpackError::InvalidFormat)
                }
            }
            _ => Err(UnpackError::UnsupportedType),
        }
    }

    fn get_array_len(data: &[u8]) -> Result<(usize, usize), UnpackError> {
        if data.len() < 2 {
            return Err(UnpackError::InsufficientData);
        }
        let array_len = data[1] as usize;
        let start = 2;
        Ok((array_len, start))
    }

    fn unpack_u8_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        Ok(UnpackValue::U8(data[1]))
    }

    fn unpack_u8_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            return Ok(UnpackValue::U8Array(vec![data[1]]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let end = start + array_len * element_size;

        if end > data.len() {
            return Ok(UnpackValue::U8Array(data[start..].to_vec()));
        }

        Ok(UnpackValue::U8Array(data[start..end].to_vec()))
    }

    fn unpack_i8_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        Ok(UnpackValue::I8(data[1] as i8))
    }

    fn unpack_i8_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            return Ok(UnpackValue::I8Array(vec![data[1] as i8]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let mut result = Vec::with_capacity(array_len);

        for i in 0..array_len {
            let offset = start + i * element_size;
            if offset >= data.len() {
                break;
            }
            result.push(data[offset] as i8);
        }

        Ok(UnpackValue::I8Array(result))
    }

    fn unpack_u16_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        Ok(UnpackValue::U16(u16::from_le_bytes([data[1], data[2]])))
    }

    fn unpack_u16_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            return Ok(UnpackValue::U16Array(vec![u16::from_le_bytes([
                data[1], data[2],
            ])]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let mut result = Vec::with_capacity(array_len);

        for i in 0..array_len {
            let offset = start + i * element_size;
            if offset + 2 > data.len() {
                break;
            }
            result.push(u16::from_le_bytes([data[offset], data[offset + 1]]));
        }

        Ok(UnpackValue::U16Array(result))
    }

    fn unpack_i16_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        Ok(UnpackValue::I16(i16::from_le_bytes([data[1], data[2]])))
    }

    fn unpack_i16_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            return Ok(UnpackValue::I16Array(vec![i16::from_le_bytes([
                data[1], data[2],
            ])]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let mut result = Vec::with_capacity(array_len);

        for i in 0..array_len {
            let offset = start + i * element_size;
            if offset + 2 > data.len() {
                break;
            }
            result.push(i16::from_le_bytes([data[offset], data[offset + 1]]));
        }

        Ok(UnpackValue::I16Array(result))
    }

    fn unpack_u32_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        let bytes: [u8; 4] = data[1..5]
            .try_into()
            .map_err(|_| UnpackError::InsufficientData)?;
        Ok(UnpackValue::U32(u32::from_le_bytes(bytes)))
    }

    fn unpack_u32_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            let bytes: [u8; 4] = data[1..5]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            return Ok(UnpackValue::U32Array(vec![u32::from_le_bytes(bytes)]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let mut result = Vec::with_capacity(array_len);

        for i in 0..array_len {
            let offset = start + i * element_size;
            if offset + 4 > data.len() {
                break;
            }
            let bytes: [u8; 4] = data[offset..offset + 4]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            result.push(u32::from_le_bytes(bytes));
        }

        Ok(UnpackValue::U32Array(result))
    }

    fn unpack_i32_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        let bytes: [u8; 4] = data[1..5]
            .try_into()
            .map_err(|_| UnpackError::InsufficientData)?;
        Ok(UnpackValue::I32(i32::from_le_bytes(bytes)))
    }

    fn unpack_i32_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            let bytes: [u8; 4] = data[1..5]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            return Ok(UnpackValue::I32Array(vec![i32::from_le_bytes(bytes)]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let mut result = Vec::with_capacity(array_len);

        for i in 0..array_len {
            let offset = start + i * element_size;
            if offset + 4 > data.len() {
                break;
            }
            let bytes: [u8; 4] = data[offset..offset + 4]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            result.push(i32::from_le_bytes(bytes));
        }

        Ok(UnpackValue::I32Array(result))
    }

    fn unpack_u64_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        let bytes: [u8; 8] = data[1..9]
            .try_into()
            .map_err(|_| UnpackError::InsufficientData)?;
        Ok(UnpackValue::U64(u64::from_le_bytes(bytes)))
    }

    fn unpack_u64_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            let bytes: [u8; 8] = data[1..9]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            return Ok(UnpackValue::U64Array(vec![u64::from_le_bytes(bytes)]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let mut result = Vec::with_capacity(array_len);

        for i in 0..array_len {
            let offset = start + i * element_size;
            if offset + 8 > data.len() {
                break;
            }
            let bytes: [u8; 8] = data[offset..offset + 8]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            result.push(u64::from_le_bytes(bytes));
        }

        Ok(UnpackValue::U64Array(result))
    }

    fn unpack_i64_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        if data.len() < 1 + element_size {
            return Err(UnpackError::InsufficientData);
        }
        let bytes: [u8; 8] = data[1..9]
            .try_into()
            .map_err(|_| UnpackError::InsufficientData)?;
        Ok(UnpackValue::I64(i64::from_le_bytes(bytes)))
    }

    fn unpack_i64_array_internal(
        &self,
        data: &[u8],
        element_size: usize,
    ) -> Result<UnpackValue, UnpackError> {
        let header = data[0];
        if !Self::is_array(header) {
            if data.len() < 1 + element_size {
                return Err(UnpackError::InsufficientData);
            }
            let bytes: [u8; 8] = data[1..9]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            return Ok(UnpackValue::I64Array(vec![i64::from_le_bytes(bytes)]));
        }

        let (array_len, start) = Self::get_array_len(data)?;
        let mut result = Vec::with_capacity(array_len);

        for i in 0..array_len {
            let offset = start + i * element_size;
            if offset + 8 > data.len() {
                break;
            }
            let bytes: [u8; 8] = data[offset..offset + 8]
                .try_into()
                .map_err(|_| UnpackError::InsufficientData)?;
            result.push(i64::from_le_bytes(bytes));
        }

        Ok(UnpackValue::I64Array(result))
    }
}

pub fn unpack(data: &[u8], format: &str) -> Result<UnpackValue, UnpackError> {
    DataUnpacker::new().unpack(data, format)
}

pub fn unpack_u8_array(data: &[u8]) -> Vec<u8> {
    match unpack(data, "<u8*>") {
        Ok(UnpackValue::U8Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_u16_array(data: &[u8]) -> Vec<u16> {
    match unpack(data, "<u16*>") {
        Ok(UnpackValue::U16Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_u32_array(data: &[u8]) -> Vec<u32> {
    match unpack(data, "<u32*>") {
        Ok(UnpackValue::U32Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_u64_array(data: &[u8]) -> Vec<u64> {
    match unpack(data, "<u64*>") {
        Ok(UnpackValue::U64Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_i8_array(data: &[u8]) -> Vec<i8> {
    match unpack(data, "<i8*>") {
        Ok(UnpackValue::I8Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_i16_array(data: &[u8]) -> Vec<i16> {
    match unpack(data, "<i16*>") {
        Ok(UnpackValue::I16Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_i32_array(data: &[u8]) -> Vec<i32> {
    match unpack(data, "<i32*>") {
        Ok(UnpackValue::I32Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_i64_array(data: &[u8]) -> Vec<i64> {
    match unpack(data, "<i64*>") {
        Ok(UnpackValue::I64Array(arr)) => arr,
        _ => Vec::new(),
    }
}

pub fn unpack_string(data: &[u8]) -> String {
    match unpack(data, "<s>") {
        Ok(UnpackValue::String(s)) => s,
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpack_u8() {
        let data = [0x58, 0x42];
        let result = unpack(&data, "<u8>").unwrap();
        assert_eq!(result, UnpackValue::U8(0x42));
    }

    #[test]
    fn test_unpack_u8_array() {
        let data = [0x5C, 0x03, 0x01, 0x02, 0x03];
        let result = unpack(&data, "<u8*>").unwrap();
        assert_eq!(result, UnpackValue::U8Array(vec![0x01, 0x02, 0x03]));
    }

    #[test]
    fn test_unpack_i8() {
        let data = [0x59, 0xFF];
        let result = unpack(&data, "<i8>").unwrap();
        assert_eq!(result, UnpackValue::I8(-1));
    }

    #[test]
    fn test_unpack_i8_array() {
        let data = [0x5D, 0x02, 0xFF, 0x7F];
        let result = unpack(&data, "<i8*>").unwrap();
        assert_eq!(result, UnpackValue::I8Array(vec![-1, 127]));
    }

    #[test]
    fn test_unpack_u16() {
        let data = [0x60, 0x34, 0x12];
        let result = unpack(&data, "<u16>").unwrap();
        assert_eq!(result, UnpackValue::U16(0x1234));
    }

    #[test]
    fn test_unpack_u16_array() {
        let data = [0x64, 0x02, 0x34, 0x12, 0x78, 0x56];
        let result = unpack(&data, "<u16*>").unwrap();
        assert_eq!(result, UnpackValue::U16Array(vec![0x1234, 0x5678]));
    }

    #[test]
    fn test_unpack_i16() {
        let data = [0x61, 0xFF, 0xFF];
        let result = unpack(&data, "<i16>").unwrap();
        assert_eq!(result, UnpackValue::I16(-1));
    }

    #[test]
    fn test_unpack_i16_array() {
        let data = [0x65, 0x02, 0xFF, 0xFF, 0x00, 0x80];
        let result = unpack(&data, "<i16*>").unwrap();
        assert_eq!(result, UnpackValue::I16Array(vec![-1, -32768]));
    }

    #[test]
    fn test_unpack_u32() {
        let data = [0x68, 0x78, 0x56, 0x34, 0x12];
        let result = unpack(&data, "<u32>").unwrap();
        assert_eq!(result, UnpackValue::U32(0x12345678));
    }

    #[test]
    fn test_unpack_u32_array() {
        let data = [0x6C, 0x02, 0x78, 0x56, 0x34, 0x12, 0xEF, 0xCD, 0xAB, 0x90];
        let result = unpack(&data, "<u32*>").unwrap();
        assert_eq!(result, UnpackValue::U32Array(vec![0x12345678, 0x90ABCDEF]));
    }

    #[test]
    fn test_unpack_i32() {
        let data = [0x69, 0x78, 0x56, 0x34, 0x12];
        let result = unpack(&data, "<i32>").unwrap();
        assert_eq!(result, UnpackValue::I32(0x12345678));
    }

    #[test]
    fn test_unpack_d32() {
        let data = [0x69, 0x78, 0x56, 0x34, 0x12];
        let result = unpack(&data, "<d32>").unwrap();
        assert_eq!(result, UnpackValue::I32(0x12345678));
    }

    #[test]
    fn test_unpack_i32_array() {
        let data = [0x6D, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x80];
        let result = unpack(&data, "<i32*>").unwrap();
        assert_eq!(result, UnpackValue::I32Array(vec![-1, -2147483648]));
    }

    #[test]
    fn test_unpack_u64() {
        let data = [0x70, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12];
        let result = unpack(&data, "<u64>").unwrap();
        assert_eq!(result, UnpackValue::U64(0x1234567890ABCDEF));
    }

    #[test]
    fn test_unpack_u64_array() {
        let data = [
            0x74, 0x02, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let result = unpack(&data, "<u64*>").unwrap();
        assert_eq!(
            result,
            UnpackValue::U64Array(vec![0x1234567890ABCDEF, 0xFFFFFFFFFFFFFFFF])
        );
    }

    #[test]
    fn test_unpack_i64() {
        let data = [0x71, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let result = unpack(&data, "<i64>").unwrap();
        assert_eq!(result, UnpackValue::I64(-1));
    }

    #[test]
    fn test_unpack_d64() {
        let data = [0x71, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let result = unpack(&data, "<d64>").unwrap();
        assert_eq!(result, UnpackValue::I64(-1));
    }

    #[test]
    fn test_unpack_i64_array() {
        let data = [
            0x75, 0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x80,
        ];
        let result = unpack(&data, "<i64*>").unwrap();
        assert_eq!(
            result,
            UnpackValue::I64Array(vec![-1, -9223372036854775808])
        );
    }

    #[test]
    fn test_unpack_string() {
        let data = [0x5C, 0x05, b'H', b'e', b'l', b'l', b'o'];
        let result = unpack(&data, "<s>").unwrap();
        assert_eq!(result, UnpackValue::String("Hello".to_string()));
    }

    #[test]
    fn test_unpack_string_function() {
        let data = [0x5C, 0x05, b'T', b'e', b's', b't', b'\0'];
        let s = unpack_string(&data);
        assert_eq!(s, "Test");
    }

    #[test]
    fn test_unpack_u16_array_function() {
        let data = [0x64, 0x02, 0x34, 0x12, 0x78, 0x56];
        let arr = unpack_u16_array(&data);
        assert_eq!(arr, vec![0x1234, 0x5678]);
    }

    #[test]
    fn test_invalid_format() {
        let data = [0x58, 0x42];
        let result = unpack(&data, "u8");
        assert!(result.is_err());
    }
}
