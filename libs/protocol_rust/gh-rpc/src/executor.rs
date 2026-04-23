//! GH-RPC Command Executor
//!
//! 命令执行器，封装RpcCore，提供高级命令调用接口

use std::sync::Arc;
use tokio::sync::Mutex;

use rpc::{RpcCore, RpcConfig, RpcError, SendFunction, LogCallback, LogLevel, NullLogger};

use crate::commands::*;
use crate::frame_decoder::FrameDecoder;
use crate::types::{DecodeError, GhFuncFrame};

pub type FrameCallback = Arc<dyn Fn(&GhFuncFrame) + Send + Sync>;

pub struct CommandExecutor {
    core: RpcCore,
    frame_decoder: Arc<Mutex<FrameDecoder>>,
    frame_callback: Option<FrameCallback>,
    logger: Arc<dyn LogCallback>,
}

impl CommandExecutor {
    pub fn new(config: RpcConfig) -> Self {
        Self {
            core: RpcCore::new(config),
            frame_decoder: Arc::new(Mutex::new(FrameDecoder::new())),
            frame_callback: None,
            logger: Arc::new(NullLogger),
        }
    }

    pub fn from_core(core: RpcCore) -> Self {
        Self {
            core,
            frame_decoder: Arc::new(Mutex::new(FrameDecoder::new())),
            frame_callback: None,
            logger: Arc::new(NullLogger),
        }
    }

    pub fn with_logger(mut self, logger: Arc<dyn LogCallback>) -> Self {
        self.core = self.core.with_logger(logger.clone());
        self.frame_decoder = Arc::new(Mutex::new(FrameDecoder::new().with_logger(logger.clone())));
        self.logger = logger;
        self
    }

    pub fn get_core(&self) -> &RpcCore {
        &self.core
    }

    pub async fn set_send_function(&self, func: SendFunction) {
        self.core.set_send_function(func).await;
    }

    pub fn register_frame_callback(&mut self, callback: FrameCallback) {
        self.frame_callback = Some(callback);
    }

    pub async fn handle_frame_data(&self, data: &[u8]) -> Result<Vec<GhFuncFrame>, DecodeError> {
        let decoder = self.frame_decoder.lock().await;
        let frames = decoder.decode_frames(data)?;

        if let Some(ref callback) = self.frame_callback {
            for frame in &frames {
                callback(frame);
            }
        }

        Ok(frames)
    }

    pub async fn register_g_handler(&self) -> Result<(), RpcError> {
        let frame_decoder = self.frame_decoder.clone();
        let frame_callback = self.frame_callback.clone();
        let logger = self.logger.clone();
        
        let handler = Arc::new(move |data: &[u8], _size: usize, _ctx: &mut rpc::InvokeContext| {
            let frame_decoder = frame_decoder.clone();
            let frame_callback = frame_callback.clone();
            let data = data.to_vec();
            let logger = logger.clone();
            
            logger.log(LogLevel::Info, "G协议", &format!("收到数据 {} 字节: {:02X?}", data.len(), data));
            
            let unpacked = rpc::unpack_u8_array(&data);
            logger.log(LogLevel::Debug, "G协议", &format!("解包后 {} 字节: {:02X?}", unpacked.len(), &unpacked[..std::cmp::min(20, unpacked.len())]));
            
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(rt) = rt {
                rt.spawn(async move {
                    let decoder = frame_decoder.lock().await;
                    match decoder.decode_frames(&unpacked) {
                        Ok(frames) => {
                            logger.log(LogLevel::Debug, "G协议", &format!("解码成功, 共 {} 帧", frames.len()));
                            if let Some(ref callback) = frame_callback {
                                for frame in &frames {
                                    callback(frame);
                                }
                            }
                        }
                        Err(e) => {
                            logger.log(LogLevel::Error, "G协议", &format!("解码失败: {:?}", e));
                        }
                    }
                });
            }
        });
        
        self.core.register(KEY_G, handler).await
    }

    pub async fn process(&self, data: &[u8]) -> Vec<Result<rpc::ParseResult, RpcError>> {
        self.core.process(data).await
    }

    pub async fn call(&self, key: &str, format: &str, params: &[u8]) -> Result<Vec<u8>, RpcError> {
        self.core.call(key, format, params).await
    }

    pub async fn send(&self, key: &str, format: &str, params: &[u8]) -> Result<(), RpcError> {
        self.core.send(key, format, params).await
    }

    pub async fn publish(&self, key: &str, format: &str, params: &[u8]) -> Result<(), RpcError> {
        self.core.publish(key, format, params).await
    }

    pub async fn sall(&self, key: &str, format: &str, params: &[u8]) -> Result<Vec<u8>, RpcError> {
        self.core.sall(key, format, params).await
    }

    pub async fn register(&self, key: &str, handler: rpc::RpcHandler) -> Result<(), RpcError> {
        self.core.register(key, handler).await
    }
}
