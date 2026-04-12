use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use crate::error::{ComBridgeError, Result};

const DEFAULT_CAPACITY: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub timestamp: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheData {
    pub entries: Vec<CacheEntry>,
    pub total_bytes: usize,
    pub entry_count: usize,
}

pub struct RingBuffer {
    buffer: Vec<u8>,
    capacity: usize,
    head: usize,
    tail: usize,
    entries: Vec<CacheEntry>,
}

impl RingBuffer {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity],
            capacity,
            head: 0,
            tail: 0,
            entries: Vec::new(),
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let data_len = data.len();
        
        if data_len == 0 {
            return;
        }

        if data_len >= self.capacity {
            self.head = 0;
            self.tail = self.capacity;
            self.buffer[..self.capacity].copy_from_slice(&data[data_len - self.capacity..]);
        } else {
            let space_to_end = self.capacity - self.tail;
            
            if data_len <= space_to_end {
                self.buffer[self.tail..self.tail + data_len].copy_from_slice(data);
                self.tail += data_len;
            } else {
                self.buffer[self.tail..].copy_from_slice(&data[..space_to_end]);
                self.buffer[..data_len - space_to_end].copy_from_slice(&data[space_to_end..]);
                self.tail = data_len - space_to_end;
            }

            if self.tail > self.head || (self.tail <= self.head && data_len > 0) {
                if self.tail <= self.head && self.head < self.capacity {
                    self.head = (self.head + data_len) % self.capacity;
                    if self.head == self.tail {
                        self.head = (self.head + 1) % self.capacity;
                    }
                }
            }
        }

        self.entries.push(CacheEntry {
            timestamp,
            data: data.to_vec(),
        });

        while self.calculate_total_bytes() > self.capacity {
            self.entries.remove(0);
        }
    }

    fn calculate_total_bytes(&self) -> usize {
        self.entries.iter().map(|e| e.data.len()).sum()
    }

    pub fn read_all(&self) -> Vec<u8> {
        if self.head == self.tail {
            return Vec::new();
        }

        if self.tail > self.head {
            self.buffer[self.head..self.tail].to_vec()
        } else {
            let mut result = Vec::with_capacity(self.capacity);
            result.extend_from_slice(&self.buffer[self.head..]);
            result.extend_from_slice(&self.buffer[..self.tail]);
            result
        }
    }

    pub fn get_entries(&self) -> &[CacheEntry] {
        &self.entries
    }

    pub fn get_cache_data(&self) -> CacheData {
        CacheData {
            entries: self.entries.clone(),
            total_bytes: self.calculate_total_bytes(),
            entry_count: self.entries.len(),
        }
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        if self.tail >= self.head {
            self.tail - self.head
        } else {
            self.capacity - self.head + self.tail
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCache {
    pub tx_cache: CacheData,
    pub rx_cache: CacheData,
}

pub struct ThreadSafeRingBuffer {
    inner: Mutex<RingBuffer>,
}

impl ThreadSafeRingBuffer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RingBuffer::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(RingBuffer::with_capacity(capacity)),
        }
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut buffer = self.inner.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        buffer.write(data);
        Ok(())
    }

    pub fn read_all(&self) -> Result<Vec<u8>> {
        let buffer = self.inner.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        Ok(buffer.read_all())
    }

    pub fn get_cache_data(&self) -> Result<CacheData> {
        let buffer = self.inner.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        Ok(buffer.get_cache_data())
    }

    pub fn clear(&self) -> Result<()> {
        let mut buffer = self.inner.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        buffer.clear();
        Ok(())
    }

    pub fn len(&self) -> Result<usize> {
        let buffer = self.inner.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        Ok(buffer.len())
    }

    pub fn is_empty(&self) -> Result<bool> {
        let buffer = self.inner.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        Ok(buffer.is_empty())
    }
}

impl Default for ThreadSafeRingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

pub type RingBufferRef = Arc<ThreadSafeRingBuffer>;

pub fn create_ring_buffer() -> RingBufferRef {
    Arc::new(ThreadSafeRingBuffer::new())
}

pub fn create_ring_buffer_with_capacity(capacity: usize) -> RingBufferRef {
    Arc::new(ThreadSafeRingBuffer::with_capacity(capacity))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let mut buffer = RingBuffer::with_capacity(10);
        
        buffer.write(&[1, 2, 3]);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.read_all(), vec![1, 2, 3]);
        
        buffer.write(&[4, 5]);
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.read_all(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = RingBuffer::with_capacity(5);
        
        buffer.write(&[1, 2, 3, 4, 5]);
        assert_eq!(buffer.len(), 5);
        
        buffer.write(&[6, 7]);
        let data = buffer.read_all();
        assert!(data.contains(&6));
        assert!(data.contains(&7));
    }

    #[test]
    fn test_ring_buffer_clear() {
        let mut buffer = RingBuffer::new();
        
        buffer.write(&[1, 2, 3]);
        assert!(!buffer.is_empty());
        
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_ring_buffer_entries() {
        let mut buffer = RingBuffer::new();
        
        buffer.write(&[1, 2, 3]);
        buffer.write(&[4, 5]);
        
        let entries = buffer.get_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].data, vec![1, 2, 3]);
        assert_eq!(entries[1].data, vec![4, 5]);
    }

    #[test]
    fn test_thread_safe_ring_buffer() {
        let buffer = create_ring_buffer_with_capacity(100);
        
        buffer.write(&[1, 2, 3]).unwrap();
        buffer.write(&[4, 5]).unwrap();
        
        assert_eq!(buffer.len().unwrap(), 5);
        assert_eq!(buffer.read_all().unwrap(), vec![1, 2, 3, 4, 5]);
        
        buffer.clear().unwrap();
        assert!(buffer.is_empty().unwrap());
    }

    #[test]
    fn test_cache_data() {
        let mut buffer = RingBuffer::new();
        
        buffer.write(&[1, 2, 3]);
        buffer.write(&[4, 5]);
        
        let cache_data = buffer.get_cache_data();
        assert_eq!(cache_data.entry_count, 2);
        assert_eq!(cache_data.total_bytes, 5);
    }
}
