//! GH3036 协议库 FFI 绑定
//!
//! 本模块提供与 C 库 `gh_protocol` 的 FFI 绑定
//!
//! ## 内存布局
//! - PackHeader: 32 bit 位域结构，总大小 4 字节
//! - DataFrame: 与 C 库 `data_frame_t` 内存布局一致
//!
//! ## 线程安全
//! - 回调函数可能在 C 库线程中调用，需要确保线程安全
//! - lock/unlock 回调用于保护共享数据

use std::os::raw::{c_char, c_int, c_void};

/// GH3036 协议句柄类型
///
/// 由 `gh_protocol_create` 创建，`gh_protocol_destroy` 销毁
/// 代表一个协议实例的所有状态
pub type GhProtocolHandle = c_void;

/// 线程锁回调函数类型
///
/// # 功能
/// 获取互斥锁，保护共享数据访问
///
/// # 线程安全
/// 必须是线程安全的，可能在任意线程调用
pub type GhProtocolLockFn = Option<unsafe extern "C" fn()>;

/// 线程解锁回调函数类型
///
/// # 功能
/// 释放互斥锁，允许其他线程访问共享数据
///
/// # 线程安全
/// 必须与 lock 配对使用
pub type GhProtocolUnlockFn = Option<unsafe extern "C" fn()>;

/// 延迟回调函数类型
///
/// # 功能
/// 阻塞当前线程指定时间
///
/// # 参数
/// - `ms`: 延迟时间，单位毫秒
///
/// # 注意
/// 在异步环境中需要特殊处理
pub type GhProtocolDelayFn = Option<unsafe extern "C" fn(u32)>;

/// 发送回调函数类型
///
/// # 功能
/// 将数据通过配置的 TX 通道发送
///
/// # 参数
/// - `data`: 待发送的数据指针
/// - `size`: 数据长度（字节）
///
/// # 返回
/// 无返回值，发送失败应记录错误日志
///
/// # 线程安全
/// 可能在 C 库线程中调用
pub type GhProtocolSendFn = Option<unsafe extern "C" fn(*mut c_void, c_int)>;

/// 事件回调函数类型
///
/// # 功能
/// 处理 RPC 响应事件，推送事件到前端显示
///
/// # 参数
/// - `event_type`: 事件类型（如响应、错误等）
/// - `data`: 事件数据指针
/// - `size`: 数据长度（字节）
///
/// # 线程安全
/// 可能在 C 库线程中调用
pub type GhProtocolEventFn = Option<unsafe extern "C" fn(u8, *mut u8, u32)>;

/// 帧数据回调函数类型
///
/// # 功能
/// 处理解析完成的帧数据，推送数据到前端并保存 CSV
///
/// # 参数
/// - `frame`: 帧数据结构指针，包含所有解析后的数据字段
///
/// # 数据字段
/// - `gs_data`: 加速度/陀螺仪数据
/// - `rawdata`: 原始数据
/// - `flags`: 标志位
/// - `algo_data`: 算法结果
/// - `agc_info`: AGC 信息
/// - `phy_value`: 物理值
///
/// # 线程安全
/// 可能在 C 库线程中调用
pub type GhProtocolFrameFn = Option<unsafe extern "C" fn(*mut DataFrame)>;

/// GH3036 协议配置结构体
///
/// # 功能
/// 配置协议实例的回调函数
///
/// # 字段说明
/// - `lock`: 线程锁回调，保护共享数据
/// - `unlock`: 线程解锁回调
/// - `delay`: 延迟回调，用于 C 库内部延时
/// - `send`: 发送回调，C 库通过此回调发送数据
/// - `event_callback`: 事件回调，处理 RPC 响应
/// - `frame_callback`: 帧回调，处理解析后的帧数据
///
/// # 使用示例
/// ```rust
/// let config = GhProtocolConfig {
///     lock: Some(lock_callback),
///     unlock: Some(unlock_callback),
///     delay: Some(delay_callback),
///     send: Some(send_callback),
///     event_callback: Some(event_callback),
///     frame_callback: Some(frame_callback),
/// };
/// let handle = gh_protocol_create(&config);
/// ```
#[repr(C)]
pub struct GhProtocolConfig {
    /// 线程锁回调
    pub lock: GhProtocolLockFn,
    /// 线程解锁回调
    pub unlock: GhProtocolUnlockFn,
    /// 延迟回调（参数：毫秒）
    pub delay: GhProtocolDelayFn,
    /// 发送回调（参数：数据指针, 数据长度）
    pub send: GhProtocolSendFn,
    /// 事件回调（参数：事件类型, 数据指针, 数据长度）
    pub event_callback: GhProtocolEventFn,
    /// 帧数据回调（参数：帧数据指针）
    pub frame_callback: GhProtocolFrameFn,
}

