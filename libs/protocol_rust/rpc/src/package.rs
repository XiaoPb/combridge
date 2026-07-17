//! Data Package/Unpackage
//!
//! 支持的格式字符串：
//! - <u8>, <u16>, <u32>, <u64>: 无符号整数
//! - <i8>, <i16>, <i32>, <i64>: 有符号整数
//! - <f64>: 双精度浮点
//! - <u8*>, <u16*>, <u32*>: 数组类型
//! - <d8>, <d16>, <d32>: 有符号整数（d表示signed）

use crate::error::RpcError;
use crate::types::ProPackType;

const MAX_PARAM_NUMBER: usize = 10;
const MAX_SUPPORT_FORMAT_LENGTH: usize = 50;

#[derive(Debug, Clone, Copy, Default)]
pub struct TypeHeader {
    pub pack_type: u8,
    pub is_array: bool,
    pub width: u8,
    pub end: bool,
    pub split: bool,
}

impl TypeHeader {
    pub fn to_byte(&self) -> u8 {
        let mut byte = self.pack_type & 0x03;
        if self.is_array {
            byte |= 0x04;
        }
        byte |= (self.width & 0x07) << 3;
        if self.end {
            byte |= 0x40;
        }
        if self.split {
            byte |= 0x80;
        }
        byte
    }

    pub fn from_byte(byte: u8) -> Self {
        Self {
            pack_type: byte & 0x03,
            is_array: (byte & 0x04) != 0,
            width: (byte >> 3) & 0x07,
            end: (byte & 0x40) != 0,
            split: (byte & 0x80) != 0,
        }
    }

    pub fn head_type(&self) -> u8 {
        if self.pack_type == ProPackType::Pack as u8 {
            return HEAD_ERROR;
        }
        if self.is_array {
            return HEAD_ARRAY;
        }
        HEAD_DATA
    }
}

const HEAD_DATA: u8 = 0;
const HEAD_ARRAY: u8 = 1;
const HEAD_ERROR: u8 = 2;

#[derive(Debug, Clone, Default)]
pub struct FormatInfo {
    pub headers: Vec<TypeHeader>,
    pub data_size: usize,
    pub array_num: usize,
}

impl FormatInfo {
    pub fn parse(format: &str) -> Result<Self, RpcError> {
        let mut info = Self::default();
        let format_bytes = format.as_bytes();

        if format_bytes.len() > MAX_SUPPORT_FORMAT_LENGTH {
            return Err(RpcError::FormatError);
        }

        let mut i = 0;
        while i < format_bytes.len() {
            while i < format_bytes.len() && format_bytes[i] != b'<' {
                i += 1;
            }
            if i >= format_bytes.len() {
                break;
            }
            i += 1;

            if info.headers.len() >= MAX_PARAM_NUMBER {
                return Err(RpcError::ParamTooMuch);
            }

            let start = i;
            while i < format_bytes.len() && format_bytes[i] != b'>' {
                i += 1;
            }
            if i >= format_bytes.len() {
                return Err(RpcError::FormatError);
            }

            let token = &format[start..i];
            let token_bytes = token.as_bytes();

            if token_bytes.len() < 2 {
                return Err(RpcError::FormatError);
            }

            let pack_type = match token_bytes[0] {
                b'u' => ProPackType::Unsigned as u8,
                b'f' => ProPackType::Double as u8,
                b'd' | b'i' => ProPackType::Signed as u8,
                _ => return Err(RpcError::FormatError),
            };

            let (is_array, width_str) = if token_bytes[token_bytes.len() - 1] == b'*' {
                (true, &token[1..token.len() - 1])
            } else {
                (false, &token[1..])
            };

            let width_bits: u8 = width_str.parse().map_err(|_| RpcError::FormatError)?;
            let width = width_bits.ilog2() as u8;

            let header = TypeHeader {
                pack_type,
                is_array,
                width,
                end: false,
                split: false,
            };

            if !is_array {
                info.data_size += (1 << width) as usize / 8;
            } else {
                info.array_num += 1;
            }

            info.headers.push(header);
            i += 1;
        }

        if !info.headers.is_empty() {
            let last_idx = info.headers.len() - 1;
            info.headers[last_idx].end = true;
        }

        Ok(info)
    }
}

pub struct Package;

impl Package {
    pub fn pack_u8(data: u8) -> Vec<u8> {
        vec![data]
    }

