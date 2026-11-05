//! GH3220 真机 BLE 冒烟测试（手动触发，默认忽略）。
//!
//! 用法：
//! ```bash
//! source ~/.local/tauri-env.sh
//! GH3220_BLE_ADDRESS=<MAC> \
//!   cargo test --test gh3220_ble_manual -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::time::Duration;

use combridge_rust_lib::device::ble::BleManager;
use combridge_rust_lib::gh3220::{
    Gh3220CaptureConfig, Gh3220CsvConfig, Gh3220LinkConfig, Gh3220LinkKind, Gh3220Manager,
    Gh3220State,
};
use combridge_rust_lib::service::event_bus::EventBus;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "需要真机 GH3220 且设置 GH3220_BLE_ADDRESS"]
async fn ble_real_device_smoke() {
    let address = std::env::var("GH3220_BLE_ADDRESS").expect("请设置 GH3220_BLE_ADDRESS=<MAC>");
    let csv_dir = std::env::var("GH3220_CSV_DIR").unwrap_or_else(|_| "/tmp/gh3220-ble".to_string());
    let duration_ms: u64 = std::env::var("GH3220_CAPTURE_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let bus = Arc::new(EventBus::new(1024));
    let ble = Arc::new(BleManager::new(bus.clone()));
    ble.initialize().await.expect("BLE 初始化失败");
    ble.configure_native().await.expect("切换 BLE Native 失败");

    let devices = ble.scan(6000).await.expect("BLE 扫描失败");
    println!(
        "scan: {:?}",
        devices
            .iter()
            .map(|d| (d.address.clone(), d.name.clone()))
            .collect::<Vec<_>>()
    );
    assert!(
        devices
            .iter()
            .any(|d| d.address.eq_ignore_ascii_case(&address)),
        "扫描结果里没有目标设备 {address}"
    );

    let mgr = Arc::new(Gh3220Manager::new(ble, bus));
    mgr.start_event_bridge();
    mgr.set_csv_config(Gh3220CsvConfig {
        enabled: true,
        output_dir: csv_dir.clone(),
        idle_close_secs: 0,
    });

    let status = mgr
        .connect(Gh3220LinkConfig {
            kind: Gh3220LinkKind::Ble,
            address: address.clone(),
            write_uuid: None,
            notify_uuid: None,
            mtu: None,
        })
        .await
        .expect("连接 GH3220 失败");
    assert_eq!(status.state, Gh3220State::Ready);
    println!("connected: {status:?}");

    let info = mgr.device_info().await.expect("读取设备信息失败");
    println!("device: {info:?}");
    assert!(info.max_len > 0);

    mgr.capture_start(Gh3220CaptureConfig {
        auto_start: true,
        auto_stop: false,
        mode: 0,
        slot_en: 0,
        func_mask: 0x2,
        channels_override: None,
        csv_enabled: Some(true),
    })
    .await
    .expect("启动采集失败");

    tokio::time::sleep(Duration::from_millis(duration_ms)).await;

    let capture = mgr.capture_status();
    let stats = mgr.stats();
    println!("capture: {capture:?}");
    println!("stats: {stats:?}");
    assert!(capture.records > 0, "未收到数据");
    assert!(stats.frames > 0, "未解析到帧");

    mgr.capture_stop().await.expect("停止采集失败");
    mgr.disconnect().await.expect("断开失败");
}
