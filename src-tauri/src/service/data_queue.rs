use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueConfig {
    pub capacity: usize,
    pub overflow_policy: OverflowPolicy,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum OverflowPolicy {
    Block,
    DropNewest,
    DropOldest,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            capacity: 1024,
            overflow_policy: OverflowPolicy::Block,
        }
    }
}

#[derive(Debug)]
pub struct QueueStats {
    pub len: usize,
    pub capacity: usize,
    pub is_empty: bool,
    pub is_full: bool,
}

pub struct DataQueue<T> {
    sender: Sender<T>,
    receiver: Arc<tokio::sync::Mutex<Receiver<T>>>,
    config: QueueConfig,
}

impl<T: Clone + Send + 'static> DataQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self::with_config(QueueConfig {
            capacity,
            ..Default::default()
        })
    }

    pub fn with_config(config: QueueConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.capacity);
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
            config,
        }
    }

    pub async fn send(&self, value: T) -> Result<(), mpsc::error::SendError<T>> {
        match self.config.overflow_policy {
            OverflowPolicy::Block => self.sender.send(value).await,
            OverflowPolicy::DropNewest => {
                if self.sender.capacity() == 0 {
                    tracing::warn!("Queue full, dropping newest item");
                    Ok(())
                } else {
                    self.sender.send(value).await
                }
            }
            OverflowPolicy::DropOldest => {
                if self.sender.is_closed() {
                    return Err(mpsc::error::SendError(value));
                }
                let mut rx = self.receiver.lock().await;
                if self.sender.capacity() == 0 {
                    let _ = rx.try_recv();
                }
                drop(rx);
                self.sender.send(value).await
            }
        }
    }

    pub fn try_send(&self, value: T) -> Result<(), mpsc::error::TrySendError<T>> {
        match self.config.overflow_policy {
            OverflowPolicy::Block => self.sender.try_send(value),
            OverflowPolicy::DropNewest => {
                if self.sender.capacity() == 0 {
                    tracing::warn!("Queue full, dropping newest item");
                    Ok(())
                } else {
                    self.sender.try_send(value)
                }
            }
            OverflowPolicy::DropOldest => {
                if self.sender.is_closed() {
                    return Err(mpsc::error::TrySendError::Closed(value));
                }
                match self.receiver.try_lock() {
                    Ok(mut rx) => {
                        if self.sender.capacity() == 0 {
                            let _ = rx.try_recv();
                        }
                        drop(rx);
                        self.sender.try_send(value)
                    }
                    Err(_) => self.sender.try_send(value),
                }
            }
        }
    }

    pub async fn recv(&self) -> Option<T> {
        let mut rx = self.receiver.lock().await;
        rx.recv().await
    }

    pub async fn recv_many(&self, buffer: &mut Vec<T>, limit: usize) -> usize {
        let mut rx = self.receiver.lock().await;
        let mut count = 0;
        while count < limit {
            match rx.try_recv() {
                Ok(item) => {
                    buffer.push(item);
                    count += 1;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => break,
            }
        }
        count
    }

    pub fn sender(&self) -> Sender<T> {
        self.sender.clone()
    }

    pub fn capacity(&self) -> usize {
        self.sender.capacity()
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    pub async fn stats(&self) -> QueueStats {
        let rx = self.receiver.lock().await;
        QueueStats {
            len: rx.len(),
            capacity: self.config.capacity,
            is_empty: rx.is_empty(),
            is_full: self.sender.capacity() == 0,
        }
    }

    pub async fn close(&self) {
        let mut rx = self.receiver.lock().await;
        rx.close();
    }
}

impl<T: Clone + Send + 'static> Default for DataQueue<T> {
    fn default() -> Self {
        Self::new(1024)
    }
}
