use std::os::raw::{c_char, c_int, c_void};

pub type GhProtocolHandle = c_void;

pub type GhProtocolLockFn = Option<unsafe extern "C" fn()>;
pub type GhProtocolUnlockFn = Option<unsafe extern "C" fn()>;
pub type GhProtocolDelayFn = Option<unsafe extern "C" fn()>;
pub type GhProtocolSendFn = Option<unsafe extern "C" fn(*mut c_void, c_int)>;
pub type GhProtocolEventFn = Option<unsafe extern "C" fn(u8, *mut u8, u32)>;
pub type GhProtocolFrameFn = Option<unsafe extern "C" fn(*mut DataFrame)>;

#[repr(C)]
pub struct GhProtocolConfig {
    pub lock: GhProtocolLockFn,
    pub unlock: GhProtocolUnlockFn,
    pub delay: GhProtocolDelayFn,
    pub send: GhProtocolSendFn,
    pub event_callback: GhProtocolEventFn,
    pub frame_callback: GhProtocolFrameFn,
}

#[repr(C, packed)]
pub struct PackHeader {
    pub rawdata_en: u32,
    pub phy_value_en: u32,
    pub gs_data_en: u32,
    pub flags_en: u32,
    pub alg_data_en: u32,
    pub agc_info_en: u32,
    pub timestamp_en: u32,
    pub frameid_en: u32,
    pub func_id_en: u32,
    pub slot_cfg_en: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct DataFrame {
    pub pack_header: PackHeader,
    pub slot_cfg: i32,
    pub function_id: i32,
    pub frame_id: i32,
    pub timestamp: i32,
    pub timestamp_high: i32,
    pub p_agc_info: *mut i32,
    pub p_agc_info_high: *mut i32,
    pub agc_info_size: i32,
    pub p_algo_data: *mut i32,
    pub algo_data_bits: i32,
    pub p_flags: *mut i32,
    pub flag_data_bits: i32,
    pub p_gs_data: *mut i32,
    pub gs_data_size: i32,
    pub p_phy_value: *mut i32,
    pub phy_value_size: i32,
    pub p_rawdata: *mut i32,
    pub rawdata_size: i32,
}

extern "C" {
    pub fn gh_protocol_create(config: *const GhProtocolConfig) -> *mut GhProtocolHandle;
    pub fn gh_protocol_destroy(handle: *mut GhProtocolHandle);
    
    pub fn gh_protocol_receive(
        handle: *mut GhProtocolHandle,
        data: *mut u8,
        size: u32,
    ) -> c_int;
    
    pub fn gh_protocol_send_raw(
        handle: *mut GhProtocolHandle,
        key: *const c_char,
        format: *const c_char,
        ...
    ) -> c_int;
    
    pub fn gh_protocol_regs_write(
        handle: *mut GhProtocolHandle,
        regs: *mut u16,
        size: i32,
    );
    
    pub fn gh_protocol_regs_read(
        handle: *mut GhProtocolHandle,
        reg_addr: u16,
        read_len: i32,
        reg_value: *mut u16,
        len: *mut i32,
    );
    
    pub fn gh_protocol_reg_bitfield_write(
        handle: *mut GhProtocolHandle,
        reg_addr: u16,
        lsb: u8,
        msb: u8,
        reg_val: u16,
    );
    
    pub fn gh_protocol_chip_ctrl(
        handle: *mut GhProtocolHandle,
        ctrl_type: u8,
    );
    
    pub fn gh_protocol_download_config(
        handle: *mut GhProtocolHandle,
        stage: u8,
    );
    
    pub fn gh_protocol_regs_list_write(
        handle: *mut GhProtocolHandle,
        regs: *mut u16,
        size: u16,
    );
    
    pub fn gh_protocol_sw_function_cmd(
        handle: *mut GhProtocolHandle,
        target_func_mode: u32,
        ctrl_type: u8,
    );
    
    pub fn gh_protocol_low_power_cmd(
        handle: *mut GhProtocolHandle,
        target_func_mode: u32,
        ctrl_type: u8,
    );
    
    pub fn gh_protocol_fw_update(
        handle: *mut GhProtocolHandle,
        src: *mut u8,
        len: u32,
        ret: *mut u8,
        ret_len: *mut u32,
    );
    
    pub fn gh_protocol_set_work_mode(
        handle: *mut GhProtocolHandle,
        work_mode: u8,
    );
    
    pub fn gh_protocol_get_chip_link_status(
        handle: *mut GhProtocolHandle,
        status_type: u8,
        status: *mut i8,
        len: *mut i32,
    );
    
    pub fn gh_protocol_timestamp_set(
        handle: *mut GhProtocolHandle,
        timestamp: u32,
    );
    
    pub fn gh_protocol_time_set(
        handle: *mut GhProtocolHandle,
        timestamp: u32,
        hour_offset: i8,
    );
    
    pub fn gh_protocol_bytes_to_frame(
        buffer: *mut u8,
        buffer_size: i32,
        frame: *mut DataFrame,
    ) -> c_int;
}

pub fn is_linked() -> bool {
    option_env!("GH_PROTOCOL_LINKED").is_some()
}
