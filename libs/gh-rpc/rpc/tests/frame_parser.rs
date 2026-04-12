//! # 帧解析器测试模块
//!
//! 测试 FrameParser 状态机的各种转换和解析逻辑。

use rpc::{
    FrameIndex, FrameParser, ParseResult, ParseState,
    TypeKey, FRAME_HEADER, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE,
};
use heapless::Vec as HeaplessVec;

mod types_tests {
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
    fn test_type_key_new() {
        let key = TypeKey::new();
        assert_eq!(key.pack_type, 2);
        assert!(!key.is_array);
        assert_eq!(key.width, 7);
        assert!(!key.secure);
        assert!(key.fin);
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
    fn test_type_key_all_combinations() {
        for pack_type in 0..=3 {
            for width in 0..=7 {
                for &is_array in &[false, true] {
                    for &secure in &[false, true] {
                        for &fin in &[false, true] {
                            let mut key = TypeKey::new();
                            key.set_pack_type(pack_type);
                            key.set_is_array(is_array);
                            key.set_width(width);
                            key.set_secure(secure);
                            key.set_fin(fin);

                            let byte = key.to_byte();
                            let restored = TypeKey::from_byte(byte);

                            assert_eq!(key, restored, 
                                "Failed for pack_type={}, width={}, is_array={}, secure={}, fin={}",
                                pack_type, width, is_array, secure, fin);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_type_key_byte_encoding() {
        let mut key = TypeKey::new();
        key.set_pack_type(0b10);
        key.set_is_array(true);
        key.set_width(0b101);
        key.set_secure(true);
        key.set_fin(true);

        let byte = key.to_byte();
        
        assert_eq!(byte & 0b11, 0b10);
        assert_eq!((byte >> 2) & 1, 1);
        assert_eq!((byte >> 3) & 0b111, 0b101);
        assert_eq!((byte >> 6) & 1, 1);
        assert_eq!((byte >> 7) & 1, 1);
    }
}

mod frame_parser_state_machine {
    use super::*;

    #[test]
    fn test_parser_initial_state() {
        let parser = FrameParser::new();
        assert_eq!(parser.state, ParseState::FrameHeader);
        assert_eq!(parser.header_index, 0);
        assert_eq!(parser.frame_size, 0);
        assert_eq!(parser.crc, 0);
        assert_eq!(parser.key_len, 0);
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = FrameParser::new();
        parser.state = ParseState::CheckKey;
        parser.frame_size = 100;
        parser.crc = 50;
        parser.header_index = 5;
        parser.key_len = 10;
        parser.is_secure = true;
        parser.is_fin = true;

        parser.reset();

        assert_eq!(parser.state, ParseState::FrameHeader);
        assert_eq!(parser.frame_size, 0);
        assert_eq!(parser.crc, 0);
        assert_eq!(parser.header_index, 0);
        assert_eq!(parser.key_len, 0);
        assert!(!parser.is_secure);
        assert!(!parser.is_fin);
    }

    #[test]
    fn test_frame_header_parsing_first_byte() {
        let mut parser = FrameParser::new();

        let result = parser.process_byte(0xAA).unwrap();
        assert!(result.is_none());
        assert_eq!(parser.state, ParseState::FrameHeader);
        assert_eq!(parser.header_index, 1);
    }

    #[test]
    fn test_frame_header_parsing_second_byte() {
        let mut parser = FrameParser::new();
        parser.process_byte(0xAA).unwrap();

        let result = parser.process_byte(0x11).unwrap();
        assert!(result.is_none());
        assert_eq!(parser.state, ParseState::FrameHeader);
        assert_eq!(parser.header_index, 2);
    }

    #[test]
    fn test_frame_header_parsing_length_byte() {
        let mut parser = FrameParser::new();
        parser.process_byte(0xAA).unwrap();
        parser.process_byte(0x11).unwrap();

        let result = parser.process_byte(10).unwrap();
        assert!(result.is_none());
        assert_eq!(parser.state, ParseState::CheckKey);
        assert_eq!(parser.frame_size, 10);
    }

    #[test]
    fn test_frame_header_invalid_byte_resets() {
        let mut parser = FrameParser::new();
        parser.process_byte(0xAA).unwrap();

        let result = parser.process_byte(0xFF).unwrap();
        assert!(result.is_none());
        assert_eq!(parser.header_index, 0);
    }

    #[test]
    fn test_state_transition_header_to_key() {
        let mut parser = FrameParser::new();
        
        parser.process_byte(0xAA).unwrap();
        parser.process_byte(0x11).unwrap();
        parser.process_byte(5).unwrap();

        assert_eq!(parser.state, ParseState::CheckKey);
    }
}

mod frame_parser_key_parsing {
    use super::*;

    fn create_parser_at_key_state(frame_size: u8) -> FrameParser {
        let mut parser = FrameParser::new();
        parser.process_byte(0xAA).unwrap();
        parser.process_byte(0x11).unwrap();
        parser.process_byte(frame_size).unwrap();
        parser
    }

    #[test]
    fn test_single_char_key_parsing() {
        let mut parser = create_parser_at_key_state(5);

        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_fin(true);
        type_key.set_secure(false);
        
        parser.process_byte(type_key.to_byte()).unwrap();
        assert_eq!(parser.key_len, 1);

        parser.process_byte(b'G').unwrap();
        assert_eq!(parser.key_buffer[0], b'G');
    }

    #[test]
    fn test_multi_char_key_parsing_reverse_order() {
        let mut parser = create_parser_at_key_state(10);

        let mut type_key = TypeKey::new();
        type_key.set_is_array(true);
        type_key.set_fin(true);
        type_key.set_secure(false);
        
        parser.process_byte(type_key.to_byte()).unwrap();
        assert_eq!(parser.key_len, 255);

        parser.process_byte(4).unwrap();
        assert_eq!(parser.key_len, 4);

        parser.process_byte(b't').unwrap();
        parser.process_byte(b'e').unwrap();
        parser.process_byte(b's').unwrap();
        parser.process_byte(b't').unwrap();

        assert_eq!(parser.key_buffer[3], b't');
        assert_eq!(parser.key_buffer[2], b'e');
        assert_eq!(parser.key_buffer[1], b's');
        assert_eq!(parser.key_buffer[0], b't');
    }

    #[test]
    fn test_key_crc_calculation() {
        let mut parser = create_parser_at_key_state(10);

        let mut type_key = TypeKey::new();
        type_key.set_is_array(true);
        type_key.set_fin(true);
        type_key.set_secure(false);
        
        parser.process_byte(type_key.to_byte()).unwrap();
        let crc_after_type_key = parser.crc;
        assert_eq!(crc_after_type_key, type_key.to_byte());

        parser.process_byte(4).unwrap();
        let crc_after_len = parser.crc;
        assert_eq!(crc_after_len, type_key.to_byte().wrapping_add(4));
    }

    #[test]
    fn test_secure_flag_parsing() {
        let mut parser = create_parser_at_key_state(5);

        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_secure(true);
        type_key.set_fin(true);
        
        parser.process_byte(type_key.to_byte()).unwrap();
        assert!(parser.is_secure);
        assert!(parser.is_fin);
    }

    #[test]
    fn test_fin_flag_parsing() {
        let mut parser = create_parser_at_key_state(5);

        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_fin(false);
        type_key.set_secure(false);
        
        parser.process_byte(type_key.to_byte()).unwrap();
        assert!(!parser.is_fin);
    }
}

mod frame_parser_index_parsing {
    use super::*;

    fn create_parser_at_index_state(is_secure: bool, is_fin: bool) -> FrameParser {
        let mut parser = FrameParser::new();
        parser.process_byte(0xAA).unwrap();
        parser.process_byte(0x11).unwrap();
        
        let frame_size = if is_secure && !is_fin {
            8
        } else {
            6
        };
        parser.process_byte(frame_size).unwrap();

        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_secure(is_secure);
        type_key.set_fin(is_fin);
        
        parser.process_byte(type_key.to_byte()).unwrap();
        parser.process_byte(b'G').unwrap();
        parser
    }

    #[test]
    fn test_index_non_secure_non_fin() {
        let mut parser = create_parser_at_index_state(false, false);
        
        parser.process_byte(0x05).unwrap();
        
        assert_eq!(parser.state, ParseState::CheckParam);
        assert_eq!(parser.frame_index.frame_idx, 0x05);
    }

    #[test]
    fn test_index_non_secure_fin() {
        let parser = create_parser_at_index_state(false, true);
        
        assert_eq!(parser.state, ParseState::CheckParam);
        assert_eq!(parser.frame_index.frame_idx, 0);
    }

    #[test]
    fn test_index_secure_fin() {
        let mut parser = create_parser_at_index_state(true, true);
        
        parser.process_byte(0x42).unwrap();
        
        assert_eq!(parser.state, ParseState::CheckParam);
        assert_eq!(parser.frame_index.invoke_idx, 0x42);
        assert_eq!(parser.frame_index.frame_idx, 255);
    }
}

mod frame_parser_crc {
    use super::*;

    #[test]
    fn test_crc_mismatch_error() {
        let mut parser = FrameParser::new();
        
        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_fin(true);
        type_key.set_secure(false);

        let length = 2u8;
        
        parser.process_byte(0xAA).unwrap();
        parser.process_byte(0x11).unwrap();
        parser.process_byte(length).unwrap();
        parser.process_byte(type_key.to_byte()).unwrap();
        parser.process_byte(b'G').unwrap();
        
        let result = parser.process_byte(0xFF);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), rpc::FrameError::CrcMismatch);
    }

    #[test]
    fn test_crc_success() {
        let mut parser = FrameParser::new();
        
        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_fin(true);
        type_key.set_secure(false);

        let length = 2u8;
        let expected_crc = type_key.to_byte().wrapping_add(b'G');
        
        parser.process_byte(0xAA).unwrap();
        parser.process_byte(0x11).unwrap();
        parser.process_byte(length).unwrap();
        parser.process_byte(type_key.to_byte()).unwrap();
        parser.process_byte(b'G').unwrap();

        let result = parser.process_byte(expected_crc).unwrap();

        assert!(result.is_some());
        let parse_result = result.unwrap();
        assert_eq!(parse_result.key[0], b'G');
    }
}

mod parse_result_tests {
    use super::*;

    #[test]
    fn test_parse_result_key_str() {
        let mut result = ParseResult {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 0,
            is_secure: false,
            is_fin: true,
            frame_index: FrameIndex::default(),
            param: HeaplessVec::new(),
        };
        result.key[0] = b't';
        result.key[1] = b'e';
        result.key[2] = b's';
        result.key[3] = b't';

        assert_eq!(result.key_str(), "test");
    }

    #[test]
    fn test_parse_result_empty_key() {
        let result = ParseResult {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 0,
            is_secure: false,
            is_fin: true,
            frame_index: FrameIndex::default(),
            param: HeaplessVec::new(),
        };

        assert_eq!(result.key_str(), "");
    }

    #[test]
    fn test_parse_result_equality() {
        let mut result1 = ParseResult {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 4,
            is_secure: false,
            is_fin: true,
            frame_index: FrameIndex::default(),
            param: HeaplessVec::new(),
        };
        result1.key[0] = b't';
        result1.key[1] = b'e';
        result1.key[2] = b's';
        result1.key[3] = b't';

        let mut result2 = ParseResult {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 4,
            is_secure: false,
            is_fin: true,
            frame_index: FrameIndex::default(),
            param: HeaplessVec::new(),
        };
        result2.key[0] = b't';
        result2.key[1] = b'e';
        result2.key[2] = b's';
        result2.key[3] = b't';

        assert_eq!(result1, result2);
    }
}

mod frame_index_tests {
    use super::*;

    #[test]
    fn test_frame_index_default() {
        let index = FrameIndex::default();
        assert_eq!(index.invoke_idx, 0);
        assert_eq!(index.frame_idx, 0);
    }
}

mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_frame_parsing_single_char_key() {
        let mut parser = FrameParser::new();

        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_fin(true);
        type_key.set_secure(false);

        let key_byte = b'G';
        let crc = type_key.to_byte().wrapping_add(key_byte);
        let length = 2u8;

        let frame: Vec<u8> = vec![
            0xAA, 0x11,
            length,
            type_key.to_byte(),
            key_byte,
            crc,
        ];

        let mut result: Option<ParseResult> = None;
        for &byte in &frame {
            match parser.process_byte(byte) {
                Ok(Some(r)) => result = Some(r),
                Ok(None) => {}
                Err(_) => panic!("Parsing failed"),
            }
        }

        assert!(result.is_some());
        let parse_result = result.unwrap();
        assert_eq!(parse_result.key[0], b'G');
        assert!(parse_result.is_fin);
        assert!(!parse_result.is_secure);
        assert!(parse_result.param.is_empty());
    }

    #[test]
    fn test_complete_frame_parsing_multi_char_key() {
        let mut parser = FrameParser::new();

        let mut type_key = TypeKey::new();
        type_key.set_is_array(true);
        type_key.set_fin(true);
        type_key.set_secure(false);

        let key = b"test";
        let key_len = key.len() as u8;
        
        let mut crc = type_key.to_byte().wrapping_add(key_len);
        for &b in key {
            crc = crc.wrapping_add(b);
        }

        let length = 1 + 1 + key.len() as u8;

        let mut frame: Vec<u8> = vec![
            0xAA, 0x11,
            length,
            type_key.to_byte(),
            key_len,
        ];
        frame.extend_from_slice(key);
        frame.push(crc);

        let mut result: Option<ParseResult> = None;
        for &byte in &frame {
            match parser.process_byte(byte) {
                Ok(Some(r)) => result = Some(r),
                Ok(None) => {}
                Err(_) => panic!("Parsing failed"),
            }
        }

        assert!(result.is_some());
        let parse_result = result.unwrap();
        assert_eq!(parse_result.key[3], b't');
        assert_eq!(parse_result.key[2], b'e');
        assert_eq!(parse_result.key[1], b's');
        assert_eq!(parse_result.key[0], b't');
        assert!(parse_result.is_fin);
    }

    #[test]
    fn test_complete_frame_with_params() {
        let mut parser = FrameParser::new();

        let mut type_key = TypeKey::new();
        type_key.set_is_array(false);
        type_key.set_fin(true);
        type_key.set_secure(false);

        let key_byte = b'G';
        let params = [0x01, 0x02, 0x03];
        
        let mut crc = type_key.to_byte().wrapping_add(key_byte);
        for &p in &params {
            crc = crc.wrapping_add(p);
        }

        let length = 1 + 1 + params.len() as u8;

        let mut frame: Vec<u8> = vec![
            0xAA, 0x11,
            length,
            type_key.to_byte(),
            key_byte,
        ];
        frame.extend_from_slice(&params);
        frame.push(crc);

        let mut result: Option<ParseResult> = None;
        for &byte in &frame {
            match parser.process_byte(byte) {
                Ok(Some(r)) => result = Some(r),
                Ok(None) => {}
                Err(_) => panic!("Parsing failed"),
            }
        }

        assert!(result.is_some());
        let parse_result = result.unwrap();
        assert_eq!(parse_result.param.as_slice(), &params);
    }

    #[test]
    fn test_multiple_frames_sequentially() {
        for _ in 0..3 {
            let mut parser = FrameParser::new();

            let mut type_key = TypeKey::new();
            type_key.set_is_array(false);
            type_key.set_fin(true);
            type_key.set_secure(false);

            let key_byte = b'G';
            let crc = type_key.to_byte().wrapping_add(key_byte);
            let length = 2u8;

            let frame: Vec<u8> = vec![
                0xAA, 0x11,
                length,
                type_key.to_byte(),
                key_byte,
                crc,
            ];

            let mut result: Option<ParseResult> = None;
            for &byte in &frame {
                match parser.process_byte(byte) {
                    Ok(Some(r)) => result = Some(r),
                    Ok(None) => {}
                    Err(_) => panic!("Parsing failed"),
                }
            }

            assert!(result.is_some());
        }
    }
}
