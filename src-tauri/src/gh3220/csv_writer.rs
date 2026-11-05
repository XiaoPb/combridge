//! GH3220 CSV 落盘。
//!
//! 按 function_id 分文件：不同功能的通道数不同，混写会导致列错位。
//! 首条记录确定列宽；后续记录变宽时新建 `_partN`，不回改表头。

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::types::Gh3220Record;

pub struct CsvWriter {
    output_dir: PathBuf,
    function_id: u8,
    part: u32,
    path: PathBuf,
    file: Option<BufWriter<File>>,
    raw_w: usize,
    agc_w: usize,
    timestamp: String,
    idle_close: Option<Duration>,
    last_write: Instant,
}

impl CsvWriter {
    pub fn new(output_dir: &Path, function_id: u8, idle_close_secs: u64) -> Self {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let idle_close = if idle_close_secs == 0 {
            None
        } else {
            Some(Duration::from_secs(idle_close_secs))
        };
        let path = Self::build_path(output_dir, function_id, &timestamp, 0);

        Self {
            output_dir: output_dir.to_path_buf(),
            function_id,
            part: 0,
            path,
            file: None,
            raw_w: 0,
            agc_w: 0,
            timestamp,
            idle_close,
            last_write: Instant::now(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write(&mut self, rec: &Gh3220Record) -> io::Result<()> {
        if rec.raw.len() > self.raw_w || rec.agc.len() > self.agc_w {
            let first_record = self.raw_w == 0 && self.agc_w == 0;
            self.raw_w = self.raw_w.max(rec.raw.len());
            self.agc_w = self.agc_w.max(rec.agc.len());
            if !first_record {
                self.rotate()?;
            }
        }

        if let Some(idle) = self.idle_close {
            if self.file.is_some() && self.last_write.elapsed() > idle {
                self.close_handle();
            }
        }

        self.ensure_open()?;
        let line = self.line(rec);
        if let Some(file) = self.file.as_mut() {
            file.write_all(line.as_bytes())?;
        }
        self.last_write = Instant::now();
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = self.file.as_mut() {
            file.flush()?;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        self.close_handle();
    }

    fn close_handle(&mut self) {
        if let Some(mut file) = self.file.take() {
            let _ = file.flush();
        }
    }

    fn ensure_open(&mut self) -> io::Result<()> {
        if self.file.is_some() {
            return Ok(());
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        let existed = self.path.exists()
            && std::fs::metadata(&self.path)
                .map(|m| m.len() > 0)
                .unwrap_or(false);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let mut writer = BufWriter::new(file);
        if !existed {
            writer.write_all(self.header().as_bytes())?;
        }
        self.file = Some(writer);
        Ok(())
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.close_handle();
        self.part += 1;
        self.path = Self::build_path(
            &self.output_dir,
            self.function_id,
            &self.timestamp,
            self.part,
        );
        self.ensure_open()
    }

    fn build_path(output_dir: &Path, function_id: u8, timestamp: &str, part: u32) -> PathBuf {
        let suffix = if part == 0 {
            String::new()
        } else {
            format!("_part{part}")
        };
        output_dir.join(format!("gh3220_func{function_id}_{timestamp}{suffix}.csv"))
    }

    fn header(&self) -> String {
        let mut h = String::from("func_id,frame_id");
        for i in 0..self.raw_w {
            h.push_str(&format!(",raw{i}"));
        }
        for i in 0..self.agc_w {
            h.push_str(&format!(",agc{i}"));
        }
        h.push_str(",gs_x,gs_y,gs_z,ts_ms,algo");
        h.push_str(",cap0,cap1,cap2,cap3,temp0,temp1,temp2,temp3");
        h.push_str(",flag2,flag3,flag4,slot_cfg\n");
        h
    }

    fn line(&self, rec: &Gh3220Record) -> String {
        let mut line = format!("{},{}", rec.function_id, rec.frame_id);

        for i in 0..self.raw_w {
            match rec.raw.get(i) {
                Some(v) => line.push_str(&format!(",{}", v & 0x00FF_FFFF)),
                None => line.push(','),
            }
        }
        for i in 0..self.agc_w {
            match rec.agc.get(i) {
                Some(v) => line.push_str(&format!(",{v}")),
                None => line.push(','),
            }
        }

        let gs = rec.gs.unwrap_or([0, 0, 0]);
        line.push_str(&format!(",{},{},{}", gs[0], gs[1], gs[2]));
        line.push_str(&format!(",{}", rec.ts_ms));
        line.push(',');
        line.push_str(&algo_str(&rec.algo));

        let cap = rec.cap.unwrap_or([0; 4]);
        for v in cap {
            line.push_str(&format!(",{v}"));
        }
        let temp = rec.temp.unwrap_or([0; 4]);
        for v in temp {
            line.push_str(&format!(",{v}"));
        }
        for v in &rec.flags[2..5] {
            line.push_str(&format!(",{v}"));
        }
        line.push(',');
        line.push('\n');
        line
    }
}

fn algo_str(algo: &[(u8, i32)]) -> String {
    let mut s = String::new();
    for (i, v) in algo {
        if !s.is_empty() {
            s.push(';');
        }
        s.push_str(&format!("{i}:{v}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(raw: Vec<u32>, agc: Vec<u32>) -> Gh3220Record {
        Gh3220Record {
            function_id: 2,
            frame_id: 7,
            raw,
            agc,
            gs: Some([1, 2, 3]),
            gyro: None,
            cap: None,
            temp: None,
            flags: [0; 5],
            algo: vec![(1, 42)],
            ts_ms: 1234,
        }
    }

    #[test]
    fn writes_header_and_rows() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = CsvWriter::new(dir.path(), 2, 0);
        w.write(&record(vec![1, 2], vec![3, 4])).unwrap();
        w.flush().unwrap();
        w.close();

        let text = std::fs::read_to_string(w.path()).unwrap();
        assert!(text.starts_with("func_id,frame_id,raw0,raw1,agc0,agc1"));
        assert!(text.contains("2,7,1,2,3,4,1,2,3,1234,1:42"));
    }

    #[test]
    fn widens_into_new_part() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = CsvWriter::new(dir.path(), 2, 0);
        w.write(&record(vec![1], vec![1])).unwrap();
        let first = w.path().to_path_buf();
        w.write(&record(vec![1, 2, 3], vec![1, 2, 3])).unwrap();
        w.flush().unwrap();
        w.close();

        let second = w.path().to_path_buf();
        assert_ne!(first, second);
        assert!(second
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("_part1"));
        let text = std::fs::read_to_string(&second).unwrap();
        assert!(text.starts_with("func_id,frame_id,raw0,raw1,raw2,agc0,agc1,agc2"));
    }
}
