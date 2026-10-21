//! 基础 `a6ed0401` 服务的 DFU V1 / V2 驱动。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::super::engine::events::{DfuReport, ProgressSink, Stage};
use super::super::engine::plan::{Route, TargetAddress, TransferOptions};
use super::super::engine::{use_fast_mode, EngineConfig};
use super::super::error::{DfuError, Result};
use super::super::firmware::{
    check_overlap_v1, check_overlap_v2, ensure_status, sum_checksum, AppBootloaderExtraInfo,
    ChipInfo, DfuImage, ImgInfo, ImgInfoList, StartupBootInfo, BOOT_INFO_LEN, IMG_INFO_LEN,
};
use super::super::frame::basic::{opcode, CTRL_POINT_DFU_PATTERN};
use super::super::link::BasicLink;
use super::super::transport::{Channel, ReconnectHint, Transport};

const MAX_SEGMENT_SIZE: usize = 1024;
const IMG_LIST_SIZE: usize = IMG_INFO_LEN * ImgInfoList::SLOTS;

/// 驱动基础服务的 DFU 会话。
pub struct NormalDriver {
    link: BasicLink,
    config: EngineConfig,
    progress: ProgressSink,
    cancel: Arc<AtomicBool>,
    is_v2: bool,
}

impl NormalDriver {
    /// 订阅通知并准备驱动。
    pub async fn new(
        transport: Arc<dyn Transport>,
        config: EngineConfig,
        progress: ProgressSink,
        cancel: Arc<AtomicBool>,
        is_v2: bool,
    ) -> Result<Self> {
        let link = BasicLink::new(
            transport,
            config.command_timeout,
            config.verify_frame_checksum,
        )
        .await?;
        Ok(Self {
            link,
            config,
            progress,
            cancel,
            is_v2,
        })
    }

    /// 更新固件镜像。
    pub async fn update_firmware(
        &mut self,
        image: &DfuImage,
        options: &TransferOptions,
    ) -> Result<DfuReport> {
        self.ensure_active()?;
        let data_len = image.len() as u32;

        let (write_address, fast_mode) = if self.is_v2 {
            self.prepare_v2_firmware(image, options).await?
        } else {
            self.prepare_v1_firmware(image, options).await?
        };

        self.progress.stage(Stage::Prepare);
        self.program_start(true, fast_mode, Some(image), data_len, write_address, false)
            .await?;

        self.progress.stage(Stage::Program);
        if fast_mode {
            self.program_flash_fast(&image.data).await?;
        } else {
            self.program_flash_normal(&image.data, write_address, false)
                .await?;
        }

        self.progress.stage(Stage::Verify);
        let reset_type = if options.run_immediately { 0x01 } else { 0x00 };
        self.program_end(fast_mode, image.checksum, reset_type)
            .await?;

        self.progress.stage(Stage::Reboot);
        Ok(DfuReport {
            route: Route::Normal,
            protocol_version: if self.is_v2 { 2 } else { 1 },
            bytes_transferred: image.len() as u64,
            checksum: image.checksum,
        })
    }

    /// 更新原始资源镜像。
    pub async fn update_resource(
        &mut self,
        data: &[u8],
        address: u32,
        ext_flash: bool,
        options: &TransferOptions,
    ) -> Result<DfuReport> {
        self.ensure_active()?;
        if data.is_empty() {
            return Err(DfuError::Firmware("resource image is empty".to_string()));
        }
        let data_len = data.len() as u32;
        let checksum = sum_checksum(data);

        let fast_mode = if self.is_v2 {
            self.prepare_v2_resource(data_len, address, ext_flash)
                .await?;
            use_fast_mode(options.mode, true)
        } else {
            self.prepare_v1_resource(data_len, address, ext_flash, options)
                .await?;
            false
        };

        self.progress.stage(Stage::Prepare);
        self.program_start(false, fast_mode, None, data_len, address, ext_flash)
            .await?;

        self.progress.stage(Stage::Program);
        if fast_mode {
            self.program_flash_fast(data).await?;
        } else {
            self.program_flash_normal(data, address, ext_flash).await?;
        }

        self.progress.stage(Stage::Verify);
        let reset_type = if ext_flash { 0x12 } else { 0x02 };
        self.program_end(fast_mode, checksum, reset_type).await?;

        Ok(DfuReport {
            route: Route::Normal,
            protocol_version: if self.is_v2 { 2 } else { 1 },
            bytes_transferred: data.len() as u64,
            checksum,
        })
    }