/// 帧数据包头位域结构
///
/// # 内存布局
/// 总大小：32 bit（4 字节）
///
/// # 位域定义
/// | 位范围 | 字段 | 说明 |
/// |--------|------|------|
/// | 0 | rawdata_en | 原始数据使能 |
/// | 1 | phy_value_en | 物理值使能 |
/// | 2 | gs_data_en | 加速度/陀螺仪数据使能 |
/// | 3 | flags_en | 标志位使能 |
/// | 4 | alg_data_en | 算法数据使能 |
/// | 5 | agc_info_en | AGC 信息使能 |
/// | 6 | timestamp_en | 时间戳使能 |
/// | 7 | frameid_en | 帧 ID 使能 |
/// | 8 | func_id_en | 功能 ID 使能 |
/// | 9 | slot_cfg_en | 插槽配置使能 |
/// | 10-31 | reserved | 保留位 |
///
/// # 注意
/// C 库使用位域结构，Rust 使用 u32 和位操作模拟
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PackHeader {
    /// 位域值（32 bit）
    /// bit 0: rawdata_en
    /// bit 1: phy_value_en
    /// bit 2: gs_data_en
    /// bit 3: flags_en
    /// bit 4: alg_data_en
    /// bit 5: agc_info_en
    /// bit 6: timestamp_en
    /// bit 7: frameid_en
    /// bit 8: func_id_en
    /// bit 9: slot_cfg_en
    /// bit 10-31: reserved
    pub bits: u32,
}

impl PackHeader {
    /// 创建新的 PackHeader
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    /// 原始数据使能
    pub fn rawdata_en(&self) -> bool {
        (self.bits & (1 << 0)) != 0
    }

    /// 设置原始数据使能
    pub fn set_rawdata_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 0;
        } else {
            self.bits &= !(1 << 0);
        }
    }

    /// 物理值使能
    pub fn phy_value_en(&self) -> bool {
        (self.bits & (1 << 1)) != 0
    }

    /// 设置物理值使能
    pub fn set_phy_value_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 1;
        } else {
            self.bits &= !(1 << 1);
        }
    }

    /// 加速度/陀螺仪数据使能
    pub fn gs_data_en(&self) -> bool {
        (self.bits & (1 << 2)) != 0
    }

    /// 设置加速度/陀螺仪数据使能
    pub fn set_gs_data_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 2;
        } else {
            self.bits &= !(1 << 2);
        }
    }

    /// 标志位使能
    pub fn flags_en(&self) -> bool {
        (self.bits & (1 << 3)) != 0
    }

    /// 设置标志位使能
    pub fn set_flags_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 3;
        } else {
            self.bits &= !(1 << 3);
        }
    }

    /// 算法数据使能
    pub fn alg_data_en(&self) -> bool {
        (self.bits & (1 << 4)) != 0
    }

    /// 设置算法数据使能
    pub fn set_alg_data_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 4;
        } else {
            self.bits &= !(1 << 4);
        }
    }

    /// AGC 信息使能
    pub fn agc_info_en(&self) -> bool {
        (self.bits & (1 << 5)) != 0
    }

    /// 设置 AGC 信息使能
    pub fn set_agc_info_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 5;
        } else {
            self.bits &= !(1 << 5);
        }
    }

    /// 时间戳使能
    pub fn timestamp_en(&self) -> bool {
        (self.bits & (1 << 6)) != 0
    }

    /// 设置时间戳使能
    pub fn set_timestamp_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 6;
        } else {
            self.bits &= !(1 << 6);
        }
    }

    /// 帧 ID 使能
    pub fn frameid_en(&self) -> bool {
        (self.bits & (1 << 7)) != 0
    }

    /// 设置帧 ID 使能
    pub fn set_frameid_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 7;
        } else {
            self.bits &= !(1 << 7);
        }
    }

    /// 功能 ID 使能
    pub fn func_id_en(&self) -> bool {
        (self.bits & (1 << 8)) != 0
    }

    /// 设置功能 ID 使能
    pub fn set_func_id_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 8;
        } else {
            self.bits &= !(1 << 8);
        }
    }

    /// 插槽配置使能
    pub fn slot_cfg_en(&self) -> bool {
        (self.bits & (1 << 9)) != 0
    }

    /// 设置插槽配置使能
    pub fn set_slot_cfg_en(&mut self, enable: bool) {
        if enable {
            self.bits |= 1 << 9;
        } else {
            self.bits &= !(1 << 9);
        }
    }
}

