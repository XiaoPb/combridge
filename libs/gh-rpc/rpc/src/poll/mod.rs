//! Memory pool management
//!
//! 提供静态内存池管理，包括：
//! - `SlabMemory`: 泛型 slab 分配器
//! - `LinkedBuffer`: 链表缓冲区
//! - `BufferPool`: 缓冲池管理器

use core::mem::MaybeUninit;

/// 内存池错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolError {
    /// 内存池已满
    PoolFull,
    /// 内存池为空
    PoolEmpty,
    /// 无效索引
    InvalidIndex,
    /// 已经释放
    AlreadyFreed,
}

/// 泛型 Slab 内存分配器
///
/// 提供固定大小的内存块分配和释放，适用于 `no_std` 环境。
///
/// # 类型参数
///
/// * `T` - 存储的元素类型
/// * `N` - 最大元素数量
///
/// # 示例
///
/// ```rust
/// use rpc::poll::SlabMemory;
///
/// let mut slab: SlabMemory<u32, 4> = SlabMemory::new();
///
/// // 分配内存
/// let ptr = slab.alloc().unwrap();
/// *ptr = 42;
///
/// // 释放内存
/// slab.free(ptr as *mut u32).unwrap();
/// ```
pub struct SlabMemory<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    free_list: [u16; N],
    next_free: usize,
    allocated_count: usize,
}

impl<T, const N: usize> SlabMemory<T, N> {
    /// 创建新的 Slab 内存分配器
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            free_list: [0; N],
            next_free: N,
            allocated_count: 0,
        }
    }

    /// 分配一个内存块
    ///
    /// # 返回值
    ///
    /// 成功返回指向分配内存的可变引用，失败返回 `None`（内存池已满）
    pub fn alloc(&mut self) -> Option<&mut T> {
        if self.next_free == 0 {
            return None;
        }

        self.next_free -= 1;
        let idx = self.next_free;

        if idx < N - 1 {
            self.free_list[idx] = self.free_list[idx + 1];
        }

        self.allocated_count += 1;
        unsafe {
            self.buffer[idx].as_mut_ptr().write(MaybeUninit::uninit().assume_init());
        }

        Some(unsafe { &mut *self.buffer[idx].as_mut_ptr() })
    }

    /// 释放一个内存块
    ///
    /// # 参数
    ///
    /// * `ptr` - 要释放的内存指针
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误类型
    pub fn free(&mut self, ptr: *mut T) -> Result<(), PoolError> {
        if ptr.is_null() {
            return Err(PoolError::InvalidIndex);
        }

        let base = self.buffer.as_ptr() as *const T;
        let offset = unsafe { ptr.offset_from(base) };

        if offset < 0 || offset as usize >= N {
            return Err(PoolError::InvalidIndex);
        }

        let idx = offset as usize;

        unsafe {
            ptr.drop_in_place();
            self.buffer[idx] = MaybeUninit::uninit();
        }

        if self.next_free < N {
            self.free_list[self.next_free] = idx as u16;
        }
        self.next_free += 1;

        if self.allocated_count > 0 {
            self.allocated_count -= 1;
        }

        Ok(())
    }

    /// 检查内存池是否已满
    pub fn is_full(&self) -> bool {
        self.next_free == 0
    }

    /// 检查内存池是否为空
    pub fn is_empty(&self) -> bool {
        self.allocated_count == 0
    }

    /// 获取已分配的内存块数量
    pub fn allocated_count(&self) -> usize {
        self.allocated_count
    }
}

impl<T, const N: usize> Drop for SlabMemory<T, N> {
    fn drop(&mut self) {
        if self.allocated_count == 0 {
            return;
        }

        let mut freed = [false; N];
        let mut current = self.next_free;

        while current < N {
            let idx = self.free_list[current] as usize;
            if idx < N {
                freed[idx] = true;
            }
            current += 1;
        }

        for (i, &is_freed) in freed.iter().enumerate() {
            if !is_freed {
                unsafe {
                    self.buffer[i].as_mut_ptr().drop_in_place();
                }
            }
        }
    }
}

