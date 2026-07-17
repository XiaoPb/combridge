//! Frame Parser
//!
//! 帧格式：
//! +----------+--------+---------+----------+-------+----------+--------+-----+
//! | Header   | Length | TypeKey | KeyData  | ComID | FrameID  | Param  | CRC |
//! | 2 bytes  | 1 byte | 1 byte  | N bytes  | 1 byte| 1 byte   | N bytes|1byte|
//! +----------+--------+---------+----------+-------+----------+--------+-----+

use std::collections::VecDeque;

use crate::error::RpcError;
use crate::types::{TypeKey, FRAME_HEADER, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE};

const LAST_FRAME_FIX_INDEX: u8 = 255;
pub const RPC_RECEIVE_BUFFER_SIZE: usize = 512;

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
    receive_buffer: VecDeque<u8>,
}

impl Default for FrameParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameParser {
    pub fn new() -> Self {
        Self {
            receive_buffer: VecDeque::with_capacity(RPC_RECEIVE_BUFFER_SIZE),
        }
    }

    pub fn reset(&mut self) {
        self.receive_buffer.clear();
    }

    pub fn process(&mut self, data: &[u8]) -> Vec<Result<ParseResult, RpcError>> {
        let mut results = Vec::new();

        let mut offset = 0;
        while offset < data.len() {
            if self.receive_buffer.len() == RPC_RECEIVE_BUFFER_SIZE {
                let previous_len = self.receive_buffer.len();
                self.parse_available(&mut results);
                if self.receive_buffer.len() == previous_len {
                    self.recover_from_invalid_candidate();
                }
            }

            let available = RPC_RECEIVE_BUFFER_SIZE - self.receive_buffer.len();
            let chunk_len = available.min(data.len() - offset);
            self.receive_buffer
                .extend(&data[offset..offset + chunk_len]);
            offset += chunk_len;
            self.parse_available(&mut results);
        }

        self.parse_available(&mut results);
        results
    }

    fn parse_available(&mut self, results: &mut Vec<Result<ParseResult, RpcError>>) {
        loop {
            if !self.align_to_frame_header() || self.receive_buffer.len() < 3 {
                return;
            }

            let body_len = self.receive_buffer[2] as usize;
            let frame_len = FRAME_HEADER.len() + 1 + body_len + 1;
            if self.receive_buffer.len() < frame_len {
                return;
            }

            let (crc_matches, frame_body) = {
                let buffer = self.receive_buffer.make_contiguous();
                let body = &buffer[3..3 + body_len];
                (calculate_crc(body) == buffer[3 + body_len], body.to_vec())
            };

            if !crc_matches {
                results.push(Err(RpcError::CrcMismatch));
                self.recover_from_invalid_candidate();
                continue;
            }

            match Self::parse_frame_body(&frame_body) {
                Ok(result) => {
                    self.receive_buffer.drain(..frame_len);
                    results.push(Ok(result));
                }
                Err(error) => {
                    results.push(Err(error));
                    self.recover_from_invalid_candidate();
                }
            }
        }
    }

    fn align_to_frame_header(&mut self) -> bool {
        let header_pos = self.find_frame_header(0);
        match header_pos {
            Some(0) => true,
            Some(position) => {
                self.receive_buffer.drain(..position);
                true
            }
            None => {
                let keep_trailing_header_byte =
                    self.receive_buffer.back().copied() == Some(FRAME_HEADER[0]);
                self.receive_buffer.clear();
                if keep_trailing_header_byte {
                    self.receive_buffer.push_back(FRAME_HEADER[0]);
                }
                false
            }
        }
    }

    fn recover_from_invalid_candidate(&mut self) {
        if let Some(position) = self.find_frame_header(1) {
            self.receive_buffer.drain(..position);
            return;
        }

        let keep_trailing_header_byte =
            self.receive_buffer.back().copied() == Some(FRAME_HEADER[0]);
        self.receive_buffer.clear();
        if keep_trailing_header_byte {
            self.receive_buffer.push_back(FRAME_HEADER[0]);
        }
    }

    fn find_frame_header(&self, start: usize) -> Option<usize> {
        if self.receive_buffer.len() < FRAME_HEADER.len() || start >= self.receive_buffer.len() {
            return None;
        }

        (start..self.receive_buffer.len() - 1).find(|&index| {
            self.receive_buffer[index] == FRAME_HEADER[0]
                && self.receive_buffer[index + 1] == FRAME_HEADER[1]
        })
    }

