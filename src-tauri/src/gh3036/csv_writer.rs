use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use csv::{Writer, WriterBuilder};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::types::{Gh3036FrameData, REF_DATA_COUNT};

/// CSV 第一行信息（精简 JSON，对应 gh3036.yaml info_row）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvInfoRow {
    pub app: String,
    pub version: String,
    pub function: String,
    pub ble_name: Option<String>,
    pub ble_address: Option<String>,
}

pub struct CsvWriter {
    writer: Mutex<Option<Writer<std::fs::File>>>,
    output_dir: PathBuf,
    function_name: String,
    info_row: CsvInfoRow,
    current_file_index: u32,
    last_frame_id: i32,
    rows_since_flush: u32,
}

impl CsvWriter {
    pub fn new(output_dir: PathBuf, _function_id: i32, function_name: String) -> Self {
        Self {
            writer: Mutex::new(None),
            output_dir,
            info_row: CsvInfoRow {
                app: String::new(),
                version: String::new(),
                function: function_name.clone(),
                ble_name: None,
                ble_address: None,
            },
            function_name,
            current_file_index: 0,
            last_frame_id: -1,
            rows_since_flush: 0,
        }
    }

    /// 更新 CSV 信息行内容（应用名、版本、测试功能、蓝牙名称/地址）
    ///
    /// 在创建新文件前由管理器刷新，保证信息行与当前采集设备一致
    pub fn set_info_row(&mut self, info_row: CsvInfoRow) {
        self.info_row = info_row;
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

        let mut writer = WriterBuilder::new().flexible(true).from_path(&filepath)?;

        // 第 1 行：精简 JSON 信息行（应用名、版本、测试功能、蓝牙名称、蓝牙地址）
        let info_json = serde_json::to_string(&self.info_row).map_err(std::io::Error::other)?;
        writer.write_record([info_json.as_str()])?;

        // 第 2 行：列名
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
    use super::{CsvInfoRow, CsvWriter};
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

        // 验证：恰好生成两个 CSV 文件，每个文件包含 信息行+表头+一行数据
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
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_path(entry.path())
                .unwrap();
            let records: Vec<_> = reader.records().collect::<Result<_, _>>().unwrap();
            assert_eq!(records.len(), 3, "每个文件应包含信息行、表头和数据行");

            // 第 1 行：精简 JSON 信息行
            let info_json = records[0].get(0).unwrap();
            let info: serde_json::Value =
                serde_json::from_str(info_json).expect("信息行应为合法 JSON");
            assert!(info.get("app").is_some(), "信息行应包含 app 字段");
            assert!(info.get("version").is_some(), "信息行应包含 version 字段");
            assert!(info.get("function").is_some(), "信息行应包含 function 字段");
            assert!(info.get("bleName").is_some(), "信息行应包含 bleName 字段");
            assert!(
                info.get("bleAddress").is_some(),
                "信息行应包含 bleAddress 字段"
            );

            // 第 2 行：表头
            let header: Vec<String> = records[1].iter().map(String::from).collect();
            assert_eq!(header, CsvWriter::headers(), "文件应包含表头行");

            // 第 3 行：数据
            frame_ids.push(records[2].get(1).unwrap().parse::<i32>().unwrap());
        }
        frame_ids.sort_unstable();
        assert_eq!(frame_ids, vec![1, 2], "两个文件应分别包含 frame_id 1 和 2");
    }

    #[test]
    fn info_row_contains_app_and_ble_metadata_before_header() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let mut writer = CsvWriter::new(temp_dir.path().to_path_buf(), 2, "SPO2".to_string());
        writer.set_info_row(CsvInfoRow {
            app: "ComBridge".to_string(),
            version: "0.5.24".to_string(),
            function: "SPO2".to_string(),
            ble_name: Some("GH3036-DEV".to_string()),
            ble_address: Some("AA:BB:CC:DD:EE:FF".to_string()),
        });
        writer.write_frame(&sample_frame()).unwrap();
        drop(writer);

        let file = std::fs::read_dir(temp_dir.path().join("SPO2"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(file)
            .unwrap();
        let records: Vec<_> = reader.records().collect::<Result<_, _>>().unwrap();

        assert_eq!(records.len(), 3, "信息行 + 表头 + 一行数据");

        let info: serde_json::Value =
            serde_json::from_str(records[0].get(0).unwrap()).expect("第一行应为合法 JSON");
        assert_eq!(info["app"], "ComBridge");
        assert_eq!(info["version"], "0.5.24");
        assert_eq!(info["function"], "SPO2");
        assert_eq!(info["bleName"], "GH3036-DEV");
        assert_eq!(info["bleAddress"], "AA:BB:CC:DD:EE:FF");

        let header: Vec<String> = records[1].iter().map(String::from).collect();
        assert_eq!(header, CsvWriter::headers());

        let data = &records[2];
        assert_eq!(data.get(0), Some("123"));
        assert_eq!(data.get(1), Some("7"));
    }
}
