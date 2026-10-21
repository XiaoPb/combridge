//! 专用快速 DFU 服务（`A6ED0701`）驱动。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::super::engine::events::{DfuReport, ProgressSink, Stage};
use super::super::engine::plan::{Route, TargetAddress, TransferOptions};
use super::super::engine::EngineConfig;
use super::super::error::{DfuError, Result};
use super::super::firmware::{
    ensure_no_overlap, ensure_status, sum_checksum, DfuImage, ImgInfo,
};
use super::super::frame::fast::{erase_stage, opcode, FastFrame};
use super::super::link::FastLink;
use super::super::transport::Transport;

/// 驱动独立的快速 DFU 服务。
pub struct FastServiceDriver {
    link: FastLink,
    config: EngineConfig,
    progress: ProgressSink,
    cancel: Arc<AtomicBool>,
}

impl FastServiceDriver {
    /// 订阅快速命令通道。
    pub async fn new(
        transport: Arc<dyn Transport>,
        config: EngineConfig,
        progress: ProgressSink,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self> {
        let link = FastLink::new(transport, config.fast_command_timeout).await?;
        Ok(Self {
            link,
            config,
            progress,
            cancel,
        })
    }

    /// 通过快速服务更新固件镜像。
    pub async fn update_firmware(
        &mut self,
        image: &DfuImage,
        options: &TransferOptions,
    ) -> Result<DfuReport> {
        self.ensure_active()?;
        let version = self.get_version().await?;
        let buffer_size = self.buffer_size(version).await?;

        let write_address = match options.target {
            TargetAddress::LoadAddress => image.boot_info().load_addr,
            TargetAddress::Explicit(address) => address,
            TargetAddress::Recommended => {
                return Err(DfuError::Unsupported(
                    "the fast service has no recommended address".to_string(),
                ))
            }
        };
        let copy_mode = write_address != image.boot_info().load_addr;
        if copy_mode {
            ensure_no_overlap(
                "COPY",
                write_address,
                image.len() as u32,
                "APP_NEW",
                image.boot_info().load_addr,
                image.len() as u32,
            )?;
        }

        self.progress.stage(Stage::Prepare);
        self.set_flash_type(false).await?;
        self.erase(write_address, image.len() as u32).await?;
        self.program(version, &image.data, buffer_size).await?;

        tokio::time::sleep(self.config.fast_settle).await;

        self.progress.stage(Stage::Verify);
        self.verify(image.checksum).await?;

        self.progress.stage(Stage::Reboot);
        self.reboot(image, copy_mode, write_address).await?;

        Ok(DfuReport {
            route: Route::FastService,
            protocol_version: version,
            bytes_transferred: image.len() as u64,
            checksum: image.checksum,
        })
    }

    /// 通过快速服务更新资源镜像。
    pub async fn update_resource(
        &mut self,
        address: u32,
        data: &[u8],
        ext_flash: bool,
    ) -> Result<DfuReport> {
        self.ensure_active()?;
        if data.is_empty() {
            return Err(DfuError::Firmware("resource image is empty".to_string()));
        }
        let version = self.get_version().await?;
        let buffer_size = self.buffer_size(version).await?;
        let checksum = sum_checksum(data);

        self.progress.stage(Stage::Prepare);
        self.set_flash_type(ext_flash).await?;
        self.erase(address, data.len() as u32).await?;
        self.program(version, data, buffer_size).await?;

        tokio::time::sleep(self.config.fast_settle).await;

        self.progress.stage(Stage::Verify);
        self.verify(checksum).await?;

        Ok(DfuReport {
            route: Route::FastService,
            protocol_version: version,
            bytes_transferred: data.len() as u64,
            checksum,
        })
    }

    async fn get_version(&mut self) -> Result<u8> {
        self.progress.message("read fast DFU version");
        let ack = self.cmd(opcode::GET_VERSION, &[]).await?;
        let version = ack.params.first().copied().ok_or_else(|| {
            DfuError::Protocol("GET_VERSION acknowledgement has no payload".to_string())
        })?;
        tracing::info!(version, "fast DFU version");
        Ok(version)
    }

    async fn buffer_size(&mut self, version: u8) -> Result<u32> {
        if version < 3 {
            let ack = self.cmd(opcode::GET_BUFFER_SIZE, &[]).await?;
            if ack.params.len() < 4 {
                return Err(DfuError::Protocol(format!(
                    "GET_BUFFER_SIZE returned {} bytes",
                    ack.params.len()
                )));
            }
            let size =
                u32::from_le_bytes([ack.params[0], ack.params[1], ack.params[2], ack.params[3]]);
            if size == 0 {
                return Err(DfuError::Protocol(
                    "device reported a zero buffer".to_string(),
                ));
            }
            tracing::info!(size, "fast DFU buffer size");
            Ok(size)
        } else {
            Ok(4096)
        }
    }

    async fn set_flash_type(&mut self, ext_flash: bool) -> Result<()> {
        tracing::info!(ext_flash, "select fast DFU flash type");
        self.cmd(opcode::SELECT_FLASH_TYPE, &[u8::from(ext_flash)])
            .await?;
        Ok(())
    }

    async fn erase(&mut self, address: u32, size: u32) -> Result<()> {
        self.ensure_active()?;
        self.progress.stage(Stage::Erase);
        let mut params = Vec::with_capacity(8);
        params.extend_from_slice(&address.to_le_bytes());
        params.extend_from_slice(&size.to_le_bytes());

        self.link
            .send(&FastFrame::new(opcode::ERASE_FLASH, params))
            .await?;

        let mut total = 0_u16;
        loop {
            self.ensure_active()?;
            let ack = self
                .link
                .next_ack(self.config.fast_command_timeout, "erase acknowledgement")
                .await?;
            if ack.opcode != opcode::ERASE_FLASH || ack.params.len() < 2 {
                return Err(DfuError::Protocol(format!(
                    "unexpected erase acknowledgement 0x{:02X} with {} params",
                    ack.opcode,
                    ack.params.len()
                )));
            }
            let stage = ack.params[0];
            let value = if ack.params.len() >= 3 {
                u16::from_le_bytes([ack.params[1], ack.params[2]])
            } else {
                0
            };

            match stage {
                erase_stage::UNALIGNED => {
                    return Err(DfuError::Device(
                        "the write address is not 4K aligned".to_string(),
                    ))
                }
                erase_stage::TOTAL => {
                    total = value;
                    tracing::info!(total, "erase started");
                }
                erase_stage::ERASED => {
                    if total == 0 {
                        total = value;
                    }
                    self.progress.erase(value, total);
                }
                erase_stage::DONE => {
                    tracing::info!("erase completed");
                    return Ok(());
                }
                erase_stage::OVERLAP => {
                    return Err(DfuError::Overlap(
                        "the erase region overlaps the running firmware".to_string(),
                    ))
                }
                erase_stage::FAILED => {
                    return Err(DfuError::Device("flash erase failed".to_string()))
                }
                erase_stage::NO_EXT_FLASH => {
                    return Err(DfuError::Device(
                        "the device has no external flash".to_string(),
                    ))
                }
                other => {
                    return Err(DfuError::Protocol(format!(
                        "unknown erase stage 0x{other:02X}"
                    )))
                }
            }
        }
    }

    async fn program(&mut self, version: u8, data: &[u8], buffer_size: u32) -> Result<()> {
        self.ensure_active()?;
        self.progress.stage(Stage::Program);
        let total = data.len() as u64;
        let mut written = 0_u64;

        if version >= 3 {
            let chunk_size = self.link.transport().capabilities().write_chunk_size();
            for chunk in data.chunks(chunk_size) {
                self.ensure_active()?;
                self.link.write_data(chunk).await?;
                written += chunk.len() as u64;
                self.progress.transfer(written, total);
            }
            self.cmd(opcode::FLUSH_FLASH, &[]).await?;
            return Ok(());
        }

        let block_size = usize::try_from(buffer_size)
            .map_err(|_| DfuError::Protocol("buffer size does not fit usize".to_string()))?;
        if block_size == 0 {
            return Err(DfuError::Protocol("device buffer is zero".to_string()));
        }

        for block in data.chunks(block_size) {
            self.ensure_active()?;
            self.link.write_data(block).await?;
            let ack = self
                .link
                .next_ack(self.config.fast_command_timeout, "NEXT_BUFFER")
                .await?;
            match ack.opcode {
                opcode::NEXT_BUFFER => {}
                opcode::FLOW_CTRL_PAUSE => {
                    return Err(DfuError::Device(
                        "device buffer overflowed (flow control pause)".to_string(),
                    ))
                }
                opcode::FLOW_CTRL_RESUME => {
                    return Err(DfuError::Device(
                        "unexpected flow control resume".to_string(),
                    ))
                }
                other => {
                    return Err(DfuError::Protocol(format!(
                        "unexpected data acknowledgement 0x{other:02X}"
                    )))
                }
            }
            written += block.len() as u64;
            self.progress.transfer(written, total);
        }

        self.cmd(opcode::FLUSH_FLASH, &[]).await?;
        Ok(())
    }

    async fn verify(&mut self, checksum: u32) -> Result<()> {
        self.ensure_active()?;
        let ack = self
            .cmd(opcode::VERIFY_CHECKSUM, &checksum.to_le_bytes())
            .await?;
        if ack.params.len() < 4 {
            return Err(DfuError::Protocol(
                "VERIFY_CHECKSUM returned no checksum".to_string(),
            ));
        }
        let device_checksum =
            u32::from_le_bytes([ack.params[0], ack.params[1], ack.params[2], ack.params[3]]);
        if device_checksum != checksum {
            return Err(DfuError::Protocol(format!(
                "flash checksum 0x{device_checksum:08X} does not match 0x{checksum:08X}"
            )));
        }
        tracing::info!("fast DFU checksum verified");
        Ok(())
    }

    async fn reboot(&self, image: &DfuImage, copy_mode: bool, copy_address: u32) -> Result<()> {
        let info: ImgInfo = image.img_info.clone();
        let frame = if copy_mode {
            let mut params = Vec::with_capacity(48);
            params.extend_from_slice(&info.to_bytes());
            params.extend_from_slice(&copy_address.to_le_bytes());
            params.extend_from_slice(&(image.len() as u32).to_le_bytes());
            FastFrame::new(opcode::START_COPY, params)
        } else {
            FastFrame::new(opcode::WRITE_BOOT, info.to_bytes())
        };
        tracing::info!(copy_mode, "requesting firmware boot");
        // 写入 boot 信息后设备立即复位，链路断开属于预期行为。
        self.link.transport().mark_expected_disconnect();
        self.link.send(&frame).await
    }

    async fn cmd(&mut self, opcode: u8, params: &[u8]) -> Result<FastFrame> {
        self.ensure_active()?;
        self.link
            .request(FastFrame::new(opcode, params.to_vec()))
            .await
    }

    fn ensure_active(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(DfuError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// 为测试构造应答字节重新导出。
pub fn ack_bytes(opcode: u8, params: &[u8]) -> Vec<u8> {
    FastFrame::new(opcode, params.to_vec()).encode()
}

/// 驱动校验状态字节的辅助函数。
pub fn status_ok(payload: &[u8]) -> Result<()> {
    ensure_status(payload.first().copied().unwrap_or(0))
}
