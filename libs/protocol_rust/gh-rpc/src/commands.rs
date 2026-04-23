//! GH-RPC Commands
//!
//! GH协议命令定义，参考C代码gh_rpc_functions.h

use serde::{Deserialize, Serialize};

pub const KEY_EVENT: &str = "Event";
pub const KEY_F: &str = "F";
pub const KEY_FW: &str = "FW";
pub const KEY_F_GET_MODE: &str = "F_GetMode";
pub const KEY_F_SET_MODE: &str = "F_SetMode";
pub const KEY_G: &str = "G";
pub const KEY_GH3X_CHIP_CTRL: &str = "GH3X_ChipCtrl";
pub const KEY_GH3X_GET_VERSION: &str = "GH3X_GetVersion";
pub const KEY_GH3X_REG_BIT_FIELD_WRITE_CMD: &str = "GH3X_RegBitFieldWriteCmd";
pub const KEY_GH3X_REGS_BIT_FIELD_WRITE_CMD: &str = "GH3X_RegsBitFieldWriteCmd";
pub const KEY_GH3X_REGS_LIST_WRITE_CMD: &str = "GH3X_RegsListWriteCmd";
pub const KEY_GH3X_REGS_READ_CMD: &str = "GH3X_RegsReadCmd";
pub const KEY_GH3X_REGS_WRITE_CMD: &str = "GH3X_RegsWriteCmd";
pub const KEY_GH3X_SW_FUNCTION_CMD: &str = "GH3X_SwFunctionCmd";
pub const KEY_GH_SET_WORK_MODE_CMD: &str = "GHSetWorkModeCmd";
pub const KEY_DOWNLOAD_CONFIG: &str = "download_config";
pub const KEY_GET_CHIP_LINK_STATUS: &str = "get_chip_link_status";
pub const KEY_GH_LOW_POWER_CMD: &str = "gh_low_power_cmd";
pub const KEY_GH_TIME_SET: &str = "gh_time_set";
pub const KEY_GH_TIMESTAMP_SET: &str = "gh_timestamp_set";

pub const FMT_EVENT: &str = "<u8*>";
pub const FMT_F: &str = "<u8*><u32>";
pub const FMT_FW: &str = "<u8*>";
pub const FMT_F_GET_MODE: &str = "<u8>";
pub const FMT_F_SET_MODE: &str = "<u8>";
pub const FMT_G: &str = "<u8*>";
pub const FMT_GH3X_CHIP_CTRL: &str = "<u8>";
pub const FMT_GH3X_GET_VERSION: &str = "<u8>";
pub const FMT_GH3X_REG_BIT_FIELD_WRITE_CMD: &str = "<u16><u8><u8><u16>";
pub const FMT_GH3X_REGS_BIT_FIELD_WRITE_CMD: &str = "<u16*>";
pub const FMT_GH3X_REGS_LIST_WRITE_CMD: &str = "<u16*>";
pub const FMT_GH3X_REGS_READ_CMD: &str = "<u16><d32>";
pub const FMT_GH3X_REGS_WRITE_CMD: &str = "<u16*>";
pub const FMT_GH3X_SW_FUNCTION_CMD: &str = "<u32><u8>";
pub const FMT_GH_SET_WORK_MODE_CMD: &str = "<u8>";
pub const FMT_DOWNLOAD_CONFIG: &str = "<u8>";
pub const FMT_GET_CHIP_LINK_STATUS: &str = "<u8>";
pub const FMT_GH_LOW_POWER_CMD: &str = "<u32><u8>";
pub const FMT_GH_TIME_SET: &str = "<u32><d8>";
pub const FMT_GH_TIMESTAMP_SET: &str = "<u32>";

