//! 固件结构与校验。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

pub mod boot_info;
pub mod chip_info;
pub mod image;
pub mod img_info;
pub mod overlap;
pub mod types;

pub use boot_info::{regions_overlap, BootInfo, BOOT_INFO_LEN};
pub use chip_info::{ChipInfo, CHIP_INFO_LEN};
pub use image::{sum_checksum, DfuImage};
pub use img_info::{ImgInfo, COMMENTS_LEN, IMG_INFO_LEN, VALID_PATTERN};
pub use overlap::{check_overlap_v1, check_overlap_v2, ensure_no_overlap, SCA_SIZE};
pub use types::{ensure_status, AppBootloaderExtraInfo, ImgInfoList, StartupBootInfo};
