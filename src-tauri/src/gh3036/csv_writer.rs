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
    rows_since_flush: u32,
}

impl CsvWriter {
    pub fn new(output_dir: PathBuf, _function_id: i32, function_name: String) -> Self {
        Self {
            writer: Mutex::new(None),
            output_dir,
            function_name,
            current_file_index: 0,
            last_frame_id: -1,
            rows_since_flush: 0,
        }
    }

    pub fn write_frame(&mut self, frame: &Gh3036FrameData) -> std::io::Result<()> {
        let should_create_new_file = frame.frame_id == 0
            || self
                .writer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_none();

        if should_create_new_file {
            self.create_new_file()?;
            self.rows_since_flush = 0;
        }

        let mut writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut writer) = *writer_guard {
            self.write_row(writer, frame)?;
            self.last_frame_id = frame.frame_id;
            self.rows_since_flush += 1;
            if self.rows_since_flush >= 100 {
                writer.flush()?;
                self.rows_since_flush = 0;
            }
        }

        Ok(())
    }

    /// 强制创建新的 CSV 文件
    ///
    /// 用于在特定事件（如启动/停止命令、设备断开）发生时，
    /// 确保后续数据写入新文件
    pub fn force_new_file(&mut self) -> std::io::Result<()> {
        // 先刷新当前文件（如果有）
        {
            let mut writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref mut writer) = *writer_guard {
                writer.flush()?;
            }
            // 清空 writer，下次写入时会创建新文件
            *writer_guard = None;
        }

        self.last_frame_id = -1;
        self.rows_since_flush = 0;

        info!("[CsvWriter] 强制创建新文件标记已设置");
        Ok(())
    }

    fn create_new_file(&mut self) -> std::io::Result<()> {
        let function_output_dir = self.output_dir.join(&self.function_name);
        std::fs::create_dir_all(&function_output_dir)?;

        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!(
            "gh3036_{}_{}_{}.csv",
            self.function_name, timestamp, self.current_file_index
        );
        let filepath = function_output_dir.join(&filename);

        let mut writer = Writer::from_path(&filepath)?;

        self.write_header(&mut writer)?;

        info!("创建新的 CSV 文件: {:?}", filepath);

        let mut writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        *writer_guard = Some(writer);
        self.current_file_index += 1;

        Ok(())
    }

    fn write_header(&self, writer: &mut Writer<std::fs::File>) -> std::io::Result<()> {
        let headers = Self::headers();
        writer.write_record(&headers)?;
        Ok(())
    }

    fn headers() -> Vec<String> {
        let mut headers = vec![
            "TimeStamp".to_string(),
            "FRAME_ID".to_string(),
            "ACCX".to_string(),
            "ACCY".to_string(),
            "ACCZ".to_string(),
        ];
        headers.extend((0..32).map(|i| format!("Ipd{}", i)));
        headers.extend((0..8).map(|i| format!("FLAG{}", i)));
        headers.extend((0..REF_DATA_COUNT).map(|i| format!("REF_RESULT{}", i)));
        headers.extend((0..16).map(|i| format!("ALGO_RESULT{}", i)));
        headers.extend((0..32).map(|i| format!("Rawdata{}", i)));
        headers.extend((0..32).map(|i| format!("AGC_INFO_CH{}", i)));
        headers.extend((0..32).map(|i| format!("LED_INFO_CH{}", i)));
        headers.extend(["GYRO_X", "GYRO_Y", "GYRO_Z"].map(String::from));
        headers
    }

    fn write_row(
        &self,
        writer: &mut Writer<std::fs::File>,
        frame: &Gh3036FrameData,
    ) -> std::io::Result<()> {
        let row = Self::row_values(frame);
        writer.write_record(&row)?;
        Ok(())
    }

    fn row_values(frame: &Gh3036FrameData) -> Vec<String> {
        let mut row: Vec<String> = Vec::new();

        row.push(frame.timestamp.to_string());
        row.push(frame.frame_id.to_string());

        for i in 0..3 {
            let val = frame.gs_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..32 {
            let val = frame.phy_value.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..8 {
            let val = frame.flags.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..REF_DATA_COUNT {
            let val = frame.ref_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..16 {
            let val = frame.algo_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..32 {
            let val = frame.rawdata.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..32 {
            let val = frame.agc_info.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 0..32 {
            let val = frame.led_info.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        for i in 3..6 {
            let val = frame.gs_data.get(i).copied().unwrap_or(0);
            row.push(val.to_string());
        }

        row
    }
}

#[cfg(test)]
mod tests {
    use super::CsvWriter;
    use crate::gh3036::types::Gh3036FrameData;

    fn expected_headers() -> Vec<String> {
        let mut headers = vec![
            "TimeStamp".to_string(),
            "FRAME_ID".to_string(),
            "ACCX".to_string(),
            "ACCY".to_string(),
            "ACCZ".to_string(),
        ];
        headers.extend((0..32).map(|i| format!("Ipd{}", i)));
        headers.extend((0..8).map(|i| format!("FLAG{}", i)));
        headers.extend((0..16).map(|i| format!("REF_RESULT{}", i)));
        headers.extend((0..16).map(|i| format!("ALGO_RESULT{}", i)));
        headers.extend((0..32).map(|i| format!("Rawdata{}", i)));
        headers.extend((0..32).map(|i| format!("AGC_INFO_CH{}", i)));
        headers.extend((0..32).map(|i| format!("LED_INFO_CH{}", i)));
        headers.extend(["GYRO_X", "GYRO_Y", "GYRO_Z"].map(String::from));
        headers
    }

    fn sample_frame() -> Gh3036FrameData {
        Gh3036FrameData {
            function_id: 2,
            function_name: "SPO2".to_string(),
            frame_id: 7,
            timestamp: 123,
            gs_data: vec![1, 2, 3, 4, 5, 6],
            rawdata: vec![100, 101],
            flags: vec![7],
            ref_data: vec![8],
            algo_data: vec![9],
            agc_info: vec![10],
            phy_value: vec![200, 201],
            led_info: vec![11],
        }
    }

    #[test]
    fn headers_match_gh3036_yaml_contract() {
        let headers = CsvWriter::headers();

        assert_eq!(headers.len(), 176);
        assert_eq!(headers, expected_headers());
    }

    #[test]
    fn row_maps_phy_to_ipd_and_raw_channels_to_rawdata() {
        let row = CsvWriter::row_values(&sample_frame());

        assert_eq!(row.len(), 176);
        assert_eq!(&row[0..5], ["123", "7", "1", "2", "3"]);
        assert_eq!(&row[5..7], ["200", "201"]);
        assert_eq!(row[36], "0");
        assert_eq!(row[37], "7");
        assert_eq!(row[45], "8");
        assert_eq!(row[61], "9");
        assert_eq!(&row[77..79], ["100", "101"]);
        assert_eq!(row[109], "10");
        assert_eq!(row[141], "11");
        assert_eq!(&row[173..176], ["4", "5", "6"]);
    }

    #[test]
    fn force_new_file_creates_separate_files() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().to_path_buf();
        let mut writer = CsvWriter::new(output_dir.clone(), 2, "SPO2".to_string());

        // 写入第一帧
        let frame1 = Gh3036FrameData {
            function_id: 2,
            function_name: "SPO2".to_string(),
            frame_id: 1,
            timestamp: 100,
            gs_data: vec![1, 2, 3, 4, 5, 6],
            rawdata: vec![100],
            flags: vec![0],
            ref_data: vec![0],
            algo_data: vec![98],
            agc_info: vec![0],
            phy_value: vec![200],
            led_info: vec![0],
        };
        writer.write_frame(&frame1).unwrap();

        // 强制创建新文件
        writer.force_new_file().unwrap();

        // 写入第二帧（应该在新文件中）
        let frame2 = Gh3036FrameData {
            frame_id: 2,
            timestamp: 200,
            ..frame1.clone()
        };
        writer.write_frame(&frame2).unwrap();

        // 释放 writer，确保所有缓冲数据落盘
        drop(writer);

        // 验证：恰好生成两个 CSV 文件，每个文件包含表头和一行数据
        let csv_files: Vec<_> = std::fs::read_dir(output_dir.join("SPO2"))
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|e| e == "csv")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(csv_files.len(), 2, "应该恰好创建两个CSV文件");

        let mut frame_ids = Vec::new();
        for entry in &csv_files {
            let mut reader = csv::Reader::from_path(entry.path()).unwrap();
            let headers = reader.headers().unwrap().clone();
            assert_eq!(
                headers.iter().map(String::from).collect::<Vec<_>>(),
                CsvWriter::headers(),
                "文件应包含表头行"
            );
            let records: Vec<_> = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
            assert_eq!(records.len(), 1, "每个文件应恰好包含一行数据");
            frame_ids.push(records[0].get(1).unwrap().parse::<i32>().unwrap());
        }
        frame_ids.sort_unstable();
        assert_eq!(frame_ids, vec![1, 2], "两个文件应分别包含 frame_id 1 和 2");
    }
}
