use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("payload too long: {0} bytes (max 233)")]
    PayloadTooLong(usize),
    #[error("frame too short ({0} bytes)")]
    Short(usize),
    #[error("crc mismatch: got {got:#04x}, expect {expect:#04x}")]
    CrcMismatch { got: u8, expect: u8 },
    #[error("unexpected payload length {len} for cmd {cmd:#04x}")]
    UnexpectedLength { cmd: u8, len: usize },
    #[error("unsupported value {0}")]
    UnsupportedValue(u8),
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error("packet shorter than 8-byte header")]
    ShortHeader,
    #[error("truncated {what}: need {need}, got {got}")]
    Truncated {
        what: &'static str,
        need: usize,
        got: usize,
    },
    #[error("invalid zip type nibble {0}")]
    BadZipType(u8),
    #[error("bad package_type {0:#04x}")]
    BadPackageType(u8),
}
