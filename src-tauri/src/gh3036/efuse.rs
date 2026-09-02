use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

use super::manager::Gh3036Manager;

#[async_trait]
pub trait RegisterAccess: Send + Sync {
    async fn read_u16(&self, address: u16, count: i32) -> Result<Vec<u16>, String>;
    async fn write_u16(&self, pairs: &[(u16, u16)]) -> Result<(), String>;
}

pub struct Gh3036EfuseReader;

impl Gh3036EfuseReader {
    const EFUSE_MODE_ADDR: u16 = 0x0580;
    const EFUSE_RDEN_ADDR: u16 = 0x0584;
    const EFUSE_START_ADDR: u16 = 0x058A;
    const EFUSE_DONE_ADDR: u16 = 0x05A6;
    const EFUSE_RDATA_ADDR: u16 = 0x059E;
    const EFUSE_MODE_SEL_KEEP_MASK: u16 = 0xFFF2;
    const EFUSE_DONE_POLL_INTERVAL_MS: u64 = 50;
    const EFUSE_DONE_TIMEOUT_MS: u64 = 200;

    pub async fn read_segment<A: RegisterAccess + ?Sized>(
        io: &A,
        segment: u8,
    ) -> Result<u64, String> {
        if segment > 3 {
            return Err(format!("invalid efuse segment: {}", segment));
        }

        let original_mode = Self::read_single(io, Self::EFUSE_MODE_ADDR).await?;
        let selected_mode =
            (original_mode & Self::EFUSE_MODE_SEL_KEEP_MASK) | ((segment as u16) << 2);
        Self::write_single(io, Self::EFUSE_MODE_ADDR, selected_mode).await?;

        let original_rden = Self::read_single(io, Self::EFUSE_RDEN_ADDR).await?;
        if let Err(error) = Self::write_single(io, Self::EFUSE_RDEN_ADDR, original_rden | 1).await
        {
            let _ = Self::restore_rden(io, original_rden).await;
            return Err(error);
        }

        let original_start = Self::read_single(io, Self::EFUSE_START_ADDR).await?;
        if let Err(error) = Self::write_single(io, Self::EFUSE_START_ADDR, original_start | 1).await
        {
            let _ = Self::restore_rden(io, original_rden).await;
            return Err(error);
        }

        let mut done = false;
        let mut elapsed_ms = 0;

        while elapsed_ms < Self::EFUSE_DONE_TIMEOUT_MS {
            sleep(Duration::from_millis(Self::EFUSE_DONE_POLL_INTERVAL_MS)).await;
            elapsed_ms += Self::EFUSE_DONE_POLL_INTERVAL_MS;

            let done_reg = match Self::read_single(io, Self::EFUSE_DONE_ADDR).await {
                Ok(value) => value,
                Err(error) => {
                    let _ = Self::restore_rden(io, original_rden).await;
                    return Err(error);
                }
            };

            if done_reg & 1 != 0 {
                done = true;
                break;
            }
        }

        if !done {
            let _ = Self::restore_rden(io, original_rden).await;
            return Err("efuse read timed out".to_string());
        }

        let values = match io.read_u16(Self::EFUSE_RDATA_ADDR, 4).await {
            Ok(values) => values,
            Err(error) => {
                let _ = Self::restore_rden(io, original_rden).await;
                return Err(error);
            }
        };

        if values.len() < 4 {
            let _ = Self::restore_rden(io, original_rden).await;
            return Err(format!(
                "insufficient efuse rdata words: expected 4, got {}",
                values.len()
            ));
        }

        Self::restore_rden(io, original_rden).await?;

        Ok((values[0] as u64)
            | ((values[1] as u64) << 16)
            | ((values[2] as u64) << 32)
            | ((values[3] as u64) << 48))
    }

    pub async fn read_all<A: RegisterAccess + ?Sized>(io: &A) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::with_capacity(32);
        for segment in 0..4 {
            let value = Self::read_segment(io, segment).await?;
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
    }

