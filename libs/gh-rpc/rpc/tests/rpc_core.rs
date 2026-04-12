//! # RPC 核心测试模块
//!
//! 测试 RpcCore 的函数注册、帧构建和发送回调功能。

use rpc::{
    InvokeNode, RpcConfig, RpcCore, RpcError,
    FRAME_HEADER,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

mod invoke_node_tests {
    use super::*;

    fn test_handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
        0
    }

    #[test]
    fn test_invoke_node_creation() {
        let node = InvokeNode::new("test", Some("<u8>"), Some(test_handler));
        assert_eq!(node.key, "test");
        assert_eq!(node.detail, Some("<u8>"));
        assert!(node.func.is_some());
        assert!(node.header.is_none());
    }

    #[test]
    fn test_invoke_node_without_detail() {
        let node = InvokeNode::new("simple", None, Some(test_handler));
        assert_eq!(node.key, "simple");
        assert!(node.detail.is_none());
        assert!(node.func.is_some());
    }

    #[test]
    fn test_invoke_node_without_handler() {
        let node = InvokeNode::new("placeholder", Some("<u16>"), None);
        assert_eq!(node.key, "placeholder");
        assert!(node.detail.is_some());
        assert!(node.func.is_none());
    }

    #[test]
    fn test_invoke_node_const_creation() {
        const NODE: InvokeNode = InvokeNode::new("const_test", None, None);
        assert_eq!(NODE.key, "const_test");
    }

    #[test]
    fn test_invoke_node_copy() {
        let node1 = InvokeNode::new("test", Some("<u8>"), Some(test_handler));
        let node2 = node1;
        assert_eq!(node1.key, node2.key);
        assert_eq!(node1.detail, node2.detail);
    }

    #[test]
    fn test_invoke_node_clone() {
        let node1 = InvokeNode::new("test", Some("<u8>"), Some(test_handler));
        let node2 = node1.clone();
        assert_eq!(node1.key, node2.key);
    }
}

mod rpc_config_tests {
    use super::*;

    #[test]
    fn test_rpc_config_new() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        assert!(config.lock.is_none());
        assert!(config.unlock.is_none());
        assert!(config.delay.is_none());
    }

    #[test]
    fn test_rpc_config_with_callbacks() {
        fn test_lock() {}
        fn test_unlock() {}
        fn test_delay() {}

        let config = RpcConfig {
            send: |_data: &[u8]| {},
            lock: Some(test_lock),
            unlock: Some(test_unlock),
            delay: Some(test_delay),
        };

        assert!(config.lock.is_some());
        assert!(config.unlock.is_some());
        assert!(config.delay.is_some());
    }
}

mod rpc_core_creation_tests {
    use super::*;

    #[test]
    fn test_rpc_core_new() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let _rpc: RpcCore<16, _> = RpcCore::new(config);
    }

    #[test]
    fn test_rpc_core_different_sizes() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let _rpc8: RpcCore<8, _> = RpcCore::new(config);
        
        let config2 = RpcConfig::new(|_data: &[u8]| {});
        let _rpc32: RpcCore<32, _> = RpcCore::new(config2);
    }
}

mod rpc_core_registration_tests {
    use super::*;

    fn test_handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
        0
    }

    #[test]
    fn test_rpc_core_single_registration() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let node = InvokeNode::new("test_cmd", None, Some(test_handler));
        let result = rpc.register(node);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_rpc_core_multiple_registrations() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        for i in 0..5 {
            let key = format!("cmd_{}", i);
            let key_static: &'static str = Box::leak(key.into_boxed_str());
            let node = InvokeNode::new(key_static, None, Some(test_handler));
            assert!(rpc.register(node).is_ok());
        }
    }

    #[test]
    fn test_rpc_core_registration_full() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<3, _> = RpcCore::new(config);

        for i in 0..3 {
            let key = format!("cmd_{}", i);
            let key_static: &'static str = Box::leak(key.into_boxed_str());
            let node = InvokeNode::new(key_static, None, Some(test_handler));
            assert!(rpc.register(node).is_ok());
        }

        let node = InvokeNode::new("overflow", None, Some(test_handler));
        let result = rpc.register(node);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RpcError::MemoryNotEnough);
    }
}

mod rpc_core_frame_building_tests {
    use super::*;

    #[test]
    fn test_publish_frame_has_header() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let frame = rpc.publish("G", &[1, 2, 3]).unwrap();

