//! 命令层：请求构造与响应解析（payload 布局见文档第 4 章）

use crate::command;
use crate::error::FrameError;
use crate::frame::Frame;

#[derive(Debug, Clone)]
pub struct Request {
    pub cmd: u8,
    pub payload: Vec<u8>,
}

impl Request {
    pub fn new(cmd: u8, payload: Vec<u8>) -> Self {
        Self { cmd, payload }
    }

    pub fn frame(&self) -> Frame {
        Frame::new(self.cmd, self.payload.clone())
    }

    // ---- 0x19 / 0xA0 / 0x25 ----
    pub fn get_version(ver_type: u8) -> Self {
        Self::new(command::GET_VER, vec![ver_type])
    }
    pub fn get_max_len() -> Self {
        // 设备外层凑帧器拒绝 LEN==0（见 gh_demo_protocol.c 的 WAIT_LEN 分支），
        // 因此无参命令统一补一个占位字节。
        Self::new(command::GET_MAX_LEN, vec![0])
    }
    pub fn get_opened_func() -> Self {
        Self::new(command::GET_OPENED_FUNC, vec![0])
    }

    // ---- 0x10 / 0x0C / 0x1C ----
    pub fn work_mode(mode: u8, func_mask: u32) -> Self {
        let mut p = vec![mode];
        p.extend_from_slice(&func_mask.to_le_bytes());
        Self::new(command::WORK_MODE, p)
    }
    pub fn start_ctrl(start: bool, mode: u8, slot_en: u8, func_mask: u32) -> Self {
        // 设备语义：0 = 启动，1 = 停止（UPROTOCOL_START_SAMPLING = 0）
        let mut p = vec![if start { 0 } else { 1 }, mode, slot_en];
        p.extend_from_slice(&func_mask.to_le_bytes());
        Self::new(command::START_CTRL, p)
    }
    pub fn switch_slot_func(slot: u8, extra: u8, func_mask: u32, on: bool) -> Self {
        let mut p = vec![slot, extra];
        p.extend_from_slice(&func_mask.to_le_bytes());
        p.push(if on { 0 } else { 1 }); // 0 = 开，1 = 关
        Self::new(command::SWITCH_SLOT_FUNC, p)
    }

    // ---- 0x03 ----
    pub fn reg_read(addr: u16, count: u8) -> Self {
        let mut p = vec![0u8, count];
        p.extend_from_slice(&addr.to_be_bytes());
        Self::new(command::REG_RW, p)
    }
    pub fn reg_write(addr: u16, values: &[u16]) -> Self {
        let mut p = vec![1u8, values.len() as u8];
        p.extend_from_slice(&addr.to_be_bytes());
        for v in values {
            p.extend_from_slice(&v.to_be_bytes());
        }
        Self::new(command::REG_RW, p)
    }

    // ---- 0xA1 ----
    pub fn load_reg_list(regs: &[(u16, u16)]) -> Self {
        let mut p = Vec::with_capacity(regs.len() * 4);
        for (a, v) in regs {
            p.extend_from_slice(&a.to_le_bytes());
            p.extend_from_slice(&v.to_le_bytes());
        }
        Self::new(command::LOAD_REG_LIST, p)
    }

    // ---- 0x17 / 0xA2 / 0x12 / 0x13 / 0x1A / 0x05 ----
    pub fn chip_ctrl(ctrl: u8) -> Self {
        Self::new(command::CHIP_CTRL, vec![ctrl])
    }
    pub fn debug_status(kind: u8) -> Self {
        Self::new(command::DEBUG_STATUS, vec![kind])
    }
    pub fn fifo_thr(thr: u16) -> Self {
        Self::new(
            command::FIFO_THR_CONFIG,
            vec![(thr >> 8) as u8, thr as u8],
        )
    }
    pub fn report_event_set(mask: u16) -> Self {
        Self::new(
            command::REPORT_EVENT_SET,
            vec![(mask >> 8) as u8, mask as u8],
        )
    }
    pub fn conn_status() -> Self {
        Self::new(command::GET_CONN_STATUS, vec![0])
    }
    pub fn package_test(data: &[u8]) -> Self {
        let mut p = vec![data.len() as u8];
        p.extend_from_slice(data);
        Self::new(command::PACKAGE_TEST, p)
    }
}

// ---------------------------------------------------------------------------
// 响应解析
// ---------------------------------------------------------------------------

fn need(payload: &[u8], n: usize, cmd: u8) -> Result<(), FrameError> {
    if payload.len() < n {
        return Err(FrameError::UnexpectedLength {
            cmd,
            len: payload.len(),
        });
    }
    Ok(())
}

/// 单字节状态：0 成功 / 1 失败 / 0xFF 空实现
pub fn parse_status(payload: &[u8]) -> Result<u8, FrameError> {
    need(payload, 1, 0)?;
    Ok(payload[0])
}

/// 0x19 响应：[0]=type [1]=len [2..]=字符串
pub fn parse_version(payload: &[u8]) -> Result<(u8, String), FrameError> {
    need(payload, 2, command::GET_VER)?;
    let ty = payload[0];
    let n = payload[1] as usize;
    let end = (2 + n).min(payload.len());
    Ok((ty, String::from_utf8_lossy(&payload[2..end]).to_string()))
}

/// 0xA0 响应：[0]=max len
pub fn parse_max_len(payload: &[u8]) -> Result<u8, FrameError> {
    need(payload, 1, command::GET_MAX_LEN)?;
    Ok(payload[0])
}

/// 0x25 响应：[0]=max len [1..4]=func mask（大端）
pub fn parse_opened_func(payload: &[u8]) -> Result<(u8, u32), FrameError> {
    need(payload, 5, command::GET_OPENED_FUNC)?;
    Ok((
        payload[0],
        u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]),
    ))
}

/// 0x03 读响应：[0..3] 头 + cnt 个 16bit（大端）
pub fn parse_reg_values(payload: &[u8]) -> Result<Vec<u16>, FrameError> {
    need(payload, 4, command::REG_RW)?;
    let cnt = payload[1] as usize;
    let end = 4 + cnt * 2;
    need(payload, end, command::REG_RW)?;
    Ok(payload[4..end]
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect())
}

/// 0xA2 响应：[0]=type [1]=bytes [2..5]=data
pub fn parse_debug(payload: &[u8]) -> Result<(u8, u8, [u8; 4]), FrameError> {
    need(payload, 6, command::DEBUG_STATUS)?;
    Ok((
        payload[0],
        payload[1],
        [payload[2], payload[3], payload[4], payload[5]],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_mode_payload() {
        let r = Request::work_mode(0, 0x0000_0006);
        assert_eq!(r.payload, vec![0x00, 0x06, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn start_ctrl_payload() {
        let r = Request::start_ctrl(true, 0, 0, 0x2);
        assert_eq!(r.payload, vec![0, 0, 0, 0x02, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn reg_write_payload() {
        let r = Request::reg_write(0x0000, &[0x1234, 0xABCD]);
        assert_eq!(r.payload, vec![1, 2, 0x00, 0x00, 0x12, 0x34, 0xAB, 0xCD]);
    }

    #[test]
    fn reg_read_parse() {
        let p = [0u8, 2, 0, 0, 0x12, 0x34, 0xAB, 0xCD];
        assert_eq!(parse_reg_values(&p).unwrap(), vec![0x1234, 0xABCD]);
    }

    #[test]
    fn version_parse() {
        let mut p = vec![0x01, 5];
        p.extend_from_slice(b"1.0.0");
        assert_eq!(parse_version(&p).unwrap(), (1, "1.0.0".to_string()));
    }
}