    // ------------------------------------------------------------------
    // 准备阶段
    // ------------------------------------------------------------------

    async fn prepare_v2_firmware(
        &mut self,
        image: &DfuImage,
        options: &TransferOptions,
    ) -> Result<(u32, bool)> {
        self.enable_dfu_schedule().await?;

        let (_chip, sca) = self.load_chip_info().await?;
        let startup = self.load_startup_boot_info(sca).await?;
        let extra = self.load_extra_info().await?;

        let write_address = match options.target {
            TargetAddress::LoadAddress => image.boot_info().load_addr,
            TargetAddress::Explicit(address) => address,
            TargetAddress::Recommended => extra.recommend_save_address,
        };
        let is_double_bank = image.boot_info().load_addr != write_address;

        if image.encrypted != startup.encrypted {
            return Err(DfuError::Device(format!(
                "encryption mismatch: firmware = {}, chip = {}",
                image.encrypted, startup.encrypted
            )));
        }

        self.progress.stage(Stage::CheckOverlap);
        check_overlap_v2(
            true,
            false,
            image.len() as u32,
            write_address,
            sca,
            &startup.boot_info,
            Some(&extra.app_fw_img_info.boot_info),
            Some(image.boot_info()),
        )?;

        self.progress.stage(Stage::SetDfuMode);
        if is_double_bank || extra.position == AppBootloaderExtraInfo::CURRENT_FW_IS_APP {
            self.set_dfu_mode(is_double_bank).await?;
        }

        if !is_double_bank && extra.position == AppBootloaderExtraInfo::CURRENT_FW_IS_APP {
            self.progress.stage(Stage::Reconnect);
            self.reconnect_after_jump().await?;
        }

        Ok((write_address, use_fast_mode(options.mode, true)))
    }

    async fn prepare_v1_firmware(
        &mut self,
        image: &DfuImage,
        options: &TransferOptions,
    ) -> Result<(u32, bool)> {
        if let Some(command) = &options.control_command {
            self.link.write_control(command).await?;
        }

        let (_chip, sca) = self.load_chip_info().await?;
        let startup = self.load_startup_boot_info(sca).await?;

        let write_address = match options.target {
            TargetAddress::LoadAddress => image.boot_info().load_addr,
            TargetAddress::Explicit(address) => address,
            TargetAddress::Recommended => {
                return Err(DfuError::Unsupported(
                    "the recommended address is only available with DFU V2".to_string(),
                ))
            }
        };

        if image.encrypted != startup.encrypted {
            return Err(DfuError::Device(format!(
                "encryption mismatch: firmware = {}, chip = {}",
                image.encrypted, startup.encrypted
            )));
        }

        self.progress.stage(Stage::CheckOverlap);
        check_overlap_v1(
            true,
            false,
            image.len() as u32,
            write_address,
            sca,
            &startup.boot_info,
            Some(image.boot_info()),
        )?;

        self.progress.stage(Stage::Prepare);
        let list = self.load_img_list(sca).await?;
        self.tidy_img_list(
            image.boot_info().load_addr,
            image.len() as u32,
            &startup.boot_info,
            &list.images,
            sca,
        )
        .await?;

        Ok((write_address, false))
    }

    async fn prepare_v2_resource(
        &mut self,
        data_len: u32,
        address: u32,
        ext_flash: bool,
    ) -> Result<()> {
        self.enable_dfu_schedule().await?;
        let (_chip, sca) = self.load_chip_info().await?;
        let startup = self.load_startup_boot_info(sca).await?;
        let extra = self.load_extra_info().await?;

        self.progress.stage(Stage::CheckOverlap);
        check_overlap_v2(
            false,
            ext_flash,
            data_len,
            address,
            sca,
            &startup.boot_info,
            Some(&extra.app_fw_img_info.boot_info),
            None,
        )
    }