/// 帧数据结构体
///
/// # 功能
/// 存储解析后的帧数据，包含所有数据字段
///
/// # 内存布局
/// 与 C 库 `data_frame_t` 内存布局一致
///
/// # 字段说明
/// - `pack_header`: 包头位域结构（4 字节）
/// - `slot_cfg`: 插槽配置
/// - `function_id`: 功能 ID（对应 GhFuncFixIdx）
/// - `frame_id`: 帧 ID（每帧递增，0 表示新序列开始）
/// - `timestamp`: 时间戳低 32 位
/// - `timestamp_high`: 时间戳高 32 位
/// - `p_agc_info`: AGC 信息数组指针
/// - `p_agc_info_high`: AGC 信息高 32 位数组指针
/// - `agc_info_size`: AGC 信息数组大小
/// - `p_algo_data`: 算法数据数组指针
/// - `algo_data_bits`: 算法数据位数
/// - `p_flags`: 标志位数组指针
/// - `flag_data_bits`: 标志位数据位数
/// - `p_gs_data`: 加速度/陀螺仪数据数组指针
/// - `gs_data_size`: 加速度/陀螺仪数据数组大小
/// - `p_phy_value`: 物理值数组指针
/// - `phy_value_size`: 物理值数组大小
/// - `p_rawdata`: 原始数据数组指针
/// - `rawdata_size`: 原始数据数组大小
#[repr(C)]
pub struct DataFrame {
    /// 包头位域结构（4 字节）
    pub pack_header: PackHeader,
    /// 插槽配置
    pub slot_cfg: i32,
    /// 功能 ID（对应 GhFuncFixIdx 枚举）
    pub function_id: i32,
    /// 帧 ID（每帧递增，0 表示新序列开始）
    pub frame_id: i32,
    /// 时间戳低 32 位
    pub timestamp: i32,
    /// 时间戳高 32 位
    pub timestamp_high: i32,
    /// AGC 信息数组指针
    pub p_agc_info: *mut i32,
    /// AGC 信息高 32 位数组指针
    pub p_agc_info_high: *mut i32,
    /// AGC 信息数组大小
    pub agc_info_size: i32,
    /// 算法数据数组指针
    pub p_algo_data: *mut i32,
    /// 算法数据位数
    pub algo_data_bits: i32,
    /// 标志位数组指针
    pub p_flags: *mut i32,
    /// 标志位数据位数
    pub flag_data_bits: i32,
    /// 加速度/陀螺仪数据数组指针
    pub p_gs_data: *mut i32,
    /// 加速度/陀螺仪数据数组大小
    pub gs_data_size: i32,
    /// 物理值数组指针
    pub p_phy_value: *mut i32,
    /// 物理值数组大小
    pub phy_value_size: i32,
    /// 原始数据数组指针
    pub p_rawdata: *mut i32,
    /// 原始数据数组大小
    pub rawdata_size: i32,
}