pub const RET_GH3X_GET_VERSION: &str = "<u8*>";
pub const RET_GH3X_REGS_READ_CMD: &str = "<u16*>";
pub const RET_FW: &str = "<u8*>";
pub const RET_GET_CHIP_LINK_STATUS: &str = "<d8*>";
pub const RET_F_GET_MODE: &str = "<u16*>";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Event(EventParams),
    F(FParams),
    Fw(FwParams),
    FGetMode(FGetModeParams),
    FSetMode(FSetModeParams),
    G(GParams),
    Gh3xChipCtrl(Gh3xChipCtrlParams),
    Gh3xGetVersion(Gh3xGetVersionParams),
    Gh3xRegBitFieldWriteCmd(Gh3xRegBitFieldWriteCmdParams),
    Gh3xRegsBitFieldWriteCmd(Gh3xRegsBitFieldWriteCmdParams),
    Gh3xRegsListWriteCmd(Gh3xRegsListWriteCmdParams),
    Gh3xRegsReadCmd(Gh3xRegsReadCmdParams),
    Gh3xRegsWriteCmd(Gh3xRegsWriteCmdParams),
    Gh3xSwFunctionCmd(Gh3xSwFunctionCmdParams),
    GhSetWorkModeCmd(GhSetWorkModeCmdParams),
    DownloadConfig(DownloadConfigParams),
    GetChipLinkStatus(GetChipLinkStatusParams),
    GhLowPowerCmd(GhLowPowerCmdParams),
    GhTimeSet(GhTimeSetParams),
    GhTimestampSet(GhTimestampSetParams),
}

