use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParserType {
    Delimiter,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    pub parser_type: ParserType,
    pub delimiter: Option<String>,
    pub pattern: Option<String>,
    pub column_names: Vec<String>,
    pub trim_whitespace: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            parser_type: ParserType::Delimiter,
            delimiter: Some(",".to_string()),
            pattern: None,
            column_names: vec![
                "CH0".to_string(),
                "CH1".to_string(),
                "CH2".to_string(),
                "CH3".to_string(),
                "CH4".to_string(),
            ],
            trim_whitespace: true,
        }
    }
}

pub trait DataParser: Send + Sync {
    fn parse(&self, data: &str) -> Result<Vec<String>, crate::error::ComBridgeError>;
    fn config(&self) -> ParserConfig;
}

pub struct DelimiterParser {
    config: ParserConfig,
}

impl DelimiterParser {
    pub fn new(config: ParserConfig) -> Self {
        Self { config }
    }
}

impl DataParser for DelimiterParser {
    fn parse(&self, data: &str) -> Result<Vec<String>, crate::error::ComBridgeError> {
        let delimiter = self.config.delimiter.as_deref().unwrap_or(",");
        let mut values: Vec<String> = data.split(delimiter).map(|s| s.to_string()).collect();

        if self.config.trim_whitespace {
            values = values.iter().map(|s| s.trim().to_string()).collect();
        }

        Ok(values)
    }

    fn config(&self) -> ParserConfig {
        self.config.clone()
    }
}

pub struct RegexParser {
    config: ParserConfig,
    regex: Regex,
}

impl RegexParser {
    pub fn new(config: ParserConfig) -> Result<Self, crate::error::ComBridgeError> {
        let pattern = config.pattern.as_deref().ok_or_else(|| {
            crate::error::ComBridgeError::parse("Regex pattern is required for RegexParser")
        })?;

        let regex = Regex::new(pattern).map_err(|e| {
            crate::error::ComBridgeError::parse(format!("Invalid regex pattern: {}", e))
        })?;

        Ok(Self { config, regex })
    }
}

impl DataParser for RegexParser {
    fn parse(&self, data: &str) -> Result<Vec<String>, crate::error::ComBridgeError> {
        let captures = self.regex.captures(data).ok_or_else(|| {
            crate::error::ComBridgeError::parse("Regex pattern did not match")
        })?;

        let mut values = Vec::new();
        for i in 1..captures.len() {
            if let Some(cap) = captures.get(i) {
                let value = if self.config.trim_whitespace {
                    cap.as_str().trim().to_string()
                } else {
                    cap.as_str().to_string()
                };
                values.push(value);
            }
        }

        Ok(values)
    }

    fn config(&self) -> ParserConfig {
        self.config.clone()
    }
}

pub struct ParserManager {
    parsers: RwLock<HashMap<String, Arc<dyn DataParser>>>,
}

impl ParserManager {
    pub fn new() -> Self {
        Self {
            parsers: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_parser(&self, id: &str, config: ParserConfig) -> Result<(), crate::error::ComBridgeError> {
        let parser: Arc<dyn DataParser> = match config.parser_type {
            ParserType::Delimiter => Arc::new(DelimiterParser::new(config)),
            ParserType::Regex => Arc::new(RegexParser::new(config)?),
        };

        let mut parsers = self.parsers.write();
        parsers.insert(id.to_string(), parser);

        Ok(())
    }

    pub fn parse(&self, id: &str, data: &str) -> Result<Vec<String>, crate::error::ComBridgeError> {
        let parsers = self.parsers.read();
        let parser = parsers.get(id).ok_or_else(|| {
            crate::error::ComBridgeError::parse(format!("Parser '{}' not found", id))
        })?;

        parser.parse(data)
    }

    pub fn remove_parser(&self, id: &str) {
        let mut parsers = self.parsers.write();
        parsers.remove(id);
    }

    pub fn get_parser_config(&self, id: &str) -> Option<ParserConfig> {
        let parsers = self.parsers.read();
        parsers.get(id).map(|p| p.config())
    }

    pub fn list_parsers(&self) -> Vec<String> {
        let parsers = self.parsers.read();
        parsers.keys().cloned().collect()
    }
}

impl Default for ParserManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delimiter_parser() {
        let config = ParserConfig {
            parser_type: ParserType::Delimiter,
            delimiter: Some(",".to_string()),
            pattern: None,
            column_names: vec!["A".to_string(), "B".to_string()],
            trim_whitespace: true,
        };

        let parser = DelimiterParser::new(config);
        let result = parser.parse("1.5, 2.5, 3.5").unwrap();
        assert_eq!(result, vec!["1.5", "2.5", "3.5"]);
    }

    #[test]
    fn test_regex_parser() {
        let config = ParserConfig {
            parser_type: ParserType::Regex,
            delimiter: None,
            pattern: Some(r"(-?\d+),(-?\d+),(-?\d+)".to_string()),
            column_names: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            trim_whitespace: false,
        };

        let parser = RegexParser::new(config).unwrap();
        let result = parser.parse("10,-20,30").unwrap();
        assert_eq!(result, vec!["10", "-20", "30"]);
    }

    #[test]
    fn test_parser_manager() {
        let manager = ParserManager::new();
        let config = ParserConfig::default();

        manager.create_parser("test", config).unwrap();
        let result = manager.parse("test", "1,2,3,4,5").unwrap();
        assert_eq!(result.len(), 5);

        manager.remove_parser("test");
        assert!(manager.parse("test", "1,2,3").is_err());
    }
}