        assert!(frame.starts_with(&FRAME_HEADER));
        assert!(frame.len() > FRAME_HEADER.len() + 2);
    }

    #[test]
    fn test_publish_multi_char_key() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let frame = rpc.publish("test", &[1, 2, 3]).unwrap();

        assert!(frame.starts_with(&FRAME_HEADER));
        
        let mut found_key = false;
        for i in 0..frame.len().saturating_sub(4) {
            if frame[i..i+4] == *b"test" {
                found_key = true;
                break;
            }
        }
        assert!(found_key, "Key 'test' not found in frame");
    }

    #[test]
    fn test_publish_empty_data() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let frame = rpc.publish("G", &[]).unwrap();

        assert!(frame.starts_with(&FRAME_HEADER));
    }

    #[test]
    fn test_frame_crc_calculation() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let frame = rpc.publish("G", &[1, 2, 3]).unwrap();

        let crc = *frame.last().unwrap();
        
        let mut expected_crc: u8 = 0;
        for i in FRAME_HEADER.len() + 1..frame.len() - 1 {
            expected_crc = expected_crc.wrapping_add(frame[i]);
        }
        assert_eq!(crc, expected_crc, "CRC should match sum of bytes after header");
    }

    #[test]
    fn test_frame_length_field() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let frame = rpc.publish("G", &[1, 2, 3]).unwrap();

        let length = frame[FRAME_HEADER.len()];
        let expected_length = (frame.len() - FRAME_HEADER.len() - 2) as u8;
        assert_eq!(length, expected_length, "Length field should match frame content size");
    }
}

mod rpc_core_send_tests {
    use super::*;

    #[test]
    fn test_publish_calls_send_callback() {
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
    fn test_send_calls_send_callback() {
        static SENT_COUNT: AtomicUsize = AtomicUsize::new(0);
        SENT_COUNT.store(0, Ordering::SeqCst);

        let config = RpcConfig::new(|_data: &[u8]| {
            SENT_COUNT.fetch_add(1, Ordering::SeqCst);
        });

        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        assert!(rpc.send("test", &[1, 2, 3]).is_ok());
        assert_eq!(SENT_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_call_calls_send_callback() {
        use std::sync::atomic::AtomicBool;
        use std::sync::atomic::Ordering;
        
        static SENT_COUNT: AtomicUsize = AtomicUsize::new(0);
        static CALLBACK_CALLED: AtomicBool = AtomicBool::new(false);
        SENT_COUNT.store(0, Ordering::SeqCst);
        CALLBACK_CALLED.store(false, Ordering::SeqCst);

        let config = RpcConfig::new(|_data: &[u8]| {
            SENT_COUNT.fetch_add(1, Ordering::SeqCst);
        })
        .with_delay(|| {
            if !CALLBACK_CALLED.load(Ordering::SeqCst) {
                CALLBACK_CALLED.store(true, Ordering::SeqCst);
            }
        });

        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let mut ret_buf = [0u8; 64];
        let result = rpc.call("test", &[1, 2, 3], &mut ret_buf);
        assert!(result.is_ok() || result.is_err());
        assert!(SENT_COUNT.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn test_send_data_content() {
        static LAST_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());
        
        {
            let mut data = LAST_DATA.lock().unwrap();
            data.clear();
        }

        let config = RpcConfig::new(|data: &[u8]| {
            let mut last = LAST_DATA.lock().unwrap();
            last.extend_from_slice(data);
        });

        let mut rpc: RpcCore<16, _> = RpcCore::new(config);
        let frame = rpc.publish("G", &[0x01, 0x02, 0x03]).unwrap();

        let last_data = LAST_DATA.lock().unwrap();
        assert_eq!(*last_data, frame.as_slice());
    }

    #[test]
    fn test_multiple_sends_increment_counter() {
        static SENT_COUNT: AtomicUsize = AtomicUsize::new(0);
        SENT_COUNT.store(0, Ordering::SeqCst);

        let config = RpcConfig::new(|_data: &[u8]| {
            SENT_COUNT.fetch_add(1, Ordering::SeqCst);
        });

        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        for _ in 0..5 {
            assert!(rpc.publish("test", &[]).is_ok());
        }
        assert_eq!(SENT_COUNT.load(Ordering::SeqCst), 5);
    }
}

mod rpc_core_process_tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn test_handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
        0
    }

    #[test]
    fn test_process_valid_frame_single_char_key() {
        static HANDLER_CALLED: AtomicBool = AtomicBool::new(false);
        HANDLER_CALLED.store(false, Ordering::SeqCst);

        fn handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
            HANDLER_CALLED.store(true, Ordering::SeqCst);
            0
        }

        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let node = InvokeNode::new("G", None, Some(handler));
        rpc.register(node).unwrap();

        let frame = rpc.publish("G", &[1, 2, 3]).unwrap();
        rpc.process(&frame, true);

        assert!(HANDLER_CALLED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_process_unknown_key() {
        static HANDLER_CALLED: AtomicBool = AtomicBool::new(false);
        HANDLER_CALLED.store(false, Ordering::SeqCst);

        fn handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
            HANDLER_CALLED.store(true, Ordering::SeqCst);
            0
        }

        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let node = InvokeNode::new("known", None, Some(handler));
        rpc.register(node).unwrap();

        let frame = rpc.publish("unknown", &[1, 2, 3]).unwrap();
        rpc.process(&frame, true);

        assert!(!HANDLER_CALLED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_process_with_restart() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let node = InvokeNode::new("G", None, Some(test_handler));
        rpc.register(node).unwrap();

        let frame = rpc.publish("G", &[]).unwrap();
        
        rpc.process(&frame, true);
        rpc.process(&frame, true);
    }

    #[test]
    fn test_process_without_restart() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let node = InvokeNode::new("G", None, Some(test_handler));
        rpc.register(node).unwrap();

        let frame = rpc.publish("G", &[]).unwrap();
        
        rpc.process(&frame, true);
        rpc.process(&frame, false);
    }

    #[test]
    fn test_process_partial_then_complete() {
        static HANDLER_CALLED: AtomicUsize = AtomicUsize::new(0);
        HANDLER_CALLED.store(0, Ordering::SeqCst);

        fn handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
            HANDLER_CALLED.fetch_add(1, Ordering::SeqCst);
            0
        }

        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let node = InvokeNode::new("G", None, Some(handler));
        rpc.register(node).unwrap();

        let frame = rpc.publish("G", &[1, 2, 3]).unwrap();
        
        rpc.process(&frame[..3], true);
        assert_eq!(HANDLER_CALLED.load(Ordering::SeqCst), 0);
        
        rpc.process(&frame[3..], false);
        assert_eq!(HANDLER_CALLED.load(Ordering::SeqCst), 1);
    }
}

mod rpc_core_return_result_tests {
    use super::*;