    pub fn pack_u16(data: u16) -> Vec<u8> {
        data.to_le_bytes().to_vec()
    }

    pub fn pack_u32(data: u32) -> Vec<u8> {
        data.to_le_bytes().to_vec()
    }

    pub fn pack_u64(data: u64) -> Vec<u8> {
        data.to_le_bytes().to_vec()
    }

    pub fn pack_i8(data: i8) -> Vec<u8> {
        vec![data as u8]
    }

    pub fn pack_i16(data: i16) -> Vec<u8> {
        data.to_le_bytes().to_vec()
    }

    pub fn pack_i32(data: i32) -> Vec<u8> {
        data.to_le_bytes().to_vec()
    }

    pub fn pack_i64(data: i64) -> Vec<u8> {
        data.to_le_bytes().to_vec()
    }

    pub fn pack_f64(data: f64) -> Vec<u8> {
        data.to_le_bytes().to_vec()
    }

    pub fn pack_u8_array(data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(data.len() + 2);
        result.extend_from_slice(&(data.len() as u16).to_le_bytes());
        result.extend_from_slice(data);
        result
    }

    pub fn pack_u16_array(data: &[u16]) -> Vec<u8> {
        let mut result = Vec::with_capacity(data.len() * 2 + 2);
        result.extend_from_slice(&(data.len() as u16).to_le_bytes());
        for &v in data {
            result.extend_from_slice(&v.to_le_bytes());
        }
        result
    }

    pub fn pack_u32_array(data: &[u32]) -> Vec<u8> {
        let mut result = Vec::with_capacity(data.len() * 4 + 2);
        result.extend_from_slice(&(data.len() as u16).to_le_bytes());
        for &v in data {
            result.extend_from_slice(&v.to_le_bytes());
        }
        result
    }

    pub fn pack_data_with_header(header: &TypeHeader, data: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(data.len() + 1);
        result.push(header.to_byte());
        result.extend_from_slice(data);
        result
    }

    pub fn pack_array_with_header(
        header: &TypeHeader,
        data: &[u8],
        element_width: usize,
    ) -> Vec<u8> {
        let mut result = Vec::new();
        let total_elements = data.len() / element_width;
        let mut remaining = total_elements;
        let mut offset = 0;

        while remaining > 0 {
            let chunk_size = remaining.min(255);
            let is_last = remaining <= 255;

            let mut chunk_header = *header;
            chunk_header.split = !is_last;
            chunk_header.end = header.end && is_last;

            result.push(chunk_header.to_byte());
            result.push(chunk_size as u8);

            let chunk_bytes = chunk_size * element_width;
            result.extend_from_slice(&data[offset..offset + chunk_bytes]);

            offset += chunk_bytes;
            remaining -= chunk_size;
        }

        result
    }

    pub fn pack(format: &str, values: &[u8]) -> Result<Vec<u8>, RpcError> {
        let info = FormatInfo::parse(format)?;
        let mut result = Vec::new();
        let mut value_offset = 0;

        for header in &info.headers {
            let width_bits = (1 << header.width) as usize;
            let width_bytes = width_bits / 8;

            if header.is_array {
                if value_offset + 2 > values.len() {
                    return Err(RpcError::UnpackageError);
                }
                let arr_len =
                    u16::from_le_bytes([values[value_offset], values[value_offset + 1]]) as usize;
                value_offset += 2;

                let arr_bytes = arr_len * width_bytes;
                if value_offset + arr_bytes > values.len() {
                    return Err(RpcError::UnpackageError);
                }

                result.extend(Self::pack_array_with_header(
                    header,
                    &values[value_offset..value_offset + arr_bytes],
                    width_bytes,
                ));
                value_offset += arr_bytes;
            } else {
                if value_offset + width_bytes > values.len() {
                    return Err(RpcError::UnpackageError);
                }
                result.push(header.to_byte());
                result.extend_from_slice(&values[value_offset..value_offset + width_bytes]);
                value_offset += width_bytes;
            }
        }

        Ok(result)
    }
}

pub struct Unpackage;

impl Unpackage {
    pub fn unpack_u8(data: &[u8]) -> Result<u8, RpcError> {
        if data.is_empty() {
            return Err(RpcError::UnpackageError);
        }
        Ok(data[0])
    }

    pub fn unpack_u16(data: &[u8]) -> Result<u16, RpcError> {
        if data.len() < 2 {
            return Err(RpcError::UnpackageError);
        }
        Ok(u16::from_le_bytes([data[0], data[1]]))
    }

