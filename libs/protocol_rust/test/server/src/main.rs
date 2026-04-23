//! GH Protocol Server Test Program
//!
//! Server端测试程序，用于接收和处理GH协议命令

use std::sync::Arc;
use std::time::Duration;

use gh_rpc::{
    FrameDecoder, GhFuncFrame,
    KEY_G, KEY_GH3X_GET_VERSION, KEY_GH3X_REGS_LIST_WRITE_CMD,
    KEY_GH3X_REGS_READ_CMD, KEY_GH3X_REGS_WRITE_CMD,
};
use log::{error, info, warn, LevelFilter};
use rpc::{InvokeContext, LogCallback, LogLevel, RpcCore, RpcConfig};
use serialport::{SerialPort, SerialPortType};
use tokio::sync::Mutex;

const SERIAL_PORT: &str = "COM10";
const BAUD_RATE: u32 = 115200;

struct ServerLogger;

impl LogCallback for ServerLogger {
    fn log(&self, level: LogLevel, context: &str, message: &str) {
        match level {
            LogLevel::Trace => log::trace!("[{}] {}", context, message),
            LogLevel::Debug => log::debug!("[{}] {}", context, message),
            LogLevel::Info => log::info!("[{}] {}", context, message),
            LogLevel::Warn => log::warn!("[{}] {}", context, message),
            LogLevel::Error => log::error!("[{}] {}", context, message),
        }
    }
}

struct ServerState {
    rpc_core: Arc<RpcCore>,
    frame_decoder: FrameDecoder,
    serial_port: Option<Box<dyn SerialPort>>,
}

impl ServerState {
    fn new() -> Self {
        let config = RpcConfig {
            timeout_ms: 5000,
            retry_count: 3,
            retry_delay_ms: 100,
            frame_size: 128,
        };

        let rpc_core = Arc::new(
            RpcCore::new(config)
                .with_logger(Arc::new(ServerLogger)),
        );

        Self {
            rpc_core,
            frame_decoder: FrameDecoder::new(),
            serial_port: None,
        }
    }

    async fn init_serial_port(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("正在初始化串口 {} @ {}...", SERIAL_PORT, BAUD_RATE);

        let port = serialport::new(SERIAL_PORT, BAUD_RATE)
            .timeout(Duration::from_millis(100))
            .open();

        match port {
            Ok(p) => {
                info!("串口 {} 打开成功", SERIAL_PORT);
                self.serial_port = Some(p);
                Ok(())
            }
            Err(e) => {
                warn!("无法打开串口 {}: {}, 将使用标准输入输出模式", SERIAL_PORT, e);
                self.serial_port = None;
                Ok(())
            }
        }
    }