    async fn read_single<A: RegisterAccess + ?Sized>(
        io: &A,
        address: u16,
    ) -> Result<u16, String> {
        let values = io.read_u16(address, 1).await?;
        values
            .into_iter()
            .next()
            .ok_or_else(|| format!("missing register value at 0x{address:04X}"))
    }

    async fn write_single<A: RegisterAccess + ?Sized>(
        io: &A,
        address: u16,
        value: u16,
    ) -> Result<(), String> {
        io.write_u16(&[(address, value)]).await
    }

    async fn restore_rden<A: RegisterAccess + ?Sized>(io: &A, original_rden: u16) -> Result<(), String> {
        io.write_u16(&[(Self::EFUSE_RDEN_ADDR, original_rden)]).await
    }
}

#[async_trait]
impl RegisterAccess for Gh3036Manager {
    async fn read_u16(&self, address: u16, count: i32) -> Result<Vec<u16>, String> {
        self.read_registers_u16(address, count).await
    }

    async fn write_u16(&self, pairs: &[(u16, u16)]) -> Result<(), String> {
        self.write_registers_u16(pairs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    struct FakeRegisterAccess {
        reads: Mutex<Vec<(u16, Vec<u16>)>>,
        read_log: Mutex<Vec<(u16, i32)>>,
        writes: Mutex<Vec<(u16, u16)>>,
    }

    impl FakeRegisterAccess {
        fn with_reads<I>(reads: I) -> Arc<Self>
        where
            I: IntoIterator<Item = (u16, Vec<u16>)>,
        {
            Arc::new(Self {
                reads: Mutex::new(reads.into_iter().collect()),
                read_log: Mutex::new(Vec::new()),
                writes: Mutex::new(Vec::new()),
            })
        }

        fn writes(&self) -> Vec<(u16, u16)> {
            self.writes.lock().unwrap().clone()
        }

    }

    #[async_trait]
    impl RegisterAccess for FakeRegisterAccess {
        async fn read_u16(&self, address: u16, count: i32) -> Result<Vec<u16>, String> {
            self.read_log.lock().unwrap().push((address, count));

            let mut reads = self.reads.lock().unwrap();
            if let Some((expected_address, values)) = reads.first().cloned() {
                if expected_address == address && count >= values.len() as i32 {
                    reads.remove(0);
                    return Ok(values);
                }
            }

            Err(format!("unexpected read: 0x{address:04X} count={count}"))
        }

        async fn write_u16(&self, pairs: &[(u16, u16)]) -> Result<(), String> {
            self.writes.lock().unwrap().extend_from_slice(pairs);
            Ok(())
        }
    }

    fn segment_reads(segment_values: &[(u16, [u16; 4])]) -> Vec<(u16, Vec<u16>)> {
        let mut reads = Vec::new();
        for (_segment, values) in segment_values {
            reads.push((Gh3036EfuseReader::EFUSE_MODE_ADDR, vec![0x0000]));
            reads.push((Gh3036EfuseReader::EFUSE_RDEN_ADDR, vec![0x0000]));
            reads.push((Gh3036EfuseReader::EFUSE_START_ADDR, vec![0x0000]));
            reads.push((Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![1]));
            reads.push((Gh3036EfuseReader::EFUSE_RDATA_ADDR, values.to_vec()));
        }
        reads
    }

    #[tokio::test]
    async fn segment_two_uses_sdk_register_sequence_and_packs_little_endian() {
        let io = FakeRegisterAccess::with_reads([
            (Gh3036EfuseReader::EFUSE_MODE_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_RDEN_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_START_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![1]),
            (
                Gh3036EfuseReader::EFUSE_RDATA_ADDR,
                vec![0x1111, 0x2222, 0x3333, 0x4444],
            ),
        ]);

        let value = Gh3036EfuseReader::read_segment(&*io, 2).await.unwrap();
        assert_eq!(value, 0x4444_3333_2222_1111);
        assert_eq!(
            io.writes(),
            vec![
                (0x0580, 0x0008),
                (0x0584, 0x0001),
                (0x058A, 0x0001),
                (0x0584, 0x0000),
            ]
        );
    }

    #[tokio::test]
    async fn delayed_done_polling_waits_for_ready_bit_before_reading_rdata() {
        let io = FakeRegisterAccess::with_reads([
            (Gh3036EfuseReader::EFUSE_MODE_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_RDEN_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_START_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![0]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![1]),
            (
                Gh3036EfuseReader::EFUSE_RDATA_ADDR,
                vec![0x1111, 0x2222, 0x3333, 0x4444],
            ),
        ]);

        let started = Instant::now();
        let task = tokio::spawn(async move { Gh3036EfuseReader::read_segment(&*io, 1).await });
        let value = task.await.unwrap().unwrap();

        assert_eq!(value, 0x4444_3333_2222_1111);
        assert!(started.elapsed() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn timeout_restores_original_rden_best_effort() {
        let io = FakeRegisterAccess::with_reads([
            (Gh3036EfuseReader::EFUSE_MODE_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_RDEN_ADDR, vec![0x0010]),
            (Gh3036EfuseReader::EFUSE_START_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![0]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![0]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![0]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![0]),
        ]);

        let started = Instant::now();
        let error = Gh3036EfuseReader::read_segment(&*io, 0).await.unwrap_err();

        assert!(error.contains("timed out"));
        assert!(started.elapsed() >= Duration::from_millis(200));
    }

    #[tokio::test]
    async fn read_all_returns_32_bytes_in_segment_order() {
        let io = FakeRegisterAccess::with_reads(segment_reads(&[
            (0, [0x1111, 0x2222, 0x3333, 0x4444]),
            (1, [0x5555, 0x6666, 0x7777, 0x8888]),
            (2, [0x9999, 0xAAAA, 0xBBBB, 0xCCCC]),
            (3, [0xDDDD, 0xEEEE, 0xFFFF, 0x0001]),
        ]));

        let bytes = Gh3036EfuseReader::read_all(&*io).await.unwrap();

        assert_eq!(
            bytes,
            vec![
                0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44, 0x55, 0x55, 0x66, 0x66, 0x77,
                0x77, 0x88, 0x88, 0x99, 0x99, 0xAA, 0xAA, 0xBB, 0xBB, 0xCC, 0xCC, 0xDD, 0xDD,
                0xEE, 0xEE, 0xFF, 0xFF, 0x01, 0x00,
            ]
        );
    }

    #[tokio::test]
    async fn read_segment_rejects_invalid_segment() {
        let io = FakeRegisterAccess::with_reads([]);
        let error = Gh3036EfuseReader::read_segment(&*io, 4).await.unwrap_err();
        assert!(error.contains("invalid efuse segment"));
    }

    #[tokio::test]
    async fn insufficient_rdata_is_rejected_and_rden_is_restored() {
        let io = FakeRegisterAccess::with_reads([
            (Gh3036EfuseReader::EFUSE_MODE_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_RDEN_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_START_ADDR, vec![0x0000]),
            (Gh3036EfuseReader::EFUSE_DONE_ADDR, vec![1]),
            (Gh3036EfuseReader::EFUSE_RDATA_ADDR, vec![0x1111, 0x2222, 0x3333]),
        ]);

        let error = Gh3036EfuseReader::read_segment(&*io, 3).await.unwrap_err();

        assert!(error.contains("insufficient efuse rdata words"));
        assert_eq!(
            io.writes(),
            vec![
                (0x0580, 0x000C),
                (0x0584, 0x0001),
                (0x058A, 0x0001),
                (0x0584, 0x0000),
            ]
        );
    }
}
