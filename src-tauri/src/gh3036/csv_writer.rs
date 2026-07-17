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

    fn create_new_file(&mut self) -> std::io::Result<()> {
        let function_output_dir = self.output_dir.join(&self.function_name);
        std::fs::create_dir_all(&function_output_dir)?;

        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("gh3036_{}_{}.csv", self.function_name, timestamp);
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
}
