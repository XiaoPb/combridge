use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use csv::Writer;
use tracing::info;

use gh_rpc::types::GhFuncFrame;

pub struct CsvWriter {
    writer: Mutex<Option<Writer<std::fs::File>>>,
    output_dir: PathBuf,
    function_name: String,
    current_file_index: u32,
    last_frame_cnt: u32,
}

impl CsvWriter {
    pub fn new(output_dir: PathBuf, _function_id: i32, function_name: String) -> Self {
        Self {
            writer: Mutex::new(None),
            output_dir,
            function_name,
            current_file_index: 0,
            last_frame_cnt: 0,
        }
    }

    pub fn write_frame(&mut self, frame: &GhFuncFrame) -> std::io::Result<()> {
        let should_create_new_file = frame.frame_cnt == 0 || self.writer.lock().unwrap_or_else(|e| e.into_inner()).is_none();

        if should_create_new_file {
            self.create_new_file()?;
        }

        let mut writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut writer) = *writer_guard {
            self.write_row(writer, frame)?;
            self.last_frame_cnt = frame.frame_cnt;
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
            "FRAME_CNT".to_string(),
            "TIMESTAMP".to_string(),
            "FUNC_ID".to_string(),
            "CH_NUM".to_string(),
            "ACC_X".to_string(),
            "ACC_Y".to_string(),
            "ACC_Z".to_string(),
        ];

        for i in 0..32 {
            headers.push(format!("CH{}_IPD", i));
        }

        for i in 0..32 {
            headers.push(format!("CH{}_RAW", i));
        }

        writer.write_record(&headers)?;
        Ok(())
    }

    fn write_row(&self, writer: &mut Writer<std::fs::File>, frame: &GhFuncFrame) -> std::io::Result<()> {
        let mut row: Vec<String> = Vec::new();

        row.push(frame.frame_cnt.to_string());
        row.push(frame.timestamp.to_string());
        row.push((frame.id as u8).to_string());
        row.push(frame.ch_num.to_string());
        
        row.push(frame.gsensor_data.acc[0].to_string());
        row.push(frame.gsensor_data.acc[1].to_string());
        row.push(frame.gsensor_data.acc[2].to_string());

        for ch_data in &frame.data {
            row.push(ch_data.ipd_pa.to_string());
        }
        for _ in frame.data.len()..32 {
            row.push("0".to_string());
        }

        for ch_data in &frame.data {
            row.push(ch_data.rawdata.to_string());
        }
        for _ in frame.data.len()..32 {
            row.push("0".to_string());
        }

        writer.write_record(&row)?;
        writer.flush()?;

        Ok(())
    }
}