impl Command {
    pub fn key(&self) -> &'static str {
        match self {
            Command::Event(_) => KEY_EVENT,
            Command::F(_) => KEY_F,
            Command::Fw(_) => KEY_FW,
            Command::FGetMode(_) => KEY_F_GET_MODE,
            Command::FSetMode(_) => KEY_F_SET_MODE,
            Command::G(_) => KEY_G,
            Command::Gh3xChipCtrl(_) => KEY_GH3X_CHIP_CTRL,
            Command::Gh3xGetVersion(_) => KEY_GH3X_GET_VERSION,
            Command::Gh3xRegBitFieldWriteCmd(_) => KEY_GH3X_REG_BIT_FIELD_WRITE_CMD,
            Command::Gh3xRegsBitFieldWriteCmd(_) => KEY_GH3X_REGS_BIT_FIELD_WRITE_CMD,
            Command::Gh3xRegsListWriteCmd(_) => KEY_GH3X_REGS_LIST_WRITE_CMD,
            Command::Gh3xRegsReadCmd(_) => KEY_GH3X_REGS_READ_CMD,
            Command::Gh3xRegsWriteCmd(_) => KEY_GH3X_REGS_WRITE_CMD,
            Command::Gh3xSwFunctionCmd(_) => KEY_GH3X_SW_FUNCTION_CMD,
            Command::GhSetWorkModeCmd(_) => KEY_GH_SET_WORK_MODE_CMD,
            Command::DownloadConfig(_) => KEY_DOWNLOAD_CONFIG,
            Command::GetChipLinkStatus(_) => KEY_GET_CHIP_LINK_STATUS,
            Command::GhLowPowerCmd(_) => KEY_GH_LOW_POWER_CMD,
            Command::GhTimeSet(_) => KEY_GH_TIME_SET,
            Command::GhTimestampSet(_) => KEY_GH_TIMESTAMP_SET,
        }
    }

    pub fn format(&self) -> &'static str {
        match self {
            Command::Event(_) => FMT_EVENT,
            Command::F(_) => FMT_F,
            Command::Fw(_) => FMT_FW,
            Command::FGetMode(_) => FMT_F_GET_MODE,
            Command::FSetMode(_) => FMT_F_SET_MODE,
            Command::G(_) => FMT_G,
            Command::Gh3xChipCtrl(_) => FMT_GH3X_CHIP_CTRL,
            Command::Gh3xGetVersion(_) => FMT_GH3X_GET_VERSION,
            Command::Gh3xRegBitFieldWriteCmd(_) => FMT_GH3X_REG_BIT_FIELD_WRITE_CMD,
            Command::Gh3xRegsBitFieldWriteCmd(_) => FMT_GH3X_REGS_BIT_FIELD_WRITE_CMD,
            Command::Gh3xRegsListWriteCmd(_) => FMT_GH3X_REGS_LIST_WRITE_CMD,
            Command::Gh3xRegsReadCmd(_) => FMT_GH3X_REGS_READ_CMD,
            Command::Gh3xRegsWriteCmd(_) => FMT_GH3X_REGS_WRITE_CMD,
            Command::Gh3xSwFunctionCmd(_) => FMT_GH3X_SW_FUNCTION_CMD,
            Command::GhSetWorkModeCmd(_) => FMT_GH_SET_WORK_MODE_CMD,
            Command::DownloadConfig(_) => FMT_DOWNLOAD_CONFIG,
            Command::GetChipLinkStatus(_) => FMT_GET_CHIP_LINK_STATUS,
            Command::GhLowPowerCmd(_) => FMT_GH_LOW_POWER_CMD,
            Command::GhTimeSet(_) => FMT_GH_TIME_SET,
            Command::GhTimestampSet(_) => FMT_GH_TIMESTAMP_SET,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Response {
    Gh3xGetVersion(Vec<u8>),
    Gh3xRegsReadCmd(Vec<u16>),
    Fw(Vec<u8>),
    GetChipLinkStatus(Vec<i8>),
    FGetMode(Vec<u16>),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventParams {
    pub buf: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FParams {
    pub buf: Vec<u8>,
    pub fifo_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FwParams {
    pub src: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FGetModeParams {
    pub test_mode: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FSetModeParams {
    pub test_mode: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GParams {
    pub buf: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xChipCtrlParams {
    pub ctrl_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xGetVersionParams {
    pub ver_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xRegBitFieldWriteCmdParams {
    pub reg_addr: u16,
    pub lsb: u8,
    pub msb: u8,
    pub reg_val: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xRegsBitFieldWriteCmdParams {
    pub reg_bits: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xRegsListWriteCmdParams {
    pub regs: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xRegsReadCmdParams {
    pub reg_addr: u16,
    pub read_len: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xRegsWriteCmdParams {
    pub regs: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gh3xSwFunctionCmdParams {
    pub target_func_mode: u32,
    pub ctrl_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhSetWorkModeCmdParams {
    pub work_mode: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadConfigParams {
    pub stage: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetChipLinkStatusParams {
    pub link_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhLowPowerCmdParams {
    pub target_func_mode: u32,
    pub ctrl_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhTimeSetParams {
    pub ts: u32,
    pub hour_offset: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhTimestampSetParams {
    pub ts: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_keys() {
        assert_eq!(KEY_EVENT, "Event");
        assert_eq!(KEY_F, "F");
        assert_eq!(KEY_FW, "FW");
        assert_eq!(KEY_F_GET_MODE, "F_GetMode");
        assert_eq!(KEY_F_SET_MODE, "F_SetMode");
        assert_eq!(KEY_G, "G");
        assert_eq!(KEY_GH3X_CHIP_CTRL, "GH3X_ChipCtrl");
        assert_eq!(KEY_GH3X_GET_VERSION, "GH3X_GetVersion");
    }

    #[test]
    fn test_command_key_method() {
        let cmd = Command::Event(EventParams { buf: vec![] });
        assert_eq!(cmd.key(), KEY_EVENT);

        let cmd = Command::Gh3xGetVersion(Gh3xGetVersionParams { ver_type: 0 });
        assert_eq!(cmd.key(), KEY_GH3X_GET_VERSION);
    }

    #[test]
    fn test_command_format_method() {
        let cmd = Command::Event(EventParams { buf: vec![] });
        assert_eq!(cmd.format(), FMT_EVENT);

        let cmd = Command::Gh3xRegBitFieldWriteCmd(Gh3xRegBitFieldWriteCmdParams {
            reg_addr: 0,
            lsb: 0,
            msb: 0,
            reg_val: 0,
        });
        assert_eq!(cmd.format(), FMT_GH3X_REG_BIT_FIELD_WRITE_CMD);
    }
}
