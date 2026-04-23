//! GH Protocol Loopback Test Program
//!
//! 本地回环测试程序，在同一进程中运行Client和Server，使用内存通道通信

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rpc::{InvokeContext, LogCallback, LogLevel, RpcConfig, RpcCore, RpcError, SendFunction};
use gh_rpc::{FrameDecoder, GhFuncFrame, KEY_G, KEY_GH3X_GET_VERSION, KEY_GH3X_REGS_LIST_WRITE_CMD, KEY_GH3X_REGS_READ_CMD, KEY_GH3X_REGS_WRITE_CMD, FMT_GH3X_GET_VERSION, FMT_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REGS_READ_CMD, FMT_GH3X_REGS_WRITE_CMD};
use tokio::sync::mpsc;

#[derive(Debug, Default)]
struct TestStats {
    total: AtomicUsize,
    passed: AtomicUsize,
    failed: AtomicUsize,
}

impl TestStats {
    fn new() -> Self {
        Self::default()
    }

    fn record_pass(&self, test_name: &str) {
        self.total.fetch_add(1, Ordering::SeqCst);
        self.passed.fetch_add(1, Ordering::SeqCst);
        println!("[PASS] {}", test_name);
    }

    fn record_fail(&self, test_name: &str, error: &str) {
        self.total.fetch_add(1, Ordering::SeqCst);
        self.failed.fetch_add(1, Ordering::SeqCst);
        println!("[FAIL] {}: {}", test_name, error);
    }

    fn print_summary(&self) {
        let total = self.total.load(Ordering::SeqCst);
        let passed = self.passed.load(Ordering::SeqCst);
        let failed = self.failed.load(Ordering::SeqCst);
        println!("\n========== Test Summary ==========");
        println!("Total:  {}", total);
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        println!("==================================");
    }
}

struct TestLogger {
    prefix: String,
}

impl TestLogger {
    fn new(prefix: &str) -> Self {
        Self { prefix: prefix.to_string() }
    }
}

impl LogCallback for TestLogger {
    fn log(&self, level: LogLevel, context: &str, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        println!("[{}][{}][{:?}][{}] {}", timestamp, self.prefix, level, context, message);
    }
}

fn print_frame_info(index: usize, frame: &GhFuncFrame) {
    println!("    帧[{}]: cnt={}, ts={}, id={:?}, ch_num={}",
        index, frame.frame_cnt, frame.timestamp, frame.id, frame.ch_num);
}

