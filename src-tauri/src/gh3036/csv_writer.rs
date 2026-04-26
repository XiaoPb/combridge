use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use csv::Writer;
use tracing::info;

use super::types::{Gh3036FrameData, REF_DATA_COUNT};

pub struct CsvWriter {
    writer: Mutex<Option<Writer<std::fs::File>>>,
    output_dir: PathBuf,
    function_name: String,
    current_file_index: u32,
    last_frame_id: i32,
}

impl CsvWriter {
    pub fn new(output_dir: PathBuf, _function_id: i32, function_name: String) -> Self {
        Self {
            writer: Mutex::new(None),
            output_dir,
            function_name,
            current_file_index: 0,
            last_frame_id: -1,
        }
    }

    pub fn write_frame(&mut self, frame: &Gh3036FrameData) -> std::io::Result<()> {
        let should_create_new_file = frame.frame_id == 0 || self.writer.lock().unwrap_or_else(|e| e.into_inner()).is_none();

        if should_create_new_file {
            self.create_new_file()?;
        }

        let mut writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut writer) = *writer_guard {
            self.write_row(writer, frame)?;
            self.last_frame_id = frame.frame_id;
        }

        Ok(())
    }

    fn create_new_file(&mut self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.output_dir)?;

        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!(
            "gh3036_{}_{}.csv",
            self.function_name,
            timestamp
        );
        let filepath = self.output_dir.join(&filename);

        let mut writer = Writer::from_path(&filepath)?;

        self.write_header(&mut writer)?;

        info!("创建新的 CSV 文件: {:?}", filepath);

        let mut writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        *writer_guard = Some(writer);
        self.current_file_index += 1;

        Ok(())
    }

    fn write_header(&self, writer: &mut Writer<std::fs::File>) -> std::io::Result<()> {
        let mut headers: Vec<String> = vec![
            "TIMESTAMP".to_string(),
            "FRAME_ID".to_string(),
        ];

        let axis_names = ["X", "Y", "Z"];
        for axis in axis_names {
            headers.push(format!("ACC_{}", axis));
        }

        for i in 0..16 {
            headers.push(format!("CH{}", i));
        }

        for i in 0..9 {
            headers.push(format!("FLAG{}", i));
        }

        for i in 0..REF_DATA_COUNT {
            headers.push(format!("REF_RESULT_{}", i));
        }

        for i in 0..9 {
            headers.push(format!("ALGO_RESULT{}", i));
        }

        for i in 0..16 {
            headers.push(format!("AGC_INFO{}", i));
        }

        for i in 0..16 {
            headers.push(format!("PHY_VALUE{}", i));
        }

        for i in 16..32 {
            headers.push(format!("CH{}", i));
        }

        for i in 16..32 {
            headers.push(format!("AGC_INFO{}", i));
        }

        for i in 16..32 {
            headers.push(format!("PHY_VALUE{}", i));
        }

        for axis in axis_names {
            headers.push(format!("GYRO_{}", axis));
        }

        writer.write_record(&headers)?;
        Ok(())
    }

    fn write_row(&self, writer: &mut Writer<std::fs::File>, frame: &Gh3036FrameData) -> std::io::Result<()> {
        let mut row: Vec<String> = Vec::new();

        row.push(frame.timestamp.to_string());
        row.push(frame.frame_id.to_string());

        for i in 0..3 {
            let val = frame.gs_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..16 {
            let val = frame.rawdata.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..9 {
            let val = frame.flags.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..REF_DATA_COUNT {
            let val = frame.ref_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..9 {
            let val = frame.algo_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..16 {
            let val = frame.agc_info.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..16 {
            let val = frame.phy_value.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 16..32 {
            let val = frame.rawdata.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 16..32 {
            let val = frame.agc_info.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 16..32 {
            let val = frame.phy_value.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 3..6 {
            let val = frame.gs_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        writer.write_record(&row)?;
        writer.flush()?;

        Ok(())
    }
}