    fn parse_frame_body(body: &[u8]) -> Result<ParseResult, RpcError> {
        let type_key_byte = *body.first().ok_or(RpcError::FormatError)?;
        let type_key = TypeKey::from_byte(type_key_byte);
        let mut offset = 1;

        let key = if type_key.is_array {
            let key_len = *body.get(offset).ok_or(RpcError::FormatError)? as usize;
            offset += 1;
            if key_len > MAX_SUPPORT_KEY_SIZE - 1 || offset + key_len > body.len() {
                return Err(if key_len > MAX_SUPPORT_KEY_SIZE - 1 {
                    RpcError::KeyOverMaxSize
                } else {
                    RpcError::FormatError
                });
            }
            let key = String::from_utf8_lossy(&body[offset..offset + key_len]).to_string();
            offset += key_len;
            key
        } else {
            let key = *body.get(offset).ok_or(RpcError::FormatError)?;
            offset += 1;
            String::from_utf8_lossy(&[key]).to_string()
        };

        let mut invoke_idx = 0;
        let mut frame_idx = LAST_FRAME_FIX_INDEX;
        if type_key.secure {
            invoke_idx = *body.get(offset).ok_or(RpcError::FormatError)?;
            offset += 1;
        }
        if !type_key.fin {
            frame_idx = *body.get(offset).ok_or(RpcError::FormatError)?;
            offset += 1;
        }

        Ok(ParseResult {
            key,
            param: body[offset..].to_vec(),
            is_secure: type_key.secure,
            is_fin: type_key.fin,
            invoke_idx,
            frame_idx,
        })
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

    pub fn build_frames(&mut self, key: &str, data: &[u8], secure: bool) -> Vec<Vec<u8>> {
        self.build_frames_with_invoke_idx(key, data, secure, 0)
    }

    pub fn build_frames_with_invoke_idx(
        &mut self,
        key: &str,
        data: &[u8],
        secure: bool,
        invoke_idx: u8,
    ) -> Vec<Vec<u8>> {
        let max_payload_intermediate = Self::calculate_max_payload(key, secure, false);
        let max_payload_final = Self::calculate_max_payload(key, secure, true);

        if data.len() <= max_payload_final {
            let frame = self.build_frame(key, data, secure, true, invoke_idx, 0);
            return vec![frame];
        }

        let mut frames = Vec::new();
        let mut offset = 0;
        let mut frame_idx: u8 = 0;

        while offset < data.len() {
            let remaining = data.len() - offset;
            let is_fin = remaining <= max_payload_final;
            let chunk_size = if is_fin {
                remaining
            } else {
                remaining.min(max_payload_intermediate)
            };

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
        parser.receive_buffer.extend([0xAA, 0x11, 0x20, 0x55]);
        parser.reset();

        assert!(parser.receive_buffer.is_empty());
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

        let frames =
            builder.build_frames_with_invoke_idx("GH3X_RegsListWriteCmd", &data, true, 0x0A);

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
    fn test_build_frames_uses_final_frame_capacity_for_boundary() {
        let key = "GH3X_RegsListWriteCmd";
        let final_capacity = FrameBuilder::calculate_max_payload(key, true, true);
        let mut builder = FrameBuilder::new();

        assert_eq!(
            builder
                .build_frames_with_invoke_idx(key, &vec![0; final_capacity], true, 1)
                .len(),
            1
        );
        assert_eq!(
            builder
                .build_frames_with_invoke_idx(key, &vec![0; final_capacity + 1], true, 1)
                .len(),
            2
        );
    }

    #[test]
    fn test_secure_register_list_frames_reassemble_all_366_bytes() {
        let data: Vec<u8> = (0..366).map(|index| (index & 0xff) as u8).collect();
        let frames = FrameBuilder::new().build_frames_with_invoke_idx(
            "GH3X_RegsListWriteCmd",
            &data,
            true,
            0x11,
        );
        let mut parser = FrameParser::new();
        let mut reassembled = Vec::new();

        assert!(frames.len() > 1);
        for frame in frames {
            let result = parser
                .process(&frame)
                .into_iter()
                .next()
                .expect("frame result")
                .expect("valid frame");
            assert_eq!(result.invoke_idx, 0x11);
            reassembled.extend_from_slice(&result.param);
        }

        assert_eq!(reassembled, data);
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

    #[test]
    fn test_length_driven_parser_handles_fragmented_frame() {
        let frame = FrameBuilder::new().build_frame("G", &[0x10, 0x20, 0x30], false, true, 0, 0);
        let mut parser = FrameParser::new();

        assert!(parser.process(&frame[..1]).is_empty());
        assert!(parser.process(&frame[1..2]).is_empty());
        assert!(parser.process(&frame[2..3]).is_empty());
        assert!(parser.process(&frame[3..frame.len() - 1]).is_empty());

        let results = parser.process(&frame[frame.len() - 1..]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().param, vec![0x10, 0x20, 0x30]);
    }

    #[test]
    fn test_length_driven_parser_ignores_headers_inside_param() {
        let param = [0x01, 0xAA, 0x11, 0x02, 0xAA, 0x11, 0x03];
        let frame = FrameBuilder::new().build_frame("G", &param, false, true, 0, 0);
        let mut parser = FrameParser::new();

        let results = parser.process(&frame);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().param, param);
    }

    #[test]
    fn test_length_driven_parser_recovers_header_swallowed_by_bad_length() {
        let valid = FrameBuilder::new().build_frame("G", &[0x42], false, true, 0, 0);
        let mut input = vec![0xAA, 0x11, 0x08, 0x80, b'X', 0x01];
        input.extend_from_slice(&valid);
        let mut parser = FrameParser::new();

        let results = parser.process(&input);

        assert!(matches!(results.first(), Some(Err(RpcError::CrcMismatch))));
        assert_eq!(results.len(), 2);
        let recovered = results[1].as_ref().unwrap();
        assert_eq!(recovered.key, "G");
        assert_eq!(recovered.param, vec![0x42]);
    }

    #[test]
    fn test_crc_byte_can_be_reused_as_next_header_prefix() {
        let valid = FrameBuilder::new().build_frame("G", &[0x42], false, true, 0, 0);
        let mut parser = FrameParser::new();
        let invalid_ending_in_aa = [0xAA, 0x11, 0x02, 0x80, b'X', 0xAA];

        let invalid_results = parser.process(&invalid_ending_in_aa);
        let recovered_results = parser.process(&valid[1..]);

        assert!(matches!(
            invalid_results.first(),
            Some(Err(RpcError::CrcMismatch))
        ));
        assert_eq!(recovered_results.len(), 1);
        assert_eq!(recovered_results[0].as_ref().unwrap().param, vec![0x42]);
    }

    #[test]
    fn test_parser_handles_noise_multiple_frames_and_trailing_fragment() {
        let first = FrameBuilder::new().build_frame("G", &[0x01], false, true, 0, 0);
        let second = FrameBuilder::new().build_frame("G", &[0x02], false, true, 0, 0);
        let third = FrameBuilder::new().build_frame("G", &[0x03], false, true, 0, 0);
        let mut input = vec![0x55, 0x66, 0x77];
        input.extend_from_slice(&first);
        input.extend_from_slice(&second);
        input.extend_from_slice(&third[..4]);
        let mut parser = FrameParser::new();

        let initial_results = parser.process(&input);
        let final_results = parser.process(&third[4..]);

        assert_eq!(initial_results.len(), 2);
        assert_eq!(initial_results[0].as_ref().unwrap().param, vec![0x01]);
        assert_eq!(initial_results[1].as_ref().unwrap().param, vec![0x02]);
        assert_eq!(final_results.len(), 1);
        assert_eq!(final_results[0].as_ref().unwrap().param, vec![0x03]);
    }

    #[test]
    fn test_length_255_frame_is_accepted() {
        let mut frame_body = vec![0x80, b'G'];
        frame_body.resize(u8::MAX as usize, 0x5A);
        let mut frame = vec![0xAA, 0x11, u8::MAX];
        frame.extend_from_slice(&frame_body);
        frame.push(calculate_crc(&frame_body));
        let mut parser = FrameParser::new();

        let results = parser.process(&frame);

        assert_eq!(frame.len(), 259);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().param.len(), 253);
    }

    #[test]
    fn test_short_lengths_use_protocol_boundary_then_fail_semantic_parse() {
        let mut parser = FrameParser::new();
        let length_zero = [0xAA, 0x11, 0x00, calculate_crc(&[])];
        let length_one_body = [0x80];
        let length_one = [
            0xAA,
            0x11,
            0x01,
            length_one_body[0],
            calculate_crc(&length_one_body),
        ];

        let zero_results = parser.process(&length_zero);
        let one_results = parser.process(&length_one);

        assert!(matches!(
            zero_results.first(),
            Some(Err(RpcError::FormatError))
        ));
        assert!(matches!(
            one_results.first(),
            Some(Err(RpcError::FormatError))
        ));
    }

    #[test]
    fn test_length_240_frame_is_accepted() {
        let mut frame_body = vec![0x80, b'G'];
        frame_body.resize(240, 0x33);
        let mut frame = vec![0xAA, 0x11, 240];
        frame.extend_from_slice(&frame_body);
        frame.push(calculate_crc(&frame_body));
        let mut parser = FrameParser::new();

        let results = parser.process(&frame);

        assert_eq!(frame.len(), 244);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().param.len(), 238);
    }

    #[test]
    fn test_parser_processes_input_larger_than_receive_buffer() {
        let first = FrameBuilder::new().build_frame("G", &[0x01], false, true, 0, 0);
        let second = FrameBuilder::new().build_frame("G", &[0x02], false, true, 0, 0);
        let mut input = vec![0x55; RPC_RECEIVE_BUFFER_SIZE + 37];
        input.extend_from_slice(&first);
        input.extend_from_slice(&second);
        let mut parser = FrameParser::new();

        let results = parser.process(&input);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().param, vec![0x01]);
        assert_eq!(results[1].as_ref().unwrap().param, vec![0x02]);
        assert!(parser.receive_buffer.len() <= RPC_RECEIVE_BUFFER_SIZE);
    }

    #[test]
    fn test_reset_discards_partial_frame() {
        let stale = FrameBuilder::new().build_frame("G", &[0x10], false, true, 0, 0);
        let fresh = FrameBuilder::new().build_frame("G", &[0x20], false, true, 0, 0);
        let mut parser = FrameParser::new();

        assert!(parser.process(&stale[..4]).is_empty());
        parser.reset();
        let results = parser.process(&fresh);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().param, vec![0x20]);
    }
}
