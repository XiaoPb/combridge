//! GH3036 协议库线程同步机制
//!
//! 本模块实现 C 库所需的线程同步回调函数：
//! - `lock`: 获取互斥锁
//! - `unlock`: 释放互斥锁
//! - `delay`: 延迟函数
//!
//! ## 线程安全
//! 所有回调函数都是线程安全的，可以在任意线程调用
//!
//! ## 实现说明
//! 由于 C 库的 lock/unlock 是分开调用的，我们需要一种方式来手动管理锁的生命周期。
//! 这里使用 `parking_lot` crate 的 Mutex，它支持手动锁定和解锁。

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 使用 parking_lot 的 Mutex，支持手动锁定/解锁
/// 如果不想添加依赖，可以使用 spin::Mutex 作为替代
mod inner {
    use std::sync::atomic::{AtomicBool, Ordering};
    
    /// 简单的自旋锁实现
    /// 用于 C 库的 lock/unlock 回调
    pub struct SpinLock {
        locked: AtomicBool,
    }
    
    impl SpinLock {
        pub const fn new() -> Self {
            Self {
                locked: AtomicBool::new(false),
            }
        }
        
        /// 获取锁
        /// 如果锁已被其他线程持有，则自旋等待
        pub fn lock(&self) {
            while self.locked.compare_exchange_weak(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed,
            ).is_err() {
                std::hint::spin_loop();
            }
        }
        
        /// 释放锁
        pub fn unlock(&self) {
            self.locked.store(false, Ordering::Release);
        }
        
        /// 检查锁是否被持有
        pub fn is_locked(&self) -> bool {
            self.locked.load(Ordering::Acquire)
        }
    }
}

/// 全局自旋锁
///
/// 用于保护 C 库和 Rust 之间的共享数据访问
/// 自旋锁适合短时间的锁定操作
static GLOBAL_LOCK: inner::SpinLock = inner::SpinLock::new();

/// 延迟状态标志
///
/// 标记当前是否正在执行延迟操作
static DELAYING: AtomicBool = AtomicBool::new(false);

/// 线程锁回调 - 获取互斥锁
///
/// # 功能
/// 获取全局互斥锁，保护共享数据访问
///
/// # 线程安全
/// - 此函数是线程安全的
/// - 如果锁已被其他线程持有，当前线程将自旋等待
///
/// # 注意
/// - 此函数可能在 C 库线程中调用
/// - 必须与 unlock 配对使用
/// - 避免死锁：不要在持有锁时调用可能再次获取锁的函数
/// - 避免长时间持有锁：自旋锁不适合长时间锁定
#[no_mangle]
pub unsafe extern "C" fn gh_protocol_lock() {
    GLOBAL_LOCK.lock();
}

/// 线程解锁回调 - 释放互斥锁
///
/// # 功能
/// 释放全局互斥锁，允许其他线程访问共享数据
///
/// # 线程安全
/// - 此函数是线程安全的
/// - 必须与 lock 配对使用
///
/// # 注意
/// - 此函数可能在 C 库线程中调用
/// - 调用时必须确保当前线程持有锁
#[no_mangle]
pub unsafe extern "C" fn gh_protocol_unlock() {
    GLOBAL_LOCK.unlock();
}

/// 延迟回调 - 阻塞线程指定时间
///
/// # 功能
/// 阻塞当前线程指定毫秒数
///
/// # 参数
/// - `ms`: 延迟时间，单位毫秒
///
/// # 实现
/// 使用 `std::thread::sleep` 实现同步延迟
///
/// # 注意
/// - 此函数可能在 C 库线程中调用
/// - 在异步环境中会阻塞当前线程
/// - 延迟精度取决于操作系统调度
/// - ms = 0 时立即返回
#[no_mangle]
pub unsafe extern "C" fn gh_protocol_delay(ms: u32) {
    if ms == 0 {
        return;
    }
    
    DELAYING.store(true, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(ms as u64));
    DELAYING.store(false, Ordering::SeqCst);
}

/// 检查是否正在延迟中
///
/// # 返回
/// - `true`: 正在执行延迟操作
/// - `false`: 未在延迟
pub fn is_delaying() -> bool {
    DELAYING.load(Ordering::SeqCst)
}

/// 获取锁状态
///
/// # 返回
/// - `true`: 锁已被持有
/// - `false`: 锁未被持有
///
/// # 注意
/// 此函数仅用于调试，返回值可能立即过时
pub fn is_locked() -> bool {
    GLOBAL_LOCK.is_locked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn test_lock_unlock() {
        unsafe {
            gh_protocol_lock();
            assert!(is_locked());
            gh_protocol_unlock();
            assert!(!is_locked());
        }
    }

    #[test]
    fn test_delay() {
        let start = Instant::now();
        unsafe {
            gh_protocol_delay(100);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 100);
        assert!(!is_delaying());
    }

    #[test]
    fn test_delay_zero() {
        let start = Instant::now();
        unsafe {
            gh_protocol_delay(0);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 10);
    }

    #[test]
    fn test_concurrent_lock() {
        let start = Instant::now();
        
        let handle = thread::spawn(|| {
            unsafe {
                gh_protocol_lock();
                thread::sleep(Duration::from_millis(50));
                gh_protocol_unlock();
            }
        });

        thread::sleep(Duration::from_millis(10));
        
        unsafe {
            gh_protocol_lock();
            gh_protocol_unlock();
        }
        let elapsed = start.elapsed();
        
        handle.join().unwrap();
        
        assert!(elapsed.as_millis() >= 40);
    }
}