async fn setup_server(
    server_core: &RpcCore,
    frame_decoder: FrameDecoder,
) -> Result<(), RpcError> {
    println!("[Server] 注册命令处理器...");

    let handler = Arc::new(move |_data: &[u8], size: usize, ctx: &mut InvokeContext| {
        println!("[Server] GH3X_GetVersion 调用, 数据长度: {}", size);
        let version_info = b"GH3X_v1.0.0_Server";
        ctx.set_response(version_info.to_vec());
        println!("[Server] 返回版本信息: {:?}", String::from_utf8_lossy(version_info));
    });
    server_core.register(KEY_GH3X_GET_VERSION, handler).await?;

    let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
        println!("[Server] GH3X_RegsWriteCmd 调用, 数据长度: {}", size);
        let regs: Vec<u16> = data.chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        println!("[Server] 写入寄存器: {} 个", regs.len() / 2);
        ctx.set_response(vec![0x00]);
    });
    server_core.register(KEY_GH3X_REGS_WRITE_CMD, handler).await?;

    let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
        println!("[Server] GH3X_RegsReadCmd 调用, 数据长度: {}", size);
        if data.len() >= 6 {
            let reg_addr = u16::from_le_bytes([data[0], data[1]]);
            let read_len = i32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
            println!("[Server] 读取寄存器: 地址=0x{:04X}, 长度={}", reg_addr, read_len);

            let mut response = Vec::with_capacity(read_len * 2);
            for i in 0..read_len {
                let value = (0x1000 + i as u16) as u16;
                response.extend_from_slice(&value.to_le_bytes());
            }
            ctx.set_response(response);
        } else {
            println!("[Server] GH3X_RegsReadCmd 数据长度不足");
            ctx.set_response(vec![]);
        }
    });
    server_core.register(KEY_GH3X_REGS_READ_CMD, handler).await?;

    let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
        println!("[Server] GH3X_ChipCtrl 调用, 数据长度: {}", size);
        println!("[Server] 芯片控制: 0x{:02X}", data.get(0).copied().unwrap_or(0));
        ctx.set_response(vec![0x00]);
    });
    server_core.register("GH3X_ChipCtrl", handler).await?;

    let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
        println!("[Server] GH3X_RegsListWriteCmd 调用, 数据长度: {}", size);
        let regs: Vec<u16> = data.chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        println!("[Server] 批量写入寄存器: {} 个", regs.len() / 2);
        ctx.set_response(vec![0x00]);
    });
    server_core.register(KEY_GH3X_REGS_LIST_WRITE_CMD, handler).await?;

    let fd = frame_decoder.clone();
    let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
        println!("[Server] G协议数据接收, 数据长度: {}", size);
        match fd.decode_frames(data) {
            Ok(frames) => {
                println!("[Server] 解码到 {} 个帧", frames.len());
                for (i, frame) in frames.iter().enumerate() {
                    print_frame_info(i, frame);
                }
            }
            Err(e) => {
                println!("[Server] G协议解码失败: {:?}", e);
            }
        }
        ctx.set_response(vec![]);
    });
    server_core.register(KEY_G, handler).await?;

    println!("[Server] 命令处理器注册完成");
    Ok(())
}

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("  GH Protocol Loopback Test Program");
    println!("========================================\n");

    let stats = Arc::new(TestStats::new());

    println!("[Setup] 创建通信通道...");
    let (client_to_server, mut server_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (server_to_client, mut client_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    println!("[Setup] 初始化Server...");
    let server_config = RpcConfig {
        timeout_ms: 5000,
        retry_count: 3,
        retry_delay_ms: 100,
        frame_size: 240,
    };
    let server_logger = Arc::new(TestLogger::new("Server"));
    let server_core = Arc::new(RpcCore::new(server_config).with_logger(server_logger));
    let frame_decoder = FrameDecoder::new();

    setup_server(&server_core, frame_decoder).await.expect("Failed to setup server");

    let server_to_client_clone = server_to_client.clone();
    let server_send_fn: SendFunction = Arc::new(move |data: &[u8]| -> Result<(), RpcError> {
        server_to_client_clone.send(data.to_vec()).map_err(|_| RpcError::SendFail)?;
        println!("[Server] 发送响应 {} 字节", data.len());
        Ok(())
    });
    server_core.set_send_function(server_send_fn).await;

    println!("[Setup] 启动Server接收任务...");
    let server_core_clone = server_core.clone();
    tokio::spawn(async move {
        while let Some(data) = server_rx.recv().await {
            println!("[Server] 收到 {} 字节", data.len());
            let results = server_core_clone.process(&data).await;
            for result in results {
                match result {
                    Ok(parse_result) => {
                        println!("[Server] 解析成功: key={}, len={}", parse_result.key, parse_result.param.len());
                    }
                    Err(e) => {
                        println!("[Server] 解析失败: {:?}", e);
                    }
                }
            }
        }
    });

    println!("[Setup] 初始化Client...");
    let client_config = RpcConfig {
        timeout_ms: 1000,
        retry_count: 3,
        retry_delay_ms: 200,
        frame_size: 240,
    };
    let client_logger = Arc::new(TestLogger::new("Client"));
    let client_core = Arc::new(RpcCore::new(client_config).with_logger(client_logger));

    let client_to_server_clone = client_to_server.clone();
    let send_fn: SendFunction = Arc::new(move |data: &[u8]| -> Result<(), RpcError> {
        client_to_server_clone.send(data.to_vec()).map_err(|_| RpcError::SendFail)?;
        println!("[Client] 发送 {} 字节", data.len());
        Ok(())
    });
    client_core.set_send_function(send_fn).await;

    println!("[Setup] 启动Client接收任务...");
    let client_core_clone = client_core.clone();
    tokio::spawn(async move {
        while let Some(data) = client_rx.recv().await {
            let results = client_core_clone.process(&data).await;
            for result in results {
                match result {
                    Ok(parse_result) => {
                        println!("[Client] 响应解析成功: key={}, len={}", parse_result.key, parse_result.param.len());
                    }
                    Err(e) => {
                        println!("[Client] 响应解析失败: {:?}", e);
                    }
                }
            }
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("\n========================================");
    println!("  开始执行测试用例");
    println!("========================================");

    let stats_ref = &stats;
    let client_core_ref = &client_core;

    async fn test_get_version(core: &RpcCore, stats: &TestStats) {
        let test_name = "test_get_version";
        println!("\n>>> Running {}...", test_name);

        match core.call(KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION, &[0]).await {
            Ok(response) => {
                println!("    Version: {:?}", String::from_utf8_lossy(&response));
                stats.record_pass(test_name);
            }
            Err(e) => {
                stats.record_fail(test_name, &e.to_string());
            }
        }
    }

    async fn test_regs_write(core: &RpcCore, stats: &TestStats) {
        let test_name = "test_regs_write";
        println!("\n>>> Running {}...", test_name);

        let regs = vec![0x1000u16, 0x1234, 0x1002, 0x5678];
        let mut data = Vec::new();
        for reg in regs {
            data.extend_from_slice(&reg.to_le_bytes());
        }

        match core.call(KEY_GH3X_REGS_WRITE_CMD, FMT_GH3X_REGS_WRITE_CMD, &data).await {
            Ok(_) => {
                println!("    Registers written successfully");
                stats.record_pass(test_name);
            }
            Err(e) => {
                stats.record_fail(test_name, &e.to_string());
            }
        }
    }

    async fn test_regs_read(core: &RpcCore, stats: &TestStats) {
        let test_name = "test_regs_read";
        println!("\n>>> Running {}...", test_name);

        let mut data = Vec::new();
        data.extend_from_slice(&0x1000u16.to_le_bytes());
        data.extend_from_slice(&10i32.to_le_bytes());

        match core.call(KEY_GH3X_REGS_READ_CMD, FMT_GH3X_REGS_READ_CMD, &data).await {
            Ok(response) => {
                let values: Vec<u16> = response.chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                println!("    Read {} registers: {:?}", values.len(), &values[..std::cmp::min(5, values.len())]);
                stats.record_pass(test_name);
            }
            Err(e) => {
                stats.record_fail(test_name, &e.to_string());
            }
        }
    }

    async fn test_chip_ctrl(core: &RpcCore, stats: &TestStats) {
        let test_name = "test_chip_ctrl";
        println!("\n>>> Running {}...", test_name);

        match core.call("GH3X_ChipCtrl", "", &[0x01]).await {
            Ok(_) => {
                println!("    Chip control executed successfully");
                stats.record_pass(test_name);
            }
            Err(e) => {
                stats.record_fail(test_name, &e.to_string());
            }
        }
    }

    async fn test_regs_list_write_single_frame(core: &RpcCore, stats: &TestStats) {
        let test_name = "test_regs_list_write_single_frame";
        println!("\n>>> Running {}...", test_name);

        let mut data = Vec::new();
        for i in 0..10u16 {
            data.extend_from_slice(&(0x1000 + i).to_le_bytes());
            data.extend_from_slice(&(0x1000 + i).to_le_bytes());
        }

        println!("    Writing {} register pairs ({} bytes)", 10, data.len());

        match core.call(KEY_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REGS_LIST_WRITE_CMD, &data).await {
            Ok(_) => {
                println!("    Single frame write completed successfully");
                stats.record_pass(test_name);
            }
            Err(e) => {
                stats.record_fail(test_name, &e.to_string());
            }
        }
    }

    async fn test_regs_list_write_multi_frame(core: &RpcCore, stats: &TestStats) {
        let test_name = "test_regs_list_write_multi_frame";
        println!("\n>>> Running {}...", test_name);

        let mut data = Vec::new();
        for i in 0..100u16 {
            data.extend_from_slice(&(0x1000 + (i % 100)).to_le_bytes());
            data.extend_from_slice(&(0x2000 + i).to_le_bytes());
        }

        println!("    Writing {} register pairs ({} bytes)", 100, data.len());

        match core.call(KEY_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REGS_LIST_WRITE_CMD, &data).await {
            Ok(_) => {
                println!("    Multi frame write completed successfully");
                stats.record_pass(test_name);
            }
            Err(e) => {
                stats.record_fail(test_name, &e.to_string());
            }
        }
    }

    async fn test_publish(core: &RpcCore, stats: &TestStats) {
        let test_name = "test_publish";
        println!("\n>>> Running {}...", test_name);

        match core.publish("GH3X_ChipCtrl", "", &[0x01]).await {
            Ok(()) => {
                println!("    Publish completed (no response expected)");
                stats.record_pass(test_name);
            }
            Err(e) => {
                stats.record_fail(test_name, &e.to_string());
            }
        }
    }

    async fn test_timeout_retry(tx: mpsc::UnboundedSender<Vec<u8>>, stats: &TestStats) {
        let test_name = "test_timeout_retry";
        println!("\n>>> Running {}...", test_name);

        let config = RpcConfig {
            timeout_ms: 100,
            retry_count: 3,
            ..Default::default()
        };

        let logger = Arc::new(TestLogger::new("Client-NoServer"));
        let core = RpcCore::new(config).with_logger(logger);

        let send_fn: SendFunction = Arc::new(move |data: &[u8]| -> Result<(), RpcError> {
            tx.send(data.to_vec()).map_err(|_| RpcError::SendFail)?;
            Ok(())
        });

        core.set_send_function(send_fn).await;

        let start = std::time::Instant::now();
        match core.call(KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION, &[0]).await {
            Ok(_) => {
                stats.record_pass(test_name);
            }
            Err(e) => {
                let elapsed = start.elapsed();
                println!("    Expected timeout after {}ms, got: {:?}", elapsed.as_millis(), e);
                if matches!(e, RpcError::Timeout) {
                    println!("    Timeout mechanism verified");
                    stats.record_pass(test_name);
                } else {
                    stats.record_fail(test_name, &format!("Expected Timeout, got: {}", e));
                }
            }
        }
    }

    test_get_version(client_core_ref, stats_ref).await;
    test_regs_write(client_core_ref, stats_ref).await;
    test_regs_read(client_core_ref, stats_ref).await;
    test_chip_ctrl(client_core_ref, stats_ref).await;
    test_regs_list_write_single_frame(client_core_ref, stats_ref).await;
    test_regs_list_write_multi_frame(client_core_ref, stats_ref).await;
    test_publish(client_core_ref, stats_ref).await;
    test_timeout_retry(client_to_server.clone(), stats_ref).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    stats.print_summary();

    let passed = stats.passed.load(Ordering::SeqCst);
    let total = stats.total.load(Ordering::SeqCst);

    if passed == total {
        println!("\nAll tests passed!");
        std::process::exit(0);
    } else {
        println!("\nSome tests failed!");
        std::process::exit(1);
    }
}
