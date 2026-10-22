//! 帧编解码、固件解析与端到端会话测试。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::engine::{
    DfuEngine, DfuPlan, EngineConfig, ProgressEvent, ProgressSink, Stage, TargetAddress,
    TransferMode, TransferOptions,
};
use super::error::DfuError;
use super::firmware::{
    check_overlap_v1, check_overlap_v2, BootInfo, DfuImage, ImgInfo, BOOT_INFO_LEN,
};
use super::frame::basic::{opcode, BasicFrame};
use super::frame::fast::{opcode as fast, FastFrame};
use super::mock::{fast_service_capabilities, v1_capabilities, v2_capabilities, MockTransport};
use super::transport::{Channel, Transport};
use super::util::bump_last_mac_byte;
use crate::ota::types::OtaStartOptions;

const APP_LOAD: u32 = 0x0001_0000;
const APP_SIZE: u32 = 0x0000_2000;
const BOOT_LOAD: u32 = 0x0000_0000;
const BOOT_SIZE: u32 = 0x0000_8000;
const STACK_SVN: u32 = 0xCA0F_33C7; // SCA at 0x0020_0000

// ----------------------------------------------------------------------
// 测试数据构造
// ----------------------------------------------------------------------

/// 尾部布局：载荷 + 镜像信息 + 8 字节保留区。
fn firmware_bytes(load_addr: u32, payload_len: usize) -> Vec<u8> {
    let total = payload_len + 40 + 8;
    let boot = BootInfo {
        bin_size: total as u32,
        checksum: 0x1234_5678,
        load_addr,
        run_addr: load_addr,
        xqspi_xip_cmd: 0x03,
        ..BootInfo::default()
    };
    let info = ImgInfo::new(1, boot, "integration");
    let mut data = vec![0xAA_u8; payload_len];
    data.extend_from_slice(&info.to_bytes());
    data.extend_from_slice(&[0_u8; 8]);
    data
}

/// 构造签名（可选加密）固件镜像。
fn signed_firmware_bytes(load_addr: u32, payload_len: usize, encrypted: bool) -> Vec<u8> {
    let total = payload_len + 856 + 40 + 8;
    let boot = BootInfo {
        bin_size: total as u32,
        checksum: 0x2222_3333,
        load_addr,
        run_addr: load_addr,
        xqspi_xip_cmd: 0x03,
        ..BootInfo::default()
    };
    let info = ImgInfo::new(1, boot, "signed");

    let mut data = vec![0xAA_u8; payload_len];
    data.extend_from_slice(&info.to_bytes());
    data.extend_from_slice(&[0_u8; 8]);
    data.extend_from_slice(&[0_u8; 856]);

    let len = data.len();
    let signature: u32 = if encrypted { 0 } else { 0x4E47_4953 };
    data[len - 784..len - 780].copy_from_slice(&signature.to_le_bytes());
    data
}