    async fn prepare_v1_resource(
        &mut self,
        data_len: u32,
        address: u32,
        ext_flash: bool,
        options: &TransferOptions,
    ) -> Result<()> {
        if let Some(command) = &options.control_command {
            self.link.write_control(command).await?;
        }
        let (_chip, sca) = self.load_chip_info().await?;
        let startup = self.load_startup_boot_info(sca).await?;

        self.progress.stage(Stage::CheckOverlap);
        check_overlap_v1(
            false,
            ext_flash,
            data_len,
            address,
            sca,
            &startup.boot_info,
            None,
        )?;

        self.progress.stage(Stage::Prepare);
        let list = self.load_img_list(sca).await?;
        self.tidy_img_list(address, data_len, &startup.boot_info, &list.images, sca)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 基础命令
    // ------------------------------------------------------------------

    async fn enable_dfu_schedule(&mut self) -> Result<()> {
        self.link.write_control(&CTRL_POINT_DFU_PATTERN).await
    }

    async fn load_chip_info(&mut self) -> Result<(ChipInfo, u32)> {
        self.ensure_active()?;
        self.progress.stage(Stage::LoadChipInfo);
        let payload = self
            .request(opcode::GET_INFO, &[], "GET_INFO response")
            .await?;
        if payload.len() < 1 + super::super::firmware::CHIP_INFO_LEN {
            return Err(DfuError::Protocol(format!(
                "GET_INFO response is too short: {} bytes",
                payload.len()
            )));
        }
        ensure_status(payload[0])?;
        let chip = ChipInfo::from_bytes(&payload[1..])?;
        let sca = chip.sca_address();
        tracing::info!(
            stack_svn = format_args!("0x{:08X}", chip.stack_svn),
            sca = format_args!("0x{sca:08X}"),
            dfu_version = chip.dfu_version,
            "chip info loaded"
        );
        Ok((chip, sca))
    }

    async fn load_startup_boot_info(&mut self, sca: u32) -> Result<StartupBootInfo> {
        self.ensure_active()?;
        self.progress.stage(Stage::LoadBootInfo);
        let mut params = Vec::with_capacity(7);
        params.push(0x00);
        params.extend_from_slice(&sca.to_le_bytes());
        params.extend_from_slice(&(BOOT_INFO_LEN as u16).to_le_bytes());
        let payload = self
            .request(opcode::SYSTEM_CONFIG, &params, "SYSTEM_CONFIG response")
            .await?;
        StartupBootInfo::from_response(&payload, sca)
    }

    async fn load_img_list(&mut self, sca: u32) -> Result<ImgInfoList> {
        self.ensure_active()?;
        let address = sca.wrapping_add(0x40);
        let mut params = Vec::with_capacity(7);
        params.push(0x00);
        params.extend_from_slice(&address.to_le_bytes());
        params.extend_from_slice(&(IMG_LIST_SIZE as u16).to_le_bytes());
        let payload = self
            .request(opcode::SYSTEM_CONFIG, &params, "SYSTEM_CONFIG image list")
            .await?;
        ImgInfoList::from_response(&payload, address)
    }

    async fn load_extra_info(&mut self) -> Result<AppBootloaderExtraInfo> {
        self.ensure_active()?;
        self.progress.stage(Stage::LoadExtraInfo);
        let payload = self
            .request(opcode::GET_FW_INFO, &[], "GET_FW_INFO response")
            .await?;
        AppBootloaderExtraInfo::from_response(&payload)
    }

    async fn set_dfu_mode(&mut self, double_bank: bool) -> Result<()> {
        if !self.is_v2 {
            return Err(DfuError::Unsupported(
                "SET_DFU_MODE requires DFU V2".to_string(),
            ));
        }
        let flag = if double_bank { 0x01 } else { 0x02 };
        tracing::info!(double_bank, "switching DFU mode");
        // 设备可能在此后重启，链路断开属于预期行为。
        self.link.transport().mark_expected_disconnect();
        // 设备对该命令不返回响应。
        self.link.send(opcode::SET_DFU_MODE, &[flag]).await?;
        tokio::time::sleep(self.config.set_dfu_mode_wait).await;
        Ok(())
    }

    async fn reconnect_after_jump(&mut self) -> Result<()> {
        let current = self.link.transport().address().ok_or_else(|| {
            DfuError::Unsupported(
                "transport does not expose the device address; cannot reconnect to the bootloader"
                    .to_string(),
            )
        })?;
        let next = super::super::util::bump_last_mac_byte(&current)
            .ok_or_else(|| DfuError::Unsupported(format!("'{current}' is not a MAC address")))?;
        tracing::info!(
            from = current,
            to = next,
            "reconnecting to the App Bootloader"
        );
        self.link
            .transport()
            .reconnect(ReconnectHint {
                address: Some(next),
                delay: self.config.reconnect_delay,
            })
            .await?;
        self.link.resubscribe().await?;
        tokio::time::sleep(self.config.post_reconnect_wait).await;
        Ok(())
    }

    async fn tidy_img_list(
        &mut self,
        target_address: u32,
        target_size: u32,
        startup: &super::super::firmware::BootInfo,
        images: &[ImgInfo],
        sca: u32,
    ) -> Result<()> {
        self.ensure_active()?;
        let mut clean: Vec<ImgInfo> = Vec::with_capacity(ImgInfoList::SLOTS);
        let mut changed = false;

        let startup_present = images.iter().any(|info| {
            info.boot_info.checksum == startup.checksum
                && info.boot_info.load_addr == startup.load_addr
        });
        if !startup_present {
            clean.push(ImgInfo::new(1, *startup, "fromBoot"));
            changed = true;
        }

        for info in images {
            if !info.is_valid() {
                continue;
            }
            if info.boot_info.has_overlap(target_address, target_size) {
                changed = true;
                continue;
            }
            clean.push(info.clone());
        }

        if !changed {
            tracing::info!("image info list is already consistent");
            return Ok(());
        }
        if clean.len() > ImgInfoList::SLOTS {
            return Err(DfuError::Device(
                "image info list would exceed its ten slots".to_string(),
            ));
        }

        let address = sca.wrapping_add(0x40);
        let mut params = Vec::with_capacity(7 + IMG_LIST_SIZE);
        params.push(0x01);
        params.extend_from_slice(&address.to_le_bytes());
        params.extend_from_slice(&(IMG_LIST_SIZE as u16).to_le_bytes());
        for info in &clean {
            params.extend_from_slice(&info.to_bytes());
        }
        params.resize(7 + IMG_LIST_SIZE, 0xFF);

        let payload = self
            .request(
                opcode::SYSTEM_CONFIG,
                &params,
                "SYSTEM_CONFIG image list write",
            )
            .await?;
        ensure_status(payload.first().copied().unwrap_or(0))?;
        tracing::info!(entries = clean.len(), "image info list updated");
        Ok(())
    }

    async fn program_start(
        &mut self,
        is_firmware: bool,
        fast_mode: bool,
        image: Option<&DfuImage>,
        data_len: u32,
        write_address: u32,
        ext_flash: bool,
    ) -> Result<()> {
        self.ensure_active()?;
        let type_byte = if ext_flash {
            if fast_mode {
                0x03
            } else {
                0x01
            }
        } else {
            let mut value = if fast_mode { 0x02 } else { 0x00 };
            if self.is_v2 {
                if let Some(image) = image {
                    if image.signed {
                        value |= if image.encrypted { 0x20 } else { 0x10 };
                    }
                }
            }
            value
        };

        let mut params = Vec::with_capacity(41);
        params.push(type_byte);
        if is_firmware {
            let image = image.ok_or_else(|| {
                DfuError::Protocol("firmware transfer requires an image".to_string())
            })?;
            let info = image.img_info_for_address(write_address);
            params.extend_from_slice(&info.to_bytes());
        } else {
            params.extend_from_slice(&write_address.to_le_bytes());
            params.extend_from_slice(&data_len.to_le_bytes());
        }

        tracing::info!(
            is_firmware,
            fast_mode,
            ext_flash,
            write_address = format_args!("0x{write_address:08X}"),
            "PROGRAM_START"
        );
        let payload = self
            .request(opcode::PROGRAM_START, &params, "PROGRAM_START response")
            .await?;
        let status = payload.first().copied().unwrap_or(0);
        ensure_status(status)?;

        if fast_mode {
            self.track_erase(payload).await?;
        }
        Ok(())
    }

    /// 跟踪通过重复 `PROGRAM_START` 响应上报的擦除进度。
    async fn track_erase(&mut self, initial: Vec<u8>) -> Result<()> {
        self.progress.stage(Stage::Erase);
        let mut total = sector_count(&initial);
        let mut state = initial.get(1).copied().unwrap_or(0);

        loop {
            self.ensure_active()?;
            match state {
                0x00 => {
                    return Err(DfuError::Device(
                        "the write address is not 4K aligned".to_string(),
                    ))
                }
                0x01 | 0x02 => {}
                0x03 => {
                    tracing::info!("erase completed");
                    return Ok(());
                }
                0x04 => {
                    return Err(DfuError::Overlap(
                        "the erase region overlaps the running firmware".to_string(),
                    ))
                }
                0x05 => return Err(DfuError::Device("flash erase failed".to_string())),
                0x06 => {
                    return Err(DfuError::Device(
                        "the requested erase region does not exist".to_string(),
                    ))
                }
                other => {
                    return Err(DfuError::Protocol(format!(
                        "unknown erase state 0x{other:02X}"
                    )))
                }
            }

            let payload = self
                .link
                .receive(opcode::PROGRAM_START, self.config.command_timeout)
                .await?;
            ensure_status(payload.first().copied().unwrap_or(0))?;
            let next_state = payload.get(1).copied().unwrap_or(0);
            let value = sector_count(&payload);

            match next_state {
                0x00 => {
                    return Err(DfuError::Device(
                        "the write address is not 4K aligned".to_string(),
                    ))
                }
                0x01 => {
                    if value > 0 {
                        total = value;
                    }
                    tracing::debug!(total, "erasing started");
                }
                0x02 => {
                    if total == 0 {
                        total = value;
                    }
                    self.progress.erase(value, total);
                }
                0x03 => {
                    tracing::info!("erase completed");
                    return Ok(());
                }
                0x04 => {
                    return Err(DfuError::Overlap(
                        "the erase region overlaps the running firmware".to_string(),
                    ))
                }
                0x05 => return Err(DfuError::Device("flash erase failed".to_string())),
                0x06 => {
                    return Err(DfuError::Device(
                        "the requested erase region does not exist".to_string(),
                    ))
                }
                other => {
                    return Err(DfuError::Protocol(format!(
                        "unknown erase state 0x{other:02X}"
                    )))
                }
            }

            state = next_state;
        }
    }

    async fn program_flash_normal(
        &mut self,
        data: &[u8],
        write_address: u32,
        ext_flash: bool,
    ) -> Result<()> {
        let total = data.len() as u64;
        let mut written = 0_u64;
        let type_byte = if ext_flash { 0x11 } else { 0x01 };

        for (index, segment) in data.chunks(MAX_SEGMENT_SIZE).enumerate() {
            self.ensure_active()?;
            let offset = index * MAX_SEGMENT_SIZE;
            let address = write_address.wrapping_add(offset as u32);
            let mut params = Vec::with_capacity(7 + segment.len());
            params.push(type_byte);
            params.extend_from_slice(&address.to_le_bytes());
            params.extend_from_slice(&(segment.len() as u16).to_le_bytes());
            params.extend_from_slice(segment);

            let payload = self
                .request(opcode::PROGRAM_FLASH, &params, "PROGRAM_FLASH response")
                .await?;
            ensure_status(payload.first().copied().unwrap_or(0))?;

            written += segment.len() as u64;
            self.progress.transfer(written, total);
        }
        Ok(())
    }

    async fn program_flash_fast(&mut self, data: &[u8]) -> Result<()> {
        let chunk_size = self.link.transport().capabilities().write_chunk_size();
        let total = data.len() as u64;
        let mut written = 0_u64;

        for chunk in data.chunks(chunk_size) {
            self.ensure_active()?;
            self.link.write_raw(chunk).await?;
            written += chunk.len() as u64;
            self.progress.transfer(written, total);
        }

        let payload = self
            .link
            .receive(opcode::PROGRAM_FLASH_FAST, self.config.command_timeout)
            .await?;
        ensure_status(payload.first().copied().unwrap_or(0))?;
        Ok(())
    }

    async fn program_end(&mut self, fast_mode: bool, checksum: u32, reset_type: u8) -> Result<()> {
        let mut params = Vec::with_capacity(5);
        params.push(reset_type);
        params.extend_from_slice(&checksum.to_le_bytes());
        tracing::info!(
            reset_type = format_args!("0x{reset_type:02X}"),
            checksum = format_args!("0x{checksum:08X}"),
            "PROGRAM_END"
        );
        if reset_type != 0x00 {
            // 立即复位会让链路断开，登记为预期断开。
            self.link.transport().mark_expected_disconnect();
        }
        self.link.send(opcode::PROGRAM_END, &params).await?;

        match self
            .link
            .receive(opcode::PROGRAM_END, self.config.command_timeout)
            .await
        {
            Ok(payload) => {
                ensure_status(payload.first().copied().unwrap_or(0))?;
                if fast_mode && payload.len() >= 5 {
                    let device_checksum =
                        u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
                    if device_checksum != checksum {
                        return Err(DfuError::Protocol(format!(
                            "flash checksum 0x{device_checksum:08X} does not match 0x{checksum:08X}"
                        )));
                    }
                }
                Ok(())
            }
            Err(error) if reset_type == 0x01 && is_disconnect(&error) => {
                tracing::info!("device rebooted before acknowledging PROGRAM_END");
                Ok(())
            }
            Err(error) if reset_type == 0x01 && error.is_timeout() => {
                tracing::info!("no PROGRAM_END acknowledgement after reboot request");
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    // ------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------

    async fn request(&mut self, opcode: u16, payload: &[u8], context: &str) -> Result<Vec<u8>> {
        let attempts = self.config.retry.max_retries.saturating_add(1);
        let mut last_error: Option<DfuError> = None;

        for attempt in 0..attempts {
            self.ensure_active()?;
            if attempt > 0 {
                tracing::warn!(attempt, context, "retrying DFU command");
                tokio::time::sleep(self.config.retry.retry_delay).await;
            }
            self.link.send(opcode, payload).await?;
            match self.link.receive(opcode, self.config.command_timeout).await {
                Ok(response) => return Ok(response),
                Err(error) if error.is_timeout() => last_error = Some(error),
                Err(error) => return Err(error),
            }
        }

        match last_error {
            Some(error) => Err(error),
            None => Err(DfuError::timeout(
                context.to_string(),
                self.config.command_timeout,
            )),
        }
    }

    fn ensure_active(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(DfuError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn sector_count(payload: &[u8]) -> u16 {
    if payload.len() >= 4 {
        u16::from_le_bytes([payload[2], payload[3]])
    } else {
        0
    }
}

fn is_disconnect(error: &DfuError) -> bool {
    match error {
        DfuError::Transport(message) => {
            let lowered = message.to_lowercase();
            lowered.contains("disconnect") || lowered.contains("connection")
        }
        _ => false,
    }
}

/// 基础服务使用的通道。
pub const BASIC_CHANNEL: Channel = Channel::DfuNotify;

/// 快速模式原始写入使用的超时；为测试导出。
pub const FAST_RAW_TIMEOUT: Duration = Duration::from_secs(10);
