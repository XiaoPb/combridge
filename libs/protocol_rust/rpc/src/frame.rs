//! Frame Parser
//! 
//! 帧格式：
//! +----------+--------+---------+----------+-------+----------+--------+-----+
//! | Header   | Length | TypeKey | KeyData  | ComID | FrameID  | Param  | CRC |
//! | 2 bytes  | 1 byte | 1 byte  | N bytes  | 1 byte| 1 byte   | N bytes|1byte|
//! +----------+--------+---------+----------+-------+----------+--------+-----+

use crate::error::RpcError;
use crate::types::{FRAME_HEADER, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE, TypeKey, FrameIndex};

const LAST_FRAME_FIX_INDEX: u8 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseState {
    FrameHeader,
    CheckLength,
    CheckTypeKey,
    CheckKey,
    CheckIndex,
    CheckParam,
    CheckCrc,
}

impl Default for ParseState {
    fn default() -> Self {
        Self::FrameHeader
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    pub key: String,
    pub param: Vec<u8>,
    pub is_secure: bool,
    pub is_fin: bool,
    pub invoke_idx: u8,
    pub frame_idx: u8,
}

#[derive(Debug)]
pub struct FrameParser {
    state: ParseState,
    frame_len: usize,
    type_key: TypeKey,
    key_data: Vec<u8>,
    key_expected_len: usize,
    frame_index: FrameIndex,
    param_data: Vec<u8>,
    crc: u8,
    header_pos: usize,
    index_state: IndexState,
}

#[derive(Debug, Clone, Copy, Default)]
enum IndexState {
    #[default]
    First,
    Second,
}

impl Default for FrameParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::FrameHeader,
            frame_len: 0,
            type_key: TypeKey::default(),
            key_data: Vec::new(),
            key_expected_len: 0,
            frame_index: FrameIndex::default(),
            param_data: Vec::new(),
            crc: 0,
            header_pos: 0,
            index_state: IndexState::default(),
        }
    }
    
    pub fn reset(&mut self) {
        self.state = ParseState::FrameHeader;
        self.frame_len = 0;
        self.type_key = TypeKey::default();
        self.key_data.clear();
        self.key_expected_len = 0;
        self.frame_index = FrameIndex::default();
        self.param_data.clear();
        self.crc = 0;
        self.header_pos = 0;
        self.index_state = IndexState::default();
    }
    
    pub fn process(&mut self, data: &[u8]) -> Vec<Result<ParseResult, RpcError>> {
        let mut results = Vec::new();
        
        for &byte in data {
            match self.process_byte(byte) {
                Ok(Some(result)) => results.push(Ok(result)),
                Ok(None) => {}
                Err(e) => {
                    self.reset();
                    results.push(Err(e));
                }
            }
        }
        
        results
    }
    
    fn process_byte(&mut self, byte: u8) -> Result<Option<ParseResult>, RpcError> {
        match self.state {
            ParseState::FrameHeader => {
                self.process_frame_header(byte);
            }
            ParseState::CheckLength => {
                self.frame_len = byte as usize;
                self.state = ParseState::CheckTypeKey;
            }
            ParseState::CheckTypeKey => {
                self.type_key = TypeKey::from_byte(byte);
                self.crc = byte;
                self.frame_len = self.frame_len.saturating_sub(1);
                
                if self.type_key.is_array {
                    self.key_expected_len = 0;
                } else {
                    self.key_expected_len = 1;
                }
                
                self.state = ParseState::CheckKey;
            }
            ParseState::CheckKey => {
                self.process_key(byte)?;
            }
            ParseState::CheckIndex => {
                self.process_index(byte)?;
            }
            ParseState::CheckParam => {
                return self.process_param(byte);
            }
            ParseState::CheckCrc => {
                return self.process_crc(byte);
            }
        }
        
        Ok(None)
    }
    
    fn process_frame_header(&mut self, byte: u8) {
        if byte == FRAME_HEADER[self.header_pos] {
            self.header_pos += 1;
            if self.header_pos >= FRAME_HEADER.len() {
                self.state = ParseState::CheckLength;
                self.header_pos = 0;
            }
        } else {
            self.header_pos = if byte == FRAME_HEADER[0] { 1 } else { 0 };
        }
    }
    
    fn process_key(&mut self, byte: u8) -> Result<(), RpcError> {
        self.crc = self.crc.wrapping_add(byte);
        self.frame_len = self.frame_len.saturating_sub(1);
        
        if self.type_key.is_array {
            if self.key_data.is_empty() {
                if byte as usize > MAX_SUPPORT_KEY_SIZE - 1 {
                    return Err(RpcError::KeyOverMaxSize);
                }
                self.key_expected_len = byte as usize;
                self.key_data.push(byte);
            } else {
                self.key_data.push(byte);
                if self.key_data.len() > self.key_expected_len {
                    self.transition_after_key();
                }
            }
        } else {
            self.key_data.push(byte);
            self.transition_after_key();
        }
        
        Ok(())
    }
    
    fn transition_after_key(&mut self) {
        let check = (self.type_key.secure as u8) << 1 | self.type_key.fin as u8;
        if check == 1 {
            self.frame_index.frame_idx = LAST_FRAME_FIX_INDEX;
            if self.frame_len == 0 {
                self.state = ParseState::CheckCrc;
            } else {
                self.state = ParseState::CheckParam;
            }
        } else {
            self.state = ParseState::CheckIndex;
        }
    }
    
    fn process_index(&mut self, byte: u8) -> Result<(), RpcError> {
        let check = (self.type_key.secure as u8) << 1 | self.type_key.fin as u8;
        
        match check {
            0 => {
                self.frame_index.frame_idx = byte;
                self.crc = self.crc.wrapping_add(byte);
                self.frame_len = self.frame_len.saturating_sub(1);
                self.state = ParseState::CheckParam;
            }
            1 => {
                self.frame_index.frame_idx = LAST_FRAME_FIX_INDEX;
                self.state = ParseState::CheckParam;
                return Ok(());
            }
            2 => {
                match self.index_state {
                    IndexState::First => {
                        self.frame_index.invoke_idx = byte;
                        self.crc = self.crc.wrapping_add(byte);
                        self.frame_len = self.frame_len.saturating_sub(1);
                        self.index_state = IndexState::Second;
                        return Ok(());
                    }
                    IndexState::Second => {
                        self.frame_index.frame_idx = byte;
                        self.crc = self.crc.wrapping_add(byte);
                        self.frame_len = self.frame_len.saturating_sub(1);
                        self.state = ParseState::CheckParam;
                    }
                }
            }
            3 => {
                self.frame_index.invoke_idx = byte;
                self.frame_index.frame_idx = LAST_FRAME_FIX_INDEX;
                self.crc = self.crc.wrapping_add(byte);
                self.frame_len = self.frame_len.saturating_sub(1);
                self.state = ParseState::CheckParam;
            }
            _ => return Err(RpcError::FormatError),
        }
        
        Ok(())
    }
    
    fn process_param(&mut self, byte: u8) -> Result<Option<ParseResult>, RpcError> {
        if self.frame_len == 0 {
            return self.process_crc(byte);
        }
        
        self.param_data.push(byte);
        self.crc = self.crc.wrapping_add(byte);
        self.frame_len = self.frame_len.saturating_sub(1);
        
        if self.frame_len == 0 {
            self.state = ParseState::CheckCrc;
        }
        
        Ok(None)
    }
    
    fn process_crc(&mut self, byte: u8) -> Result<Option<ParseResult>, RpcError> {
        if byte != self.crc {
            return Err(RpcError::CrcMismatch);
        }
        
        let key = if self.type_key.is_array && self.key_data.len() > 1 {
            String::from_utf8_lossy(&self.key_data[1..]).to_string()
        } else if !self.key_data.is_empty() {
            String::from_utf8_lossy(&self.key_data).to_string()
        } else {
            String::new()
        };
        
        let result = ParseResult {
            key,
            param: self.param_data.clone(),
            is_secure: self.type_key.secure,
            is_fin: self.type_key.fin,
            invoke_idx: self.frame_index.invoke_idx,
            frame_idx: self.frame_index.frame_idx,
        };
        
        self.reset();
        Ok(Some(result))
    }
}