/// `GET_INFO` 响应载荷：状态字节 + 17 字节芯片信息。
fn chip_info_payload(stack_svn: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(1 + 17);
    payload.push(1);
    payload.push(3); // stack major
    payload.push(0); // stack minor
    payload.extend_from_slice(&0x1234_u16.to_le_bytes()); // stack build
    payload.extend_from_slice(&stack_svn.to_le_bytes());
    payload.push(1); // sdk major
    payload.push(0);
    payload.extend_from_slice(&0_u16.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.push(2); // DFU version
    payload
}

/// `[load_addr, load_addr + size)` 的 boot 信息。
fn boot_info(load_addr: u32, size: u32) -> BootInfo {
    BootInfo {
        bin_size: size,
        checksum: 0x0102_0304,
        load_addr,
        run_addr: load_addr,
        xqspi_xip_cmd: 0x03,
        ..BootInfo::default()
    }
}

/// `[load_addr, load_addr + size)` 的镜像信息。
fn img_info(load_addr: u32, size: u32) -> ImgInfo {
    ImgInfo::new(1, boot_info(load_addr, size), "device")
}

/// 序列化后的 boot 信息字节。
fn boot_info_bytes(load_addr: u32, size: u32) -> [u8; BOOT_INFO_LEN] {
    boot_info(load_addr, size).to_bytes()
}

// ----------------------------------------------------------------------
// 帧与固件解析（黄金字节测试）
// ----------------------------------------------------------------------

#[test]
fn get_info_frame_is_byte_exact() {
    let frame = BasicFrame::new(opcode::GET_INFO, Vec::new());
    assert_eq!(
        frame.encode(),
        vec![0x44, 0x47, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00]
    );
}

#[test]
fn program_end_frame_matches_official_checksum() {
    let mut params = vec![0x01];
    params.extend_from_slice(&0x1122_3344_u32.to_le_bytes());
    let encoded = BasicFrame::new(opcode::PROGRAM_END, params).encode();
    assert_eq!(&encoded[..6], &[0x44, 0x47, 0x25, 0x00, 0x05, 0x00]);

    let decoded = BasicFrame::decode(&encoded).expect("frame must decode");
    assert_eq!(decoded.opcode, opcode::PROGRAM_END);
    assert_eq!(decoded.payload[0], 0x01);
    assert_eq!(
        u32::from_le_bytes([
            decoded.payload[1],
            decoded.payload[2],
            decoded.payload[3],
            decoded.payload[4]
        ]),
        0x1122_3344
    );
}

#[test]
fn corrupt_checksum_is_rejected() {
    let mut encoded = BasicFrame::new(opcode::GET_INFO, Vec::new()).encode();
    let last = encoded.len() - 1;
    encoded[last] ^= 0xFF;
    assert!(BasicFrame::decode(&encoded).is_err());
}

#[test]
fn wrong_magic_is_rejected() {
    let mut frame = BasicFrame::new(opcode::GET_INFO, Vec::new()).encode();
    frame[0] = 0x45;
    assert!(BasicFrame::decode(&frame).is_err());
}

#[test]
fn fast_frame_layout_is_byte_exact() {
    let frame = FastFrame::new(fast::GET_VERSION, Vec::new());
    assert_eq!(frame.encode(), vec![0x44, 0x4F, 0x4F, 0x47, 0x0B]);

    let erase = FastFrame::new(fast::ERASE_FLASH, vec![0x00, 0x00, 0x01, 0x00]);
    let decoded = FastFrame::decode(&erase.encode()).expect("fast frame must decode");
    assert_eq!(decoded.opcode, fast::ERASE_FLASH);
    assert_eq!(decoded.params, vec![0x00, 0x00, 0x01, 0x00]);
}

#[test]
fn boot_info_config_round_trip() {
    let info = BootInfo {
        bin_size: 0x1234,
        checksum: 0xDEAD_BEEF,
        load_addr: 0x0001_0000,
        run_addr: 0x0001_0101,
        xqspi_xip_cmd: 0x0B,
        xqspi_speed: 4,
        code_copy_mode: 1,
        system_clk: 5,
        check_image: 1,
        boot_delay: 0,
        is_dap_boot: 1,
    };
    let bytes = info.to_bytes();
    assert_eq!(bytes.len(), BOOT_INFO_LEN);

    // speed=4, copy=1, clk=5, check=1, dap=1 => 0x0000_05B4
    assert_eq!(
        u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        0x5B4
    );

    let parsed = BootInfo::from_bytes(&bytes).expect("boot info must parse");
    assert_eq!(parsed, info);
}

#[test]
fn img_info_round_trip_and_comments() {
    let info = ImgInfo::new(0x0102, boot_info(0x0001_0000, 0x2000), "hello");
    let bytes = info.to_bytes();
    assert_eq!(bytes.len(), 40);
    assert_eq!(&bytes[0..2], &[0x44, 0x47]);

    let parsed = ImgInfo::from_bytes(&bytes).expect("img info must parse");
    assert_eq!(parsed, info);
    assert_eq!(parsed.comments_str(), "hello");
}

#[test]
fn plain_firmware_is_parsed() {
    let data = firmware_bytes(0x0001_0000, 64);
    let image = DfuImage::parse(data.clone()).expect("firmware must parse");
    assert_eq!(image.len(), data.len());
    assert_eq!(image.boot_info().load_addr, 0x0001_0000);
    assert!(!image.signed);
    assert!(!image.encrypted);
    assert_eq!(
        image.checksum,
        data.iter()
            .map(|b| u32::from(*b))
            .fold(0_u32, |a, b| a.wrapping_add(b))
    );
}

#[test]
fn signed_firmware_offsets_are_detected() {
    let plain = signed_firmware_bytes(0x0001_0000, 64, false);
    let image = DfuImage::parse(plain).expect("signed firmware must parse");
    assert!(image.signed);
    assert!(!image.encrypted);

    let encrypted = signed_firmware_bytes(0x0001_0000, 64, true);
    let image = DfuImage::parse(encrypted).expect("encrypted firmware must parse");
    assert!(image.signed);
    assert!(image.encrypted);
}

#[test]
fn invalid_firmware_is_rejected() {
    let data = vec![0xAA_u8; 128];
    assert!(DfuImage::parse(data).is_err());
    assert!(DfuImage::parse(Vec::new()).is_err());
}

// ----------------------------------------------------------------------
// 升级选项：单区 / 双区
// ----------------------------------------------------------------------

fn build_options(
    image: &DfuImage,
    bank_mode: Option<&str>,
    address: Option<&str>,
) -> TargetAddress {
    let options = OtaStartOptions {
        bank_mode: bank_mode.map(str::to_string),
        target_address: address.map(str::to_string),
        ..OtaStartOptions::default()
    };
    super::build_transfer_options(&options, image)
        .expect("options must build")
        .target
}

#[test]
fn default_bank_mode_is_dual_with_computed_address() {
    let image = DfuImage::parse(firmware_bytes(APP_LOAD, 64)).expect("firmware must parse");
    assert_eq!(
        build_options(&image, None, None),
        TargetAddress::Explicit(APP_LOAD + 0x1000)
    );
}

#[test]
fn single_bank_ignores_write_address() {
    let image = DfuImage::parse(firmware_bytes(APP_LOAD, 64)).expect("firmware must parse");
    assert_eq!(
        build_options(&image, Some("single"), Some("0x00020000")),
        TargetAddress::LoadAddress
    );
}

#[test]
fn dual_bank_accepts_explicit_aligned_address() {
    let image = DfuImage::parse(firmware_bytes(APP_LOAD, 64)).expect("firmware must parse");
    assert_eq!(
        build_options(&image, Some("dual"), Some("0x00020000")),
        TargetAddress::Explicit(0x0002_0000)
    );
}

#[test]
fn dual_bank_rejects_invalid_address() {
    let image = DfuImage::parse(firmware_bytes(APP_LOAD, 64)).expect("firmware must parse");
    let options = |mode: &str, address: &str| OtaStartOptions {
        bank_mode: Some(mode.to_string()),
        target_address: Some(address.to_string()),
        ..OtaStartOptions::default()
    };
    // 未按 4K 对齐、与加载地址相同、未知模式。
    assert!(super::build_transfer_options(&options("dual", "0x00011800"), &image).is_err());
    assert!(super::build_transfer_options(&options("dual", "0x00010000"), &image).is_err());
    assert!(super::build_transfer_options(&options("triple", "0x00012000"), &image).is_err());
}

#[test]
fn overlap_rules_match_the_official_sdk() {
    let sca = 0x0020_0000;
    let bootloader = boot_info(0x0000_0000, 0x8000);
    let app = boot_info(0x0001_0000, 0x2000);

    // 原地更新允许。
    assert!(check_overlap_v2(
        true,
        false,
        app.bin_size,
        app.load_addr,
        sca,
        &bootloader,
        Some(&app),
        Some(&app)
    )
    .is_ok());

    // 写入 SCA 被拒绝。
    let error = check_overlap_v2(
        true,
        false,
        app.bin_size,
        sca,
        sca,
        &bootloader,
        Some(&app),
        Some(&app),
    );
    assert!(error.is_err());

    // 拷贝模式不得与运行中应用重叠。
    let copy_error = check_overlap_v2(
        true,
        false,
        app.bin_size,
        app.load_addr + 0x1000,
        sca,
        &bootloader,
        Some(&app),
        Some(&boot_info(0x0002_0000, 0x2000)),
    );
    assert!(copy_error.is_err());

    // DFU Bootloader 运行时的 V1 原地更新允许。
    let dfu_boot = boot_info(0x0000_0000, 0x8000);
    assert!(check_overlap_v1(
        true,
        false,
        app.bin_size,
        app.load_addr,
        0x0100_0000,
        &dfu_boot,
        Some(&app)
    )
    .is_ok());
}

#[test]
fn mac_bump_matches_app_bootloader_behaviour() {
    assert_eq!(
        bump_last_mac_byte("AA:BB:CC:DD:EE:FE").as_deref(),
        Some("AA:BB:CC:DD:EE:FF")
    );
    assert_eq!(
        bump_last_mac_byte("AA:BB:CC:DD:EE:FF").as_deref(),
        Some("AA:BB:CC:DD:EE:00")
    );
    assert!(bump_last_mac_byte("not-a-mac").is_none());
}

// ----------------------------------------------------------------------
// 端到端会话（脚本化 mock 传输）
// ----------------------------------------------------------------------

fn test_config() -> EngineConfig {
    EngineConfig {
        command_timeout: Duration::from_millis(500),
        fast_command_timeout: Duration::from_millis(500),
        fast_settle: Duration::from_millis(1),
        set_dfu_mode_wait: Duration::from_millis(1),
        reconnect_delay: Duration::from_millis(1),
        post_reconnect_wait: Duration::from_millis(1),
        ..EngineConfig::default()
    }
}

fn normal_options(mode: TransferMode) -> TransferOptions {
    TransferOptions {
        mode,
        target: TargetAddress::LoadAddress,
        run_immediately: true,
        ..TransferOptions::default()
    }
}

fn basic_handler(
    fast_flag: Arc<Mutex<bool>>,
    fast_total: usize,
) -> impl Fn(Channel, &[u8]) -> Vec<(Channel, Vec<u8>)> + Send + Sync {
    let raw_written = Arc::new(Mutex::new(0_usize));
    move |channel, bytes| {
        if channel != Channel::DfuWrite {
            return Vec::new();
        }
        let is_basic = bytes.len() >= 8 && bytes[0] == 0x44 && bytes[1] == 0x47;
        if !is_basic {
            // 快速模式固件流：全部字节到达后应答一次。
            let complete = match raw_written.lock() {
                Ok(mut written) => {
                    *written += bytes.len();
                    *written >= fast_total
                }
                Err(_) => false,
            };
            if complete {
                return vec![(
                    Channel::DfuNotify,
                    BasicFrame::new(opcode::PROGRAM_FLASH_FAST, vec![1]).encode(),
                )];
            }
            return Vec::new();
        }
        let frame = match BasicFrame::decode(bytes) {
            Ok(frame) => frame,
            Err(_) => return Vec::new(),
        };

        let responses: Vec<Vec<u8>> = match frame.opcode {
            opcode::GET_INFO => {
                vec![BasicFrame::new(opcode::GET_INFO, chip_info_payload(STACK_SVN)).encode()]
            }
            opcode::SYSTEM_CONFIG => {
                let payload = &frame.payload;
                if payload.len() < 7 {
                    return Vec::new();
                }
                let op = payload[0];
                let address = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
                let length = u16::from_le_bytes([payload[5], payload[6]]);
                let data = if usize::from(length) == BOOT_INFO_LEN {
                    boot_info_bytes(BOOT_LOAD, BOOT_SIZE).to_vec()
                } else {
                    vec![0xFF_u8; usize::from(length)]
                };
                let mut response = vec![1, op];
                response.extend_from_slice(&address.to_le_bytes());
                response.extend_from_slice(&length.to_le_bytes());
                response.extend_from_slice(&data);
                vec![BasicFrame::new(opcode::SYSTEM_CONFIG, response).encode()]
            }
            opcode::GET_FW_INFO => {
                let mut response = vec![1];
                response.extend_from_slice(&0x0001_8000_u32.to_le_bytes());
                response.push(0); // Bootloader 在运行
                response.extend_from_slice(&img_info(APP_LOAD, APP_SIZE).to_bytes());
                vec![BasicFrame::new(opcode::GET_FW_INFO, response).encode()]
            }
            opcode::SET_DFU_MODE => Vec::new(),
            opcode::PROGRAM_START => {
                let is_fast = frame
                    .payload
                    .first()
                    .map(|byte| byte & 0x0F == 0x02)
                    .unwrap_or(false);
                if let Ok(mut flag) = fast_flag.lock() {
                    *flag = is_fast;
                }
                if let Ok(mut written) = raw_written.lock() {
                    *written = 0;
                }
                if is_fast {
                    vec![
                        BasicFrame::new(
                            opcode::PROGRAM_START,
                            vec![1, 1, 4, 0], // started, total = 4
                        )
                        .encode(),
                        BasicFrame::new(opcode::PROGRAM_START, vec![1, 2, 1, 0]).encode(),
                        BasicFrame::new(opcode::PROGRAM_START, vec![1, 2, 2, 0]).encode(),
                        BasicFrame::new(opcode::PROGRAM_START, vec![1, 3, 0, 0]).encode(),
                    ]
                } else {
                    vec![BasicFrame::new(opcode::PROGRAM_START, vec![1]).encode()]
                }
            }
            opcode::PROGRAM_FLASH => {
                vec![BasicFrame::new(opcode::PROGRAM_FLASH, vec![1]).encode()]
            }
            opcode::PROGRAM_END => {
                let is_fast = fast_flag.lock().map(|flag| *flag).unwrap_or(false);
                let mut response = vec![1];
                if is_fast && frame.payload.len() >= 5 {
                    response.extend_from_slice(&frame.payload[1..5]);
                }
                vec![BasicFrame::new(opcode::PROGRAM_END, response).encode()]
            }
            _ => Vec::new(),
        };

        responses
            .into_iter()
            .map(|response| (Channel::DfuNotify, response))
            .collect()
    }
}

fn collect_events(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<ProgressEvent>,
) -> Vec<ProgressEvent> {
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    events
}

fn is_basic_opcode(write: &[u8], opcode_value: u16) -> bool {
    write.len() >= 4
        && write[0] == 0x44
        && write[1] == 0x47
        && u16::from_le_bytes([write[2], write[3]]) == opcode_value
}

#[tokio::test]
async fn v2_normal_firmware_update() {
    let data = firmware_bytes(APP_LOAD, 64);
    let image = DfuImage::parse(data.clone()).expect("firmware must parse");
    let fast_flag = Arc::new(Mutex::new(false));
    let mock = Arc::new(MockTransport::new(
        v2_capabilities(247),
        basic_handler(Arc::clone(&fast_flag), data.len()),
    ));

    let engine = DfuEngine::new(Arc::clone(&mock) as Arc<dyn Transport>, test_config());

    let (sink, mut rx) = ProgressSink::channel();
    let report = engine
        .execute(
            DfuPlan::firmware(image, normal_options(TransferMode::Normal)),
            sink,
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .expect("session must succeed");

    assert_eq!(report.protocol_version, 2);
    assert_eq!(report.bytes_transferred, data.len() as u64);
    assert!(mock.connect_count() >= 1);

    let writes = mock.writes();
    assert!(writes
        .iter()
        .any(|write| is_basic_opcode(&write.data, opcode::PROGRAM_START)));
    assert!(writes
        .iter()
        .any(|write| is_basic_opcode(&write.data, opcode::PROGRAM_FLASH)));
    assert!(writes
        .iter()
        .any(|write| is_basic_opcode(&write.data, opcode::PROGRAM_END)));

    let events = collect_events(&mut rx);
    assert!(events
        .iter()
        .any(|event| matches!(event, ProgressEvent::Stage(Stage::Program))));
    assert!(events
        .iter()
        .any(|event| matches!(event, ProgressEvent::Transfer { .. })));
    assert!(events
        .iter()
        .any(|event| matches!(event, ProgressEvent::Completed(_))));
}

#[tokio::test]
async fn v2_fast_firmware_update_reports_erase_progress() {
    let data = firmware_bytes(APP_LOAD, 4096);
    let data_len = data.len();
    let image = DfuImage::parse(data).expect("firmware must parse");
    let fast_flag = Arc::new(Mutex::new(false));
    let mock = Arc::new(MockTransport::new(
        v2_capabilities(247),
        basic_handler(Arc::clone(&fast_flag), data_len),
    ));

    let engine = DfuEngine::new(Arc::clone(&mock) as Arc<dyn Transport>, test_config());

    let (sink, mut rx) = ProgressSink::channel();
    let report = engine
        .execute(
            DfuPlan::firmware(image, normal_options(TransferMode::Fast)),
            sink,
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .expect("fast session must succeed");

    assert_eq!(report.protocol_version, 2);
    assert!(*fast_flag.lock().expect("flag lock"));

    let events = collect_events(&mut rx);
    assert!(events.iter().any(|event| matches!(
        event,
        ProgressEvent::Erase {
            erased: 2,
            total: 4
        }
    )));
}

#[tokio::test]
async fn v1_firmware_update_uses_image_list_cleanup() {
    let data = firmware_bytes(APP_LOAD, 2048);
    let data_len = data.len();
    let image = DfuImage::parse(data.clone()).expect("firmware must parse");
    let mock = Arc::new(MockTransport::with_address(
        v1_capabilities(247),
        "AA:BB:CC:DD:EE:FF",
        basic_handler(Arc::new(Mutex::new(false)), data_len),
    ));

    let engine = DfuEngine::new(Arc::clone(&mock) as Arc<dyn Transport>, test_config());

    let report = engine
        .run(DfuPlan::firmware(
            image,
            normal_options(TransferMode::Normal),
        ))
        .await
        .expect("V1 session must succeed");

    assert_eq!(report.protocol_version, 1);
    let writes = mock.writes();
    let system_configs = writes
        .iter()
        .filter(|write| is_basic_opcode(&write.data, opcode::SYSTEM_CONFIG))
        .count();
    assert!(
        system_configs >= 3,
        "expected boot info, image list read and image list write, got {system_configs}"
    );
}

#[tokio::test]
async fn fast_service_firmware_update() {
    let data = firmware_bytes(APP_LOAD, 4096);
    let checksum = DfuImage::parse(data.clone())
        .expect("firmware must parse")
        .checksum;
    let image = DfuImage::parse(data.clone()).expect("firmware must parse");

    let mock = Arc::new(MockTransport::new(
        fast_service_capabilities(247),
        move |channel, bytes| match channel {
            Channel::FastCommand => {
                let frame = match FastFrame::decode(bytes) {
                    Ok(frame) => frame,
                    Err(_) => return Vec::new(),
                };
                let ack = |opcode_value: u8, params: Vec<u8>| {
                    (
                        Channel::FastCommand,
                        FastFrame::new(opcode_value, params).encode(),
                    )
                };
                match frame.opcode {
                    fast::GET_VERSION => vec![ack(fast::GET_VERSION, vec![2])],
                    fast::GET_BUFFER_SIZE => {
                        vec![ack(fast::GET_BUFFER_SIZE, 512_u32.to_le_bytes().to_vec())]
                    }
                    fast::SELECT_FLASH_TYPE => vec![ack(fast::SELECT_FLASH_TYPE, Vec::new())],
                    fast::ERASE_FLASH => vec![
                        ack(fast::ERASE_FLASH, vec![1, 1, 0]),
                        ack(fast::ERASE_FLASH, vec![2, 1, 0]),
                        ack(fast::ERASE_FLASH, vec![3, 0, 0]),
                    ],
                    fast::FLUSH_FLASH => vec![ack(fast::FLUSH_FLASH, Vec::new())],
                    fast::VERIFY_CHECKSUM => {
                        vec![ack(fast::VERIFY_CHECKSUM, frame.params.clone())]
                    }
                    fast::WRITE_BOOT => Vec::new(),
                    other => vec![ack(other, Vec::new())],
                }
            }
            Channel::FastData => vec![(
                Channel::FastCommand,
                FastFrame::new(fast::NEXT_BUFFER, Vec::new()).encode(),
            )],
            _ => Vec::new(),
        },
    ));

    let engine = DfuEngine::new(Arc::clone(&mock) as Arc<dyn Transport>, test_config());

    let report = engine
        .run(DfuPlan::firmware(
            image,
            normal_options(TransferMode::Normal),
        ))
        .await
        .expect("fast service session must succeed");

    assert_eq!(report.protocol_version, 2);
    assert_eq!(report.checksum, checksum);
    assert_eq!(report.bytes_transferred, data.len() as u64);
}

#[tokio::test]
async fn cancelled_session_does_not_connect() {
    let mock = MockTransport::silent(v2_capabilities(247));
    let engine = DfuEngine::new(Arc::clone(&mock) as Arc<dyn Transport>, test_config());

    let image = DfuImage::parse(firmware_bytes(APP_LOAD, 64)).expect("firmware");
    let error = engine
        .execute(
            DfuPlan::firmware(image, normal_options(TransferMode::Normal)),
            ProgressSink::none(),
            Arc::new(AtomicBool::new(true)),
        )
        .await
        .expect_err("session must be cancelled");

    assert!(matches!(error, DfuError::Cancelled));
    assert_eq!(mock.connect_count(), 0);
}