unsafe impl<T: Send, const N: usize> Send for SlabMemory<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for SlabMemory<T, N> {}

/// 缓冲区索引信息
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferIndex {
    /// 帧索引
    pub frame_idx: u8,
    /// 调用索引
    pub invoke_idx: u8,
}

/// 链表缓冲区
///
/// 用于存储帧数据并支持链表操作。
///
/// # 类型参数
///
/// * `SIZE` - 缓冲区大小
#[derive(Debug, Clone, Copy)]
pub struct LinkedBuffer<const SIZE: usize> {
    /// 缓冲区数据
    pub buff: [u8; SIZE],
    /// 有效数据长度
    pub length: usize,
    /// 下一个节点索引
    pub next: Option<usize>,
    /// 上一个节点索引
    pub prev: Option<usize>,
    /// 缓冲区索引信息
    pub index: BufferIndex,
}

impl<const SIZE: usize> LinkedBuffer<SIZE> {
    /// 创建新的链表缓冲区
    pub const fn new() -> Self {
        Self {
            buff: [0; SIZE],
            length: 0,
            next: None,
            prev: None,
            index: BufferIndex { frame_idx: 0, invoke_idx: 0 },
        }
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.length = 0;
        self.next = None;
        self.prev = None;
        self.index = BufferIndex::default();
    }
}

impl<const SIZE: usize> Default for LinkedBuffer<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

/// 缓冲池管理器
///
/// 管理多个链表缓冲区，支持分配和释放操作。
///
/// # 类型参数
///
/// * `BUFFER_SIZE` - 单个缓冲区大小
/// * `COUNT` - 缓冲区数量
pub struct BufferPool<const BUFFER_SIZE: usize, const COUNT: usize> {
    buffers: [LinkedBuffer<BUFFER_SIZE>; COUNT],
    free_head: Option<usize>,
    used_head: Option<usize>,
}

impl<const BUFFER_SIZE: usize, const COUNT: usize> BufferPool<BUFFER_SIZE, COUNT> {
    /// 创建新的缓冲池
    pub const fn new() -> Self {
        Self {
            buffers: [LinkedBuffer::new(); COUNT],
            free_head: Some(0),
            used_head: None,
        }
    }

    fn init_free_list(&mut self) {
        for i in 0..COUNT {
            self.buffers[i].next = if i + 1 < COUNT { Some(i + 1) } else { None };
            self.buffers[i].prev = if i > 0 { Some(i - 1) } else { None };
        }
    }

    /// 分配一个缓冲区
    ///
    /// # 返回值
    ///
    /// 成功返回缓冲区的可变引用，失败返回 `None`
    pub fn alloc(&mut self) -> Option<&mut LinkedBuffer<BUFFER_SIZE>> {
        let free_idx = self.free_head?;

        let next_free = self.buffers[free_idx].next;

        self.buffers[free_idx].clear();

        self.buffers[free_idx].prev = None;
        self.buffers[free_idx].next = self.used_head;

        if let Some(used_idx) = self.used_head {
            self.buffers[used_idx].prev = Some(free_idx);
        }

        self.used_head = Some(free_idx);
        self.free_head = next_free;

        Some(&mut self.buffers[free_idx])
    }

