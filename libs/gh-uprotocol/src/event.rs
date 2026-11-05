//! 事件通道 0x16：设备上报事件，主机回 ACK

use crate::command;
use crate::frame::Frame;

/// 解析事件 payload：`[event_hi, event_lo, report_id]`
pub fn parse(payload: &[u8]) -> Option<(u16, u8)> {
    if payload.len() < 3 {
        return None;
    }
    let ev = ((payload[0] as u16) << 8) | payload[1] as u16;
    Some((ev, payload[2]))
}

/// 构造 ACK 帧（CMD 0x16，payload 为 1 字节 report_id）
pub fn ack(id: u8) -> Frame {
    Frame::new(command::CHIP_EVENT_REPORT, vec![id])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_bytes() {
        let f = ack(2);
        assert_eq!(f.encode().unwrap(), vec![0xAA, 0x11, 0x16, 0x01, 0x02, 0xA2]);
    }

    #[test]
    fn parse_event() {
        assert_eq!(parse(&[0x00, 0x40, 0x07]), Some((0x40, 7)));
        assert_eq!(parse(&[0x01]), None);
    }
}