extern "C" {
    /// 创建 GH3036 协议实例
    ///
    /// # 功能
    /// 初始化协议实例，配置回调函数
    ///
    /// # 参数
    /// - `config`: 配置结构体指针，包含所有回调函数
    ///
    /// # 返回
    /// 协议句柄指针，失败返回空指针
    ///
    /// # 线程安全
    /// 可以在任意线程调用
    ///
    /// # 使用示例
    /// ```rust
    /// let handle = gh_protocol_create(&config);
    /// if handle.is_null() {
    ///     // 处理错误
    /// }
    /// ```
    pub fn gh_protocol_create(config: *const GhProtocolConfig) -> *mut GhProtocolHandle;

    /// 销毁 GH3036 协议实例
    ///
    /// # 功能
    /// 释放协议实例资源
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    ///
    /// # 注意
    /// 销毁后句柄不再有效
    pub fn gh_protocol_destroy(handle: *mut GhProtocolHandle);

    /// 处理接收数据
    ///
    /// # 功能
    /// 将接收的数据传入协议库处理
    /// 协议库会解析数据并通过 frame_callback 返回结果
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `data`: 接收的数据指针
    /// - `size`: 数据长度（字节）
    ///
    /// # 返回
    /// - `>= 0`: 成功处理的字节数
    /// - `< 0`: 错误码
    ///
    /// # 线程安全
    /// 可以在任意线程调用，内部使用 lock/unlock 保护
    ///
    /// # 使用示例
    /// ```rust
    /// let result = gh_protocol_receive(handle, data.as_mut_ptr(), data.len() as u32);
    /// if result < 0 {
    ///     // 处理错误
    /// }
    /// ```
    pub fn gh_protocol_receive(
        handle: *mut GhProtocolHandle,
        data: *mut u8,
        size: u32,
    ) -> c_int;

    /// 发送原始数据
    ///
    /// # 功能
    /// 通过 send_callback 发送格式化的数据
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `key`: 命令键（如 "V" 表示获取版本）
    /// - `format`: 格式化字符串
    /// - `...`: 可变参数
    ///
    /// # 返回
    /// - `>= 0`: 成功发送的字节数
    /// - `< 0`: 错误码
    ///
    /// # 使用示例
    /// ```rust
    /// // 发送获取版本命令
    /// gh_protocol_send_raw(handle, b"V\0".as_ptr() as *const c_char, b"%d\0".as_ptr() as *const c_char, 0);
    /// ```
    pub fn gh_protocol_send_raw(
        handle: *mut GhProtocolHandle,
        key: *const c_char,
        format: *const c_char,
        ...
    ) -> c_int;

    /// 寄存器写入命令
    ///
    /// # 功能
    /// 写入芯片寄存器
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `regs`: 寄存器数据数组（地址和值交替）
    /// - `size`: 数据长度（16 位字数）
    ///
    /// # 使用示例
    /// ```rust
    /// // 写入寄存器 0x1000 = 0x1234
    /// let regs: [u16; 2] = [0x1000, 0x1234];
    /// gh_protocol_regs_write(handle, regs.as_mut_ptr(), 2);
    /// ```
    pub fn gh_protocol_regs_write(
        handle: *mut GhProtocolHandle,
        regs: *mut u16,
        size: i32,
    );

    /// 寄存器读取命令
    ///
    /// # 功能
    /// 读取芯片寄存器
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `reg_addr`: 寄存器起始地址
    /// - `read_len`: 读取长度（16 位字数）
    /// - `reg_value`: 输出缓冲区，存储读取的值
    /// - `len`: 输出实际读取的长度
    pub fn gh_protocol_regs_read(
        handle: *mut GhProtocolHandle,
        reg_addr: u16,
        read_len: i32,
        reg_value: *mut u16,
        len: *mut i32,
    );

    /// 寄存器位域写入命令
    ///
    /// # 功能
    /// 写入寄存器的特定位域
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `reg_addr`: 寄存器地址
    /// - `lsb`: 最低位位置（0-15）
    /// - `msb`: 最高位位置（0-15）
    /// - `reg_val`: 要写入的值
    ///
    /// # 使用示例
    /// ```rust
    /// // 写入寄存器 0x1000 的 bit 4-7，值为 0b1010
    /// gh_protocol_reg_bitfield_write(handle, 0x1000, 4, 7, 0b1010);
    /// ```
    pub fn gh_protocol_reg_bitfield_write(
        handle: *mut GhProtocolHandle,
        reg_addr: u16,
        lsb: u8,
        msb: u8,
        reg_val: u16,
    );

    /// 芯片控制命令
    ///
    /// # 功能
    /// 发送芯片控制命令（如复位、休眠等）
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `ctrl_type`: 控制类型
    pub fn gh_protocol_chip_ctrl(
        handle: *mut GhProtocolHandle,
        ctrl_type: u8,
    );

    /// 下载配置命令
    ///
    /// # 功能
    /// 下载配置到芯片
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `stage`: 下载阶段（多阶段下载）
    pub fn gh_protocol_download_config(
        handle: *mut GhProtocolHandle,
        stage: u8,
    );

    /// 寄存器列表写入命令
    ///
    /// # 功能
    /// 批量写入多个寄存器
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `regs`: 寄存器数据数组
    /// - `size`: 数据长度（16 位字数）
    pub fn gh_protocol_regs_list_write(
        handle: *mut GhProtocolHandle,
        regs: *mut u16,
        size: u16,
    );

    /// 软件功能命令
    ///
    /// # 功能
    /// 发送软件功能命令
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `target_func_mode`: 目标功能模式
    /// - `ctrl_type`: 控制类型
    pub fn gh_protocol_sw_function_cmd(
        handle: *mut GhProtocolHandle,
        target_func_mode: u32,
        ctrl_type: u8,
    );

    /// 低功耗命令
    ///
    /// # 功能
    /// 设置低功耗模式
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `target_func_mode`: 目标功能模式
    /// - `ctrl_type`: 控制类型
    pub fn gh_protocol_low_power_cmd(
        handle: *mut GhProtocolHandle,
        target_func_mode: u32,
        ctrl_type: u8,
    );

    /// 固件更新命令
    ///
    /// # 功能
    /// 更新芯片固件
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `src`: 固件数据指针
    /// - `len`: 固件数据长度
    /// - `ret`: 返回数据缓冲区
    /// - `ret_len`: 返回数据长度
    pub fn gh_protocol_fw_update(
        handle: *mut GhProtocolHandle,
        src: *mut u8,
        len: u32,
        ret: *mut u8,
        ret_len: *mut u32,
    );

    /// 设置工作模式
    ///
    /// # 功能
    /// 设置芯片工作模式
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `work_mode`: 工作模式
    pub fn gh_protocol_set_work_mode(
        handle: *mut GhProtocolHandle,
        work_mode: u8,
    );

    /// 获取芯片链路状态
    ///
    /// # 功能
    /// 查询芯片与主机的链路状态
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `status_type`: 状态类型
    /// - `status`: 输出状态缓冲区
    /// - `len`: 输出状态长度
    pub fn gh_protocol_get_chip_link_status(
        handle: *mut GhProtocolHandle,
        status_type: u8,
        status: *mut i8,
        len: *mut i32,
    );

    /// 设置时间戳
    ///
    /// # 功能
    /// 设置芯片时间戳（32 位）
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `timestamp`: 时间戳值
    pub fn gh_protocol_timestamp_set(
        handle: *mut GhProtocolHandle,
        timestamp: u32,
    );

    /// 设置时间
    ///
    /// # 功能
    /// 设置芯片时间（带时区）
    ///
    /// # 参数
    /// - `handle`: 协议句柄指针
    /// - `timestamp`: 时间戳值
    /// - `hour_offset`: 时区偏移（小时）
    pub fn gh_protocol_time_set(
        handle: *mut GhProtocolHandle,
        timestamp: u32,
        hour_offset: i8,
    );

    /// 字节数组转帧数据
    ///
    /// # 功能
    /// 将原始字节数组解析为帧数据结构
    ///
    /// # 参数
    /// - `buffer`: 输入字节数组指针
    /// - `buffer_size`: 字节数组大小
    /// - `frame`: 输出帧数据结构指针
    ///
    /// # 返回
    /// - `>= 0`: 成功
    /// - `< 0`: 错误码
    pub fn gh_protocol_bytes_to_frame(
        buffer: *mut u8,
        buffer_size: i32,
        frame: *mut DataFrame,
    ) -> c_int;
}

/// 检查 C 库是否已链接
///
/// # 返回
/// - `true`: C 库已链接
/// - `false`: C 库未链接，使用纯 Rust 模式
pub fn is_linked() -> bool {
    option_env!("GH_PROTOCOL_LINKED").is_some()
}