    /// 在指定节点后分配一个缓冲区
    ///
    /// # 参数
    ///
    /// * `prev_idx` - 前一个节点的索引
    ///
    /// # 返回值
    ///
    /// 成功返回缓冲区的可变引用，失败返回 `None`
    pub fn alloc_after(&mut self, prev_idx: usize) -> Option<&mut LinkedBuffer<BUFFER_SIZE>> {
        if prev_idx >= COUNT {
            return None;
        }

        let free_idx = self.free_head?;

        let next_free = self.buffers[free_idx].next;

        self.buffers[free_idx].clear();

        let next_used = self.buffers[prev_idx].next;

        self.buffers[free_idx].prev = Some(prev_idx);
        self.buffers[free_idx].next = next_used;

        self.buffers[prev_idx].next = Some(free_idx);

        if let Some(next_idx) = next_used {
            self.buffers[next_idx].prev = Some(free_idx);
        }

        self.free_head = next_free;

        Some(&mut self.buffers[free_idx])
    }

    /// 释放指定索引的缓冲区
    ///
    /// # 参数
    ///
    /// * `idx` - 要释放的缓冲区索引
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(())`，失败返回错误类型
    pub fn free(&mut self, idx: usize) -> Result<(), PoolError> {
        if idx >= COUNT {
            return Err(PoolError::InvalidIndex);
        }

        if self.free_head == Some(idx) {
            return Err(PoolError::AlreadyFreed);
        }

        let prev = self.buffers[idx].prev;
        let next = self.buffers[idx].next;

        if let Some(prev_idx) = prev {
            self.buffers[prev_idx].next = next;
        } else if self.used_head == Some(idx) {
            self.used_head = next;
        }

        if let Some(next_idx) = next {
            self.buffers[next_idx].prev = prev;
        }

        self.buffers[idx].clear();
        self.buffers[idx].next = self.free_head;
        self.buffers[idx].prev = None;

        self.free_head = Some(idx);

        Ok(())
    }

    /// 获取指定索引的缓冲区引用
    pub fn get(&self, idx: usize) -> Option<&LinkedBuffer<BUFFER_SIZE>> {
        if idx >= COUNT {
            return None;
        }
        Some(&self.buffers[idx])
    }

    /// 获取指定索引的缓冲区可变引用
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut LinkedBuffer<BUFFER_SIZE>> {
        if idx >= COUNT {
            return None;
        }
        Some(&mut self.buffers[idx])
    }

    /// 检查缓冲池是否已满
    pub fn is_full(&self) -> bool {
        self.free_head.is_none()
    }

    /// 检查缓冲池是否为空
    pub fn is_empty(&self) -> bool {
        self.used_head.is_none()
    }

    /// 获取空闲缓冲区数量
    pub fn free_count(&self) -> usize {
        let mut count = 0;
        let mut current = self.free_head;
        while let Some(idx) = current {
            count += 1;
            current = self.buffers[idx].next;
        }
        count
    }

    /// 获取已使用缓冲区数量
    pub fn used_count(&self) -> usize {
        let mut count = 0;
        let mut current = self.used_head;
        while let Some(idx) = current {
            count += 1;
            current = self.buffers[idx].next;
        }
        count
    }
}

impl<const BUFFER_SIZE: usize, const COUNT: usize> Default for BufferPool<BUFFER_SIZE, COUNT> {
    fn default() -> Self {
        let mut pool = Self::new();
        pool.init_free_list();
        pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_memory_basic() {
        let mut slab: SlabMemory<u32, 4> = SlabMemory::new();

        assert!(slab.is_empty());
        assert!(!slab.is_full());
        assert_eq!(slab.allocated_count(), 0);

        let ptr1_raw: *mut u32;
        {
            let ptr1 = slab.alloc().unwrap();
            *ptr1 = 100;
            assert_eq!(*ptr1, 100);
            ptr1_raw = ptr1 as *mut u32;
        }
        assert_eq!(slab.allocated_count(), 1);

        let ptr2_raw: *mut u32;
        {
            let ptr2 = slab.alloc().unwrap();
            *ptr2 = 200;
            ptr2_raw = ptr2 as *mut u32;
        }
        assert_eq!(slab.allocated_count(), 2);

        slab.free(ptr1_raw).unwrap();
        assert_eq!(slab.allocated_count(), 1);

        slab.free(ptr2_raw).unwrap();
        assert!(slab.is_empty());
    }

    #[test]
    fn test_slab_memory_full() {
        let mut slab: SlabMemory<u8, 2> = SlabMemory::new();

        let ptr1_raw: *mut u8;
        let ptr2_raw: *mut u8;
        {
            let ptr1 = slab.alloc().unwrap();
            ptr1_raw = ptr1 as *mut u8;
            let ptr2 = slab.alloc().unwrap();
            ptr2_raw = ptr2 as *mut u8;
        }

        assert!(slab.is_full());
        assert!(slab.alloc().is_none());

        slab.free(ptr1_raw).unwrap();
        assert!(!slab.is_full());

        let ptr3_raw: *mut u8;
        {
            let ptr3 = slab.alloc().unwrap();
            ptr3_raw = ptr3 as *mut u8;
        }
        assert!(slab.is_full());

        slab.free(ptr2_raw).unwrap();
        slab.free(ptr3_raw).unwrap();
    }

    #[test]
    fn test_slab_memory_invalid_free() {
        let mut slab: SlabMemory<u32, 4> = SlabMemory::new();

        let invalid_ptr = core::ptr::null_mut::<u32>();
        assert_eq!(slab.free(invalid_ptr), Err(PoolError::InvalidIndex));

        let ptr = slab.alloc().unwrap();
        let ptr_raw = ptr as *mut u32;
        slab.free(ptr_raw).unwrap();

        let outside_ptr = 0xDEADBEEF as *mut u32;
        assert_eq!(slab.free(outside_ptr), Err(PoolError::InvalidIndex));
    }

    #[test]
    fn test_buffer_index() {
        let idx = BufferIndex {
            frame_idx: 10,
            invoke_idx: 20,
        };

        assert_eq!(idx.frame_idx, 10);
        assert_eq!(idx.invoke_idx, 20);

        let default_idx = BufferIndex::default();
        assert_eq!(default_idx.frame_idx, 0);
        assert_eq!(default_idx.invoke_idx, 0);
    }

    #[test]
    fn test_linked_buffer() {
        let mut buffer: LinkedBuffer<64> = LinkedBuffer::new();

        assert_eq!(buffer.length, 0);
        assert!(buffer.next.is_none());
        assert!(buffer.prev.is_none());

        buffer.buff[0] = 0x01;
        buffer.buff[1] = 0x02;
        buffer.length = 2;
        buffer.next = Some(1);

        assert_eq!(buffer.buff[0], 0x01);
        assert_eq!(buffer.buff[1], 0x02);
        assert_eq!(buffer.length, 2);
        assert_eq!(buffer.next, Some(1));

        buffer.clear();
        assert_eq!(buffer.length, 0);
        assert!(buffer.next.is_none());
        assert!(buffer.prev.is_none());
    }

    #[test]
    fn test_buffer_pool_basic() {
        let mut pool: BufferPool<64, 4> = BufferPool::default();

        assert!(pool.is_empty());
        assert!(!pool.is_full());
        assert_eq!(pool.free_count(), 4);
        assert_eq!(pool.used_count(), 0);

        {
            let buf1 = pool.alloc().unwrap();
            buf1.length = 10;
            buf1.index.frame_idx = 1;
        }

        assert!(!pool.is_empty());
        assert_eq!(pool.used_count(), 1);
        assert_eq!(pool.free_count(), 3);

        {
            let buf2 = pool.alloc().unwrap();
            buf2.length = 20;
        }

        assert_eq!(pool.used_count(), 2);
        assert_eq!(pool.free_count(), 2);
    }

    #[test]
    fn test_buffer_pool_free() {
        let mut pool: BufferPool<64, 4> = BufferPool::default();

        let idx1: usize;
        {
            let _buf1 = pool.alloc().unwrap();
            idx1 = pool.used_head.unwrap();
        }

        pool.free(idx1).unwrap();

        assert!(pool.is_empty());
        assert_eq!(pool.free_count(), 4);

        assert_eq!(pool.free(idx1), Err(PoolError::AlreadyFreed));
        assert_eq!(pool.free(100), Err(PoolError::InvalidIndex));
    }

    #[test]
    fn test_buffer_pool_alloc_after() {
        let mut pool: BufferPool<64, 4> = BufferPool::default();

        let idx1: usize;
        {
            let buf1 = pool.alloc().unwrap();
            buf1.index.frame_idx = 1;
            idx1 = pool.used_head.unwrap();
        }

        let idx2: usize;
        {
            let buf2 = pool.alloc_after(idx1).unwrap();
            buf2.index.frame_idx = 2;
            idx2 = pool.buffers[idx1].next.unwrap();
        }

        assert_eq!(pool.buffers[idx1].next, Some(idx2));
        assert_eq!(pool.buffers[idx2].prev, Some(idx1));

        let idx3: usize;
        {
            let _buf3 = pool.alloc_after(idx1).unwrap();
            idx3 = pool.buffers[idx1].next.unwrap();
        }

        assert_eq!(pool.buffers[idx1].next, Some(idx3));
        assert_eq!(pool.buffers[idx3].prev, Some(idx1));
        assert_eq!(pool.buffers[idx3].next, Some(idx2));
        assert_eq!(pool.buffers[idx2].prev, Some(idx3));
    }

    #[test]
    fn test_buffer_pool_get() {
        let mut pool: BufferPool<64, 4> = BufferPool::default();

        let idx: usize;
        {
            let buf = pool.alloc().unwrap();
            buf.length = 42;
            buf.index.frame_idx = 5;
            idx = pool.used_head.unwrap();
        }

        let retrieved = pool.get(idx).unwrap();
        assert_eq!(retrieved.length, 42);
        assert_eq!(retrieved.index.frame_idx, 5);

        let retrieved_mut = pool.get_mut(idx).unwrap();
        retrieved_mut.length = 100;

        assert_eq!(pool.get(idx).unwrap().length, 100);

        assert!(pool.get(100).is_none());
        assert!(pool.get_mut(100).is_none());
    }

    #[test]
    fn test_buffer_pool_full() {
        let mut pool: BufferPool<64, 2> = BufferPool::default();

        {
            let _ = pool.alloc().unwrap();
            let _ = pool.alloc().unwrap();
        }

        assert!(pool.is_full());
        assert!(pool.alloc().is_none());

        pool.free(0).unwrap();
        assert!(!pool.is_full());

        {
            let _ = pool.alloc().unwrap();
        }
        assert!(pool.is_full());
    }

    #[test]
    fn test_buffer_pool_chain_operations() {
        let mut pool: BufferPool<64, 8> = BufferPool::default();

        let idx1: usize;
        {
            let buf1 = pool.alloc().unwrap();
            buf1.buff[0] = 0xAA;
        }
        idx1 = pool.used_head.unwrap();

        let idx2: usize;
        {
            let buf2 = pool.alloc_after(idx1).unwrap();
            buf2.buff[0] = 0xBB;
        }
        idx2 = pool.buffers[idx1].next.unwrap();

        let idx3: usize;
        {
            let buf3 = pool.alloc_after(idx2).unwrap();
            buf3.buff[0] = 0xCC;
        }
        idx3 = pool.buffers[idx2].next.unwrap();

        assert_eq!(pool.used_count(), 3);

        pool.free(idx2).unwrap();

        assert_eq!(pool.buffers[idx1].next, Some(idx3));
        assert_eq!(pool.buffers[idx3].prev, Some(idx1));

        assert_eq!(pool.used_count(), 2);
        assert_eq!(pool.free_count(), 6);

        {
            let _buf4 = pool.alloc().unwrap();
        }
        assert_eq!(pool.used_count(), 3);
    }
}
