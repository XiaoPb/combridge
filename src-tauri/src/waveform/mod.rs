pub mod buffer;
pub mod parser;

pub use buffer::{WaveformBuffer, WaveformBufferConfig};
pub use parser::{DataParser, ParserConfig, ParserManager, ParserType};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformStatus {
    pub buffer_id: String,
    pub row_count: usize,
    pub column_count: usize,
    pub column_names: Vec<String>,
    pub capacity: usize,
    pub parser_type: Option<ParserType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<f64>>,
    pub timestamp: u64,
}