    async fn register_handlers(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("注册命令处理器...");

        let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
            info!("[{}] GH3X_GetVersion 调用, 数据长度: {}", ctx.topic, size);

            let version_info = b"GH3X_v1.0.0_Server";
            ctx.set_response(version_info.to_vec());

            info!("[{}] 返回版本信息: {:?}", ctx.topic, String::from_utf8_lossy(version_info));
        });
        self.rpc_core.register(KEY_GH3X_GET_VERSION, handler).await?;

        let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
            info!("[{}] GH3X_RegsWriteCmd 调用, 数据长度: {}", ctx.topic, size);

            let regs: Vec<u16> = data.chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();

            info!("[{}] 写入寄存器: {:?}", ctx.topic, regs);

            ctx.set_response(vec![]);
        });
        self.rpc_core.register(KEY_GH3X_REGS_WRITE_CMD, handler).await?;

        let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
            info!("[{}] GH3X_RegsReadCmd 调用, 数据长度: {}", ctx.topic, size);

            if data.len() >= 6 {
                let reg_addr = u16::from_le_bytes([data[0], data[1]]);
                let read_len = i32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;

                info!("[{}] 读取寄存器: 地址=0x{:04X}, 长度={}", ctx.topic, reg_addr, read_len);

                let mut response = Vec::with_capacity(read_len * 2);
                for i in 0..read_len {
                    let value = (0x1000 + i as u16) as u16;
                    response.extend_from_slice(&value.to_le_bytes());
                }
                ctx.set_response(response);
            } else {
                warn!("[{}] GH3X_RegsReadCmd 数据长度不足", ctx.topic);
                ctx.set_response(vec![]);
            }
        });
        self.rpc_core.register(KEY_GH3X_REGS_READ_CMD, handler).await?;

        let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
            info!("[{}] GH3X_RegsListWriteCmd 调用, 数据长度: {}", ctx.topic, size);

            let regs: Vec<u16> = data.chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();

            info!("[{}] 批量写入寄存器: {:?}", ctx.topic, regs);

            ctx.set_response(vec![]);
        });
        self.rpc_core.register(KEY_GH3X_REGS_LIST_WRITE_CMD, handler).await?;

        let frame_decoder = self.frame_decoder.clone();
        let handler = Arc::new(move |data: &[u8], size: usize, ctx: &mut InvokeContext| {
            info!("[{}] G协议数据接收, 数据长度: {}", ctx.topic, size);

            match frame_decoder.decode_frames(data) {
                Ok(frames) => {
                    info!("[{}] 解码到 {} 个帧", ctx.topic, frames.len());
                    for (i, frame) in frames.iter().enumerate() {
                        print_frame_info(i, frame);
                    }
                }
                Err(e) => {
                    warn!("[{}] G协议解码失败: {:?}", ctx.topic, e);
                }
            }

            ctx.set_response(vec![]);
        });
        self.rpc_core.register(KEY_G, handler).await?;

        let commands = self.rpc_core.get_registered_commands().await;
        info!("已注册 {} 个命令处理器: {:?}", commands.len(), commands);

        Ok(())
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.serial_port.is_some() {
            self.run_serial_mode().await
        } else {
            self.run_stdio_mode().await
        }
    }

    async fn run_serial_mode(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("启动串口数据接收循环...");

        let mut buffer = [0u8; 4096];

        loop {
            if let Some(ref mut port) = self.serial_port {
                match port.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        info!("从串口读取到 {} 字节数据", bytes_read);

                        let results = self.rpc_core.process(&buffer[..bytes_read]).await;

                        for result in results {
                            match result {
                                Ok(parse_result) => {
                                    info!("解析帧成功: key={}, data_len={}",
                                        parse_result.key, parse_result.param.len());
                                }
                                Err(e) => {
                                    error!("解析帧失败: {:?}", e);
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => {
                        error!("串口读取错误: {}", e);
                        return Err(e.into());
                    }
                }
            }
        }
    }

    async fn run_stdio_mode(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("启动标准输入输出模式...");
        info!("请输入十六进制数据（如: 55 AA 01 00...），按回车发送");
        info!("输入 'quit' 退出程序");

        use std::io::{self, BufRead, Write};

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("> ");
            stdout.flush()?;

            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            let line = line.trim();

            if line == "quit" {
                info!("退出程序");
                break;
            }

            let bytes: Vec<u8> = line.split_whitespace()
                .filter_map(|s| u8::from_str_radix(s, 16).ok())
                .collect();

            if bytes.is_empty() {
                continue;
            }

            info!("接收到 {} 字节数据: {:02X?}", bytes.len(), bytes);

            let results = self.rpc_core.process(&bytes).await;

            for result in results {
                match result {
                    Ok(parse_result) => {
                        info!("解析帧成功: key={}, is_fin={}, data_len={}",
                            parse_result.key, parse_result.is_fin, parse_result.param.len());
                    }
                    Err(e) => {
                        error!("解析帧失败: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

fn print_frame_info(index: usize, frame: &GhFuncFrame) {
    info!("  帧[{}]: cnt={}, ts={}, id={:?}, ch_num={}",
        index, frame.frame_cnt, frame.timestamp, frame.id, frame.ch_num);

    for (i, data) in frame.data.iter().enumerate() {
        info!("    通道[{}]: ipd_pa={}, rawdata={}, flag={:?}",
            i, data.ipd_pa, data.rawdata, data.flag);
    }
}

fn list_available_ports() {
    info!("扫描可用串口...");
    match serialport::available_ports() {
        Ok(ports) => {
            if ports.is_empty() {
                warn!("未找到任何串口设备");
            } else {
                info!("找到 {} 个串口:", ports.len());
                for port in ports {
                    let type_info = match port.port_type {
                        SerialPortType::UsbPort(info) => {
                            format!("USB (VID:{:04x}, PID:{:04x})",
                                info.vid, info.pid)
                        }
                        SerialPortType::BluetoothPort => "Bluetooth".to_string(),
                        SerialPortType::PciPort => "PCI".to_string(),
                        SerialPortType::Unknown => "Unknown".to_string(),
                    };
                    info!("  - {} [{}]", port.port_name, type_info);
                }
            }
        }
        Err(e) => {
            warn!("扫描串口失败: {}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .filter_module("rpc", LevelFilter::Debug)
        .filter_module("gh_rpc", LevelFilter::Debug)
        .init();

    info!("========================================");
    info!("GH Protocol Server Test Program");
    info!("========================================");

    list_available_ports();

    let mut server = ServerState::new();

    server.init_serial_port().await?;

    server.register_handlers().await?;

    let send_fn = {
        let serial_port = server.serial_port.as_ref().map(|p| {
            let port = p.try_clone().expect("Failed to clone serial port");
            Arc::new(Mutex::new(Some(port)))
        });

        Arc::new(move |data: &[u8]| -> Result<(), rpc::RpcError> {
            if let Some(ref port_mutex) = serial_port {
                if let Some(ref mut port) = *port_mutex.try_lock().map_err(|_| rpc::RpcError::SendFail)? {
                    port.write_all(data).map_err(|_| rpc::RpcError::SendFail)?;
                    port.flush().map_err(|_| rpc::RpcError::SendFail)?;
                    info!("发送 {} 字节响应数据", data.len());
                }
            }
            Ok(())
        })
    };

    server.rpc_core.set_send_function(send_fn).await;

    info!("服务器启动完成，开始接收数据...");
    info!("----------------------------------------");

    server.run().await
}