    pub fn unpack_u32(data: &[u8]) -> Result<u32, RpcError> {
        if data.len() < 4 {
            return Err(RpcError::UnpackageError);
        }
        Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    pub fn unpack_u64(data: &[u8]) -> Result<u64, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::UnpackageError);
        }
        Ok(u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }

    pub fn unpack_i8(data: &[u8]) -> Result<i8, RpcError> {
        if data.is_empty() {
            return Err(RpcError::UnpackageError);
        }
        Ok(data[0] as i8)
    }

    pub fn unpack_i16(data: &[u8]) -> Result<i16, RpcError> {
        if data.len() < 2 {
            return Err(RpcError::UnpackageError);
        }
        Ok(i16::from_le_bytes([data[0], data[1]]))
    }

    pub fn unpack_i32(data: &[u8]) -> Result<i32, RpcError> {
        if data.len() < 4 {
            return Err(RpcError::UnpackageError);
        }
        Ok(i32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    pub fn unpack_i64(data: &[u8]) -> Result<i64, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::UnpackageError);
        }
        Ok(i64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }

    pub fn unpack_f64(data: &[u8]) -> Result<f64, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::UnpackageError);
        }
        Ok(f64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }

    pub fn unpack_u8_array(data: &[u8]) -> Result<Vec<u8>, RpcError> {
        if data.len() < 2 {
            return Err(RpcError::UnpackageError);
        }
        let len = u16::from_le_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + len {
            return Err(RpcError::UnpackageError);
        }
        Ok(data[2..2 + len].to_vec())
    }

    pub fn unpack_u16_array(data: &[u8]) -> Result<Vec<u16>, RpcError> {
        if data.len() < 2 {
            return Err(RpcError::UnpackageError);
        }
        let len = u16::from_le_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + len * 2 {
            return Err(RpcError::UnpackageError);
        }
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            let offset = 2 + i * 2;
            result.push(u16::from_le_bytes([data[offset], data[offset + 1]]));
        }
        Ok(result)
    }

    pub fn unpack_u32_array(data: &[u8]) -> Result<Vec<u32>, RpcError> {
        if data.len() < 2 {
            return Err(RpcError::UnpackageError);
        }
        let len = u16::from_le_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + len * 4 {
            return Err(RpcError::UnpackageError);
        }
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            let offset = 2 + i * 4;
            result.push(u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]));
        }
        Ok(result)
    }

    pub fn unpack_with_format(data: &[u8], format: &str) -> Result<Vec<u8>, RpcError> {
        let info = FormatInfo::parse(format)?;
        let mut result = Vec::new();
        let mut offset = 0;

        for expected_header in &info.headers {
            if offset >= data.len() {
                return Err(RpcError::UnpackageError);
            }

            let actual_header = TypeHeader::from_byte(data[offset]);
            offset += 1;

            if actual_header.pack_type != expected_header.pack_type
                || actual_header.is_array != expected_header.is_array
                || actual_header.width != expected_header.width
            {
                return Err(RpcError::UnpackageError);
            }

            let width_bits = (1 << actual_header.width) as usize;
            let width_bytes = width_bits / 8;

            if actual_header.is_array {
                let mut total_elements = 0usize;
                let mut arr_data = Vec::new();
                let mut current_header = actual_header;

                loop {
                    if offset >= data.len() {
                        return Err(RpcError::UnpackageError);
                    }
                    let chunk_len = data[offset] as usize;
                    offset += 1;

                    let chunk_bytes = chunk_len * width_bytes;
                    if offset + chunk_bytes > data.len() {
                        return Err(RpcError::UnpackageError);
                    }

                    arr_data.extend_from_slice(&data[offset..offset + chunk_bytes]);
                    offset += chunk_bytes;
                    total_elements += chunk_len;

                    if !current_header.split {
                        break;
                    }

                    if offset >= data.len() {
                        return Err(RpcError::UnpackageError);
                    }
                    current_header = TypeHeader::from_byte(data[offset]);
                    offset += 1;
                }

                result.extend_from_slice(&(total_elements as u16).to_le_bytes());
                result.extend(arr_data);
            } else {
                if offset + width_bytes > data.len() {
                    return Err(RpcError::UnpackageError);
                }
                result.extend_from_slice(&data[offset..offset + width_bytes]);
                offset += width_bytes;
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_u16() {
        let data = 0x1234u16;
        let packed = Package::pack_u16(data);
        assert_eq!(packed, vec![0x34, 0x12]);
    }

    #[test]
    fn test_unpack_u16() {
        let data = vec![0x34, 0x12];
        let unpacked = Unpackage::unpack_u16(&data).unwrap();
        assert_eq!(unpacked, 0x1234u16);
    }

    #[test]
    fn test_pack_u8_array() {
        let data = vec![1, 2, 3, 4, 5];
        let packed = Package::pack_u8_array(&data);
        assert_eq!(packed.len(), 7);
        assert_eq!(&packed[2..], &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_unpack_u8_array() {
        let data = vec![5, 0, 1, 2, 3, 4, 5];
        let unpacked = Unpackage::unpack_u8_array(&data).unwrap();
        assert_eq!(unpacked, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_format_info_parse() {
        let info = FormatInfo::parse("<u16>").unwrap();
        assert_eq!(info.headers.len(), 1);
        assert_eq!(info.headers[0].pack_type, ProPackType::Unsigned as u8);
        assert_eq!(info.headers[0].width, 4);
        assert!(!info.headers[0].is_array);
        assert!(info.headers[0].end);

        let info = FormatInfo::parse("<u16*>").unwrap();
        assert!(info.headers[0].is_array);

        let info = FormatInfo::parse("<d32>").unwrap();
        assert_eq!(info.headers[0].pack_type, ProPackType::Signed as u8);
        assert_eq!(info.headers[0].width, 5);
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        let original = 0xDEADBEEFu32;
        let packed = Package::pack_u32(original);
        let unpacked = Unpackage::unpack_u32(&packed).unwrap();
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_type_header_conversion() {
        let header = TypeHeader {
            pack_type: ProPackType::Unsigned as u8,
            is_array: true,
            width: 2,
            end: true,
            split: false,
        };
        let byte = header.to_byte();
        let restored = TypeHeader::from_byte(byte);
        assert_eq!(header.pack_type, restored.pack_type);
        assert_eq!(header.is_array, restored.is_array);
        assert_eq!(header.width, restored.width);
        assert_eq!(header.end, restored.end);
        assert_eq!(header.split, restored.split);
    }

    #[test]
    fn test_pack_with_format_single() {
        let value = 0x12345678u32;
        let values = value.to_le_bytes().to_vec();
        let packed = Package::pack("<u32>", &values).unwrap();
        assert_eq!(packed.len(), 5);
        assert_eq!(packed[0] & 0x03, ProPackType::Unsigned as u8);
        assert_eq!(&packed[1..], &[0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_unpack_with_format_single() {
        let header = TypeHeader {
            pack_type: ProPackType::Unsigned as u8,
            is_array: false,
            width: 5,
            end: true,
            split: false,
        };
        let mut data = vec![header.to_byte()];
        data.extend_from_slice(&0x12345678u32.to_le_bytes());

        let unpacked = Unpackage::unpack_with_format(&data, "<u32>").unwrap();
        assert_eq!(unpacked, vec![0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_pack_unpack_array_with_header() {
        let header = TypeHeader {
            pack_type: ProPackType::Unsigned as u8,
            is_array: true,
            width: 1,
            end: true,
            split: false,
        };
        let data: Vec<u8> = vec![1, 2, 3, 4, 5];
        let packed = Package::pack_array_with_header(&header, &data, 1);
        assert!(packed.len() > 2);
    }

    #[test]
    fn test_pack_i32() {
        let value = -12345i32;
        let packed = Package::pack_i32(value);
        let unpacked = Unpackage::unpack_i32(&packed).unwrap();
        assert_eq!(value, unpacked);
    }

    #[test]
    fn test_pack_f64() {
        let value = std::f64::consts::PI;
        let packed = Package::pack_f64(value);
        let unpacked = Unpackage::unpack_f64(&packed).unwrap();
        assert!((value - unpacked).abs() < f64::EPSILON);
    }

    #[test]
    fn test_format_info_multiple_params() {
        let info = FormatInfo::parse("<u8><u16><u32>").unwrap();
        assert_eq!(info.headers.len(), 3);
        assert!(info.headers[2].end);
        assert!(!info.headers[0].end);
        assert!(!info.headers[1].end);
    }

    #[test]
    fn test_format_info_error_cases() {
        assert!(FormatInfo::parse("<>").is_err());
        assert!(FormatInfo::parse("<x8>").is_err());
        assert!(FormatInfo::parse("<u8").is_err());
    }
}