/// 计算CRC校验值（累加和）
pub fn calculate_crc(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        crc = crc.wrapping_add(byte);
    }
    crc
}

/// 帧构建器
#[derive(Debug)]
pub struct FrameBuilder {
    buffer: Vec<u8>,
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameBuilder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(GHRPC_FRAME_SIZE),
        }
    }

    pub fn build_frame(
        &mut self,
        key: &str,
        param: &[u8],
        secure: bool,
        fin: bool,
        invoke_idx: u8,
        frame_idx: u8,
    ) -> Vec<u8> {
        self.buffer.clear();

        self.buffer.extend_from_slice(&FRAME_HEADER);

        let length_pos = self.buffer.len();
        self.buffer.push(0);

        let type_key = TypeKey {
            pack_type: crate::types::ProPackType::Signed as u8,
            is_array: key.len() > 1,
            width: 3,
            secure,
            fin,
        };
        self.buffer.push(type_key.to_byte());

        if key.len() > 1 {
            self.buffer.push(key.len() as u8);
            self.buffer.extend_from_slice(key.as_bytes());
        } else if !key.is_empty() {
            self.buffer.push(key.as_bytes()[0]);
        }

        if secure {
            self.buffer.push(invoke_idx);
        }
        if !fin {
            self.buffer.push(frame_idx);
        }

        self.buffer.extend_from_slice(param);

        let length = (self.buffer.len() - length_pos - 1) as u8;
        self.buffer[length_pos] = length;

        let crc = calculate_crc(&self.buffer[length_pos + 1..]);
        self.buffer.push(crc);

        self.buffer.clone()
    }

    pub fn calculate_max_payload(key: &str, secure: bool, fin: bool) -> usize {
        let mut overhead = FRAME_HEADER.len() + 1;

        overhead += 1;

        if key.len() > 1 {
            overhead += 1 + key.len();
        } else {
            overhead += 1;
        }

        if secure {
            overhead += 1;
        }
        if !fin {
            overhead += 1;
        }

        overhead += 1;

        GHRPC_FRAME_SIZE.saturating_sub(overhead)
    }

    pub fn build_frames(
        &mut self,
        key: &str,
        data: &[u8],
        secure: bool,
    ) -> Vec<Vec<u8>> {
        self.build_frames_with_invoke_idx(key, data, secure, 0)
    }

    pub fn build_frames_with_invoke_idx(
        &mut self,
        key: &str,
        data: &[u8],
        secure: bool,
        invoke_idx: u8,
    ) -> Vec<Vec<u8>> {
        let max_payload = Self::calculate_max_payload(key, secure, false);

        if data.len() <= max_payload {
            let frame = self.build_frame(key, data, secure, true, invoke_idx, 0);
            return vec![frame];
        }

        let mut frames = Vec::new();
        let mut offset = 0;
        let mut frame_idx: u8 = 0;

        while offset < data.len() {
            let remaining = data.len() - offset;
            let is_fin = remaining <= max_payload;
            let chunk_size = if is_fin { remaining } else { max_payload };

            let frame = self.build_frame(
                key,
                &data[offset..offset + chunk_size],
                secure,
                is_fin,
                invoke_idx,
                frame_idx,
            );
            frames.push(frame);

            offset += chunk_size;
            if !is_fin {
                frame_idx = frame_idx.wrapping_add(1);
            }
        }

        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_crc_calculation() {
        let data = [0x01, 0x02, 0x03];
        let crc = calculate_crc(&data);
        assert_eq!(crc, 0x06);
    }
    
    #[test]
    fn test_frame_parser_single_char_key() {
        let mut parser = FrameParser::new();
        
        let mut frame = vec![0xAA, 0x11];
        let type_key = 0x80u8;
        let key = b'G';
        let length = (1 + 1) as u8;
        
        frame.push(length);
        frame.push(type_key);
        frame.push(key);
        
        let crc = calculate_crc(&[type_key, key]);
        frame.push(crc);
        
        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        
        let result = results[0].as_ref().unwrap();
        assert_eq!(result.key, "G");
        assert!(result.is_fin);
        assert!(!result.is_secure);
    }
    
    #[test]
    fn test_frame_parser_multi_char_key() {
        let mut parser = FrameParser::new();
        
        let mut frame = vec![0xAA, 0x11];
        let key_str = "Test";
        let type_key = 0x84u8;
        let length = (1 + 1 + key_str.len()) as u8;
        
        frame.push(length);
        frame.push(type_key);
        frame.push(key_str.len() as u8);
        frame.extend_from_slice(key_str.as_bytes());
        
        let mut crc_data = vec![type_key, key_str.len() as u8];
        crc_data.extend_from_slice(key_str.as_bytes());
        let crc = calculate_crc(&crc_data);
        frame.push(crc);
        
        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        
        let result = results[0].as_ref().unwrap();
        assert_eq!(result.key, "Test");
        assert!(result.is_fin);
    }
    
    #[test]
    fn test_frame_parser_with_param() {
        let mut parser = FrameParser::new();
        
        let mut frame = vec![0xAA, 0x11];
        let key = b'G';
        let type_key = 0x80u8;
        let param = [0x01, 0x02, 0x03];
        let length = (1 + 1 + param.len()) as u8;
        
        frame.push(length);
        frame.push(type_key);
        frame.push(key);
        frame.extend_from_slice(&param);
        
        let mut crc_data = vec![type_key, key];
        crc_data.extend_from_slice(&param);
        let crc = calculate_crc(&crc_data);
        frame.push(crc);
        
        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        
        let result = results[0].as_ref().unwrap();
        assert_eq!(result.key, "G");
        assert_eq!(result.param, param);
    }
    
    #[test]
    fn test_frame_parser_crc_error() {
        let mut parser = FrameParser::new();
        
        let mut frame = vec![0xAA, 0x11];
        let type_key = 0x80u8;
        let key = b'G';
        let length = (1 + 1) as u8;
        
        frame.push(length);
        frame.push(type_key);
        frame.push(key);
        frame.push(0xFF);
        
        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }
    
    #[test]
    fn test_parse_state_default() {
        assert_eq!(ParseState::default(), ParseState::FrameHeader);
    }
    
    #[test]
    fn test_frame_parser_reset() {
        let mut parser = FrameParser::new();
        parser.state = ParseState::CheckLength;
        parser.frame_len = 100;
        parser.crc = 0x55;
        parser.reset();
        
        assert_eq!(parser.state, ParseState::FrameHeader);
        assert_eq!(parser.frame_len, 0);
        assert_eq!(parser.crc, 0);
        assert!(parser.key_data.is_empty());
        assert!(parser.param_data.is_empty());
    }
    
    #[test]
    fn test_frame_parser_secure_unfin() {
        let mut parser = FrameParser::new();
        
        let mut frame = vec![0xAA, 0x11];
        let key = b'G';
        let type_key = 0x40u8;
        let invoke_idx = 0x05u8;
        let frame_idx = 0x0Au8;
        let param = [0x01, 0x02];
        let length = (1 + 1 + 1 + 1 + param.len()) as u8;
        
        frame.push(length);
        frame.push(type_key);
        frame.push(key);
        frame.push(invoke_idx);
        frame.push(frame_idx);
        frame.extend_from_slice(&param);
        
        let mut crc_data = vec![type_key, key, invoke_idx, frame_idx];
        crc_data.extend_from_slice(&param);
        let crc = calculate_crc(&crc_data);
        frame.push(crc);
        
        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        
        let result = results[0].as_ref().unwrap();
        assert_eq!(result.key, "G");
        assert!(result.is_secure);
        assert!(!result.is_fin);
        assert_eq!(result.invoke_idx, invoke_idx);
        assert_eq!(result.frame_idx, frame_idx);
    }
    
    #[test]
    fn test_frame_parser_secure_fin() {
        let mut parser = FrameParser::new();
        
        let mut frame = vec![0xAA, 0x11];
        let key = b'G';
        let type_key = 0xC0u8;
        let invoke_idx = 0x05u8;
        let param = [0x01, 0x02];
        let length = (1 + 1 + 1 + param.len()) as u8;
        
        frame.push(length);
        frame.push(type_key);
        frame.push(key);
        frame.push(invoke_idx);
        frame.extend_from_slice(&param);
        
        let mut crc_data = vec![type_key, key, invoke_idx];
        crc_data.extend_from_slice(&param);
        let crc = calculate_crc(&crc_data);
        frame.push(crc);
        
        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        
        let result = results[0].as_ref().unwrap();
        assert_eq!(result.key, "G");
        assert!(result.is_secure);
        assert!(result.is_fin);
        assert_eq!(result.invoke_idx, invoke_idx);
        assert_eq!(result.frame_idx, LAST_FRAME_FIX_INDEX);
    }
    
    #[test]
    fn test_frame_parser_unsecure_unfin() {
        let mut parser = FrameParser::new();
        
        let mut frame = vec![0xAA, 0x11];
        let key = b'G';
        let type_key = 0x03u8;
        let frame_idx = 0x0Au8;
        let param = [0x01, 0x02];
        let length = (1 + 1 + 1 + param.len()) as u8;
        
        frame.push(length);
        frame.push(type_key);
        frame.push(key);
        frame.push(frame_idx);
        frame.extend_from_slice(&param);
        
        let mut crc_data = vec![type_key, key, frame_idx];
        crc_data.extend_from_slice(&param);
        let crc = calculate_crc(&crc_data);
        frame.push(crc);
        
        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        
        let result = results[0].as_ref().unwrap();
        assert_eq!(result.key, "G");
        assert!(!result.is_secure);
        assert!(!result.is_fin);
        assert_eq!(result.frame_idx, frame_idx);
    }

    #[test]
    fn test_build_single_char_key_frame() {
        let mut builder = FrameBuilder::new();
        let frame = builder.build_frame("G", &[0x01, 0x02], false, true, 0, 0);

        assert!(!frame.is_empty());
        assert!(frame.starts_with(&FRAME_HEADER));
    }

    #[test]
    fn test_build_multi_char_key_frame() {
        let mut builder = FrameBuilder::new();
        let frame = builder.build_frame("Test", &[0x01, 0x02], false, true, 0, 0);

        assert!(!frame.is_empty());
        assert!(frame.starts_with(&FRAME_HEADER));
    }

    #[test]
    fn test_build_secure_frame() {
        let mut builder = FrameBuilder::new();
        let frame = builder.build_frame("G", &[0x01, 0x02], true, true, 0x05, 0);

        assert!(!frame.is_empty());
    }

    #[test]
    fn test_calculate_max_payload() {
        let payload = FrameBuilder::calculate_max_payload("G", false, true);
        assert!(payload > 200);

        let payload_secure = FrameBuilder::calculate_max_payload("G", true, true);
        assert!(payload_secure < payload);
    }

    #[test]
    fn test_build_frames_single() {
        let mut builder = FrameBuilder::new();
        let data = vec![0x01, 0x02, 0x03];
        let frames = builder.build_frames("G", &data, false);

        assert_eq!(frames.len(), 1);
    }

    #[test]
    fn test_build_frames_multi() {
        let mut builder = FrameBuilder::new();
        let data: Vec<u8> = (0..=255u8).cycle().take(300).collect();
        let frames = builder.build_frames("G", &data, false);

        assert!(frames.len() > 1);
    }

    #[test]
    fn test_build_frames_never_exceed_frame_size() {
        let mut builder = FrameBuilder::new();
        let data: Vec<u8> = (0..=255u8).cycle().take(302).collect();

        let frames = builder.build_frames_with_invoke_idx(
            "GH3X_RegsListWriteCmd",
            &data,
            true,
            0x0A,
        );

        assert!(frames.len() > 1);
        for frame in frames {
            assert!(
                frame.len() <= GHRPC_FRAME_SIZE,
                "frame length {} exceeds {}",
                frame.len(),
                GHRPC_FRAME_SIZE
            );
        }
    }

    #[test]
    fn test_build_and_parse_roundtrip() {
        let mut builder = FrameBuilder::new();
        let mut parser = FrameParser::new();

        let key = "G";
        let param = vec![0x01, 0x02, 0x03];
        let frame = builder.build_frame(key, &param, false, true, 0, 0);

        let results = parser.process(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());

        let result = results[0].as_ref().unwrap();
        assert_eq!(result.key, key);
        assert_eq!(result.param, param);
    }
}