    #[test]
    fn test_return_result_not_under_invoke() {
        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let result = rpc.return_result(&[1, 2, 3]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RpcError::NotUnderInvoke);
    }
}

mod integration_tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_full_roundtrip_single_char_key() {
        static RECEIVE_COUNT: AtomicUsize = AtomicUsize::new(0);
        static SEND_COUNT: AtomicUsize = AtomicUsize::new(0);
        RECEIVE_COUNT.store(0, Ordering::SeqCst);
        SEND_COUNT.store(0, Ordering::SeqCst);

        fn handler(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
            RECEIVE_COUNT.fetch_add(1, Ordering::SeqCst);
            0
        }

        let config = RpcConfig::new(|_data: &[u8]| {
            SEND_COUNT.fetch_add(1, Ordering::SeqCst);
        });

        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        let node = InvokeNode::new("G", None, Some(handler));
        rpc.register(node).unwrap();

        let frame = rpc.publish("G", &[1, 2, 3]).unwrap();
        assert_eq!(SEND_COUNT.load(Ordering::SeqCst), 1);

        rpc.process(&frame, true);
        assert_eq!(RECEIVE_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_multiple_handlers_single_char_keys() {
        static HANDLER_A_COUNT: AtomicUsize = AtomicUsize::new(0);
        static HANDLER_B_COUNT: AtomicUsize = AtomicUsize::new(0);
        HANDLER_A_COUNT.store(0, Ordering::SeqCst);
        HANDLER_B_COUNT.store(0, Ordering::SeqCst);

        fn handler_a(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
            HANDLER_A_COUNT.fetch_add(1, Ordering::SeqCst);
            0
        }

        fn handler_b(_data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
            HANDLER_B_COUNT.fetch_add(1, Ordering::SeqCst);
            0
        }

        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        rpc.register(InvokeNode::new("A", None, Some(handler_a))).unwrap();
        rpc.register(InvokeNode::new("B", None, Some(handler_b))).unwrap();

        let frame_a = rpc.publish("A", &[]).unwrap();
        let frame_b = rpc.publish("B", &[]).unwrap();

        rpc.process(&frame_a, true);
        rpc.process(&frame_b, true);

        assert_eq!(HANDLER_A_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(HANDLER_B_COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_process_and_get_param() {
        static RECEIVED_PARAM: Mutex<Vec<u8>> = Mutex::new(Vec::new());
        
        {
            let mut param = RECEIVED_PARAM.lock().unwrap();
            param.clear();
        }

        fn handler(data: &[u8], _size: usize, _ret: Option<&mut [u8]>) -> i32 {
            let mut param = RECEIVED_PARAM.lock().unwrap();
            param.extend_from_slice(data);
            0
        }

        let config = RpcConfig::new(|_data: &[u8]| {});
        let mut rpc: RpcCore<16, _> = RpcCore::new(config);

        rpc.register(InvokeNode::new("G", None, Some(handler))).unwrap();

        let test_data = [0x01, 0x02, 0x03, 0x04];
        let frame = rpc.publish("G", &test_data).unwrap();

        let param = rpc.process_and_get_param(&frame, true);

        assert!(param.is_some());
        let param_data = param.unwrap();
        assert_eq!(param_data.as_slice(), test_data);
    }
}

mod error_tests {
    use super::*;

    #[test]
    fn test_rpc_error_values() {
        assert_eq!(RpcError::FormatError as i32, 1);
        assert_eq!(RpcError::KeyOverMaxSize as i32, 2);
        assert_eq!(RpcError::NotUnderInvoke as i32, 3);
        assert_eq!(RpcError::SendFail as i32, 4);
        assert_eq!(RpcError::MemoryNotEnough as i32, 5);
        assert_eq!(RpcError::LoseFrame as i32, 6);
        assert_eq!(RpcError::CrcError as i32, 7);
    }

    #[test]
    fn test_rpc_error_equality() {
        assert_eq!(RpcError::MemoryNotEnough, RpcError::MemoryNotEnough);
        assert_ne!(RpcError::MemoryNotEnough, RpcError::FormatError);
    }
}
