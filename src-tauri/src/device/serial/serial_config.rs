use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum BaudRate {
    #[serde(rename = "1200")]
    B1200 = 1200,
    #[serde(rename = "2400")]
    B2400 = 2400,
    #[serde(rename = "4800")]
    B4800 = 4800,
    #[serde(rename = "9600")]
    B9600 = 9600,
    #[serde(rename = "19200")]
    B19200 = 19200,
    #[serde(rename = "38400")]
    B38400 = 38400,
    #[serde(rename = "57600")]
    B57600 = 57600,
    #[serde(rename = "115200")]
    #[default]
    B115200 = 115200,
    #[serde(rename = "230400")]
    B230400 = 230400,
    #[serde(rename = "460800")]
    B460800 = 460800,
    #[serde(rename = "921600")]
    B921600 = 921600,
}

impl From<BaudRate> for u32 {
    fn from(baud: BaudRate) -> Self {
        baud as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DataBits {
    Five = 5,
    Six = 6,
    Seven = 7,
    #[default]
    Eight = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StopBits {
    #[default]
    One = 1,
    Two = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum FlowControl {
    #[default]
    None,
    Software,
    Hardware,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialPortConfig {
    pub port_name: String,
    #[serde(default)]
    pub baud_rate: BaudRate,
    #[serde(default)]
    pub data_bits: DataBits,
    #[serde(default)]
    pub parity: Parity,
    #[serde(default)]
    pub stop_bits: StopBits,
    #[serde(default)]
    pub flow_control: FlowControl,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_pack_timeout")]
    pub pack_timeout_ms: u64,
}

fn default_timeout() -> u64 {
    1000
}

fn default_pack_timeout() -> u64 {
    50
}

impl Default for SerialPortConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: BaudRate::default(),
            data_bits: DataBits::default(),
            parity: Parity::default(),
            stop_bits: StopBits::default(),
            flow_control: FlowControl::default(),
            timeout_ms: default_timeout(),
            pack_timeout_ms: default_pack_timeout(),
        }
    }
}

impl SerialPortConfig {
    pub fn new(port_name: impl Into<String>) -> Self {
        Self {
            port_name: port_name.into(),
            ..Default::default()
        }
    }

    pub fn baud_rate(mut self, baud_rate: BaudRate) -> Self {
        self.baud_rate = baud_rate;
        self
    }

    pub fn data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    pub fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub fn flow_control(mut self, flow_control: FlowControl) -> Self {
        self.flow_control = flow_control;
        self
    }

    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn pack_timeout_ms(mut self, pack_timeout_ms: u64) -> Self {
        self.pack_timeout_ms = pack_timeout_ms;
        self
    }

    /// 验证串口配置参数的有效性
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 配置有效
    /// - `Err(String)`: 配置无效，包含错误描述
    ///
    /// # 验证规则
    ///
    /// - 波特率必须在 300 - 2000000 范围内
    /// - 数据位必须是 5-8
    /// - 停止位必须是 1 或 2
    /// - 超时时间必须大于 0
    pub fn validate(&self) -> Result<(), String> {
        // 验证波特率范围（300 - 2000000 bps）
        let baud_rate_value = u32::from(self.baud_rate);
        if !(300..=2_000_000).contains(&baud_rate_value) {
            return Err(format!(
                "波特率必须在 300 - 2000000 范围内，当前值: {}",
                baud_rate_value
            ));
        }

        // 验证数据位（虽然枚举已经限制了值，但保留验证以明确业务规则）
        let data_bits_value = self.data_bits as u8;
        if ![5, 6, 7, 8].contains(&data_bits_value) {
            return Err(format!("数据位必须是 5-8，当前值: {}", data_bits_value));
        }

        // 验证停止位
        let stop_bits_value = self.stop_bits as u8;
        if ![1, 2].contains(&stop_bits_value) {
            return Err(format!("停止位必须是 1 或 2，当前值: {}", stop_bits_value));
        }

        // 验证超时时间
        if self.timeout_ms == 0 {
            return Err("超时时间必须大于 0".to_string());
        }

        if self.pack_timeout_ms == 0 {
            return Err("打包超时时间必须大于 0".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub port_type: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baud_rate_default() {
        assert_eq!(BaudRate::default(), BaudRate::B115200);
    }

    #[test]
    fn test_baud_rate_to_u32() {
        assert_eq!(u32::from(BaudRate::B9600), 9600);
        assert_eq!(u32::from(BaudRate::B115200), 115200);
    }

    #[test]
    fn test_serial_port_config_builder() {
        let config = SerialPortConfig::new("COM1")
            .baud_rate(BaudRate::B9600)
            .data_bits(DataBits::Seven)
            .parity(Parity::Even)
            .stop_bits(StopBits::Two)
            .flow_control(FlowControl::Hardware)
            .timeout_ms(2000);

        assert_eq!(config.port_name, "COM1");
        assert_eq!(config.baud_rate, BaudRate::B9600);
        assert_eq!(config.data_bits, DataBits::Seven);
        assert_eq!(config.parity, Parity::Even);
        assert_eq!(config.stop_bits, StopBits::Two);
        assert_eq!(config.flow_control, FlowControl::Hardware);
        assert_eq!(config.timeout_ms, 2000);
    }

    #[test]
    fn test_serial_port_config_serde() {
        let config = SerialPortConfig::new("COM3").baud_rate(BaudRate::B115200);

        let json = serde_json::to_string(&config).unwrap();
        let parsed: SerialPortConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.port_name, "COM3");
        assert_eq!(parsed.baud_rate, BaudRate::B115200);
    }

    #[test]
    fn test_serial_port_config_validate_success() {
        // 测试默认配置（所有值都应该有效）
        let config = SerialPortConfig::new("COM1");
        assert!(config.validate().is_ok());

        // 测试边界波特率值
        let config_low = SerialPortConfig::new("COM1").baud_rate(BaudRate::B1200);
        assert!(config_low.validate().is_ok());

        let config_high = SerialPortConfig::new("COM1").baud_rate(BaudRate::B921600);
        assert!(config_high.validate().is_ok());

        // 测试各种数据位
        for data_bits in [
            DataBits::Five,
            DataBits::Six,
            DataBits::Seven,
            DataBits::Eight,
        ] {
            let config = SerialPortConfig::new("COM1").data_bits(data_bits);
            assert!(config.validate().is_ok());
        }

        // 测试各种停止位
        for stop_bits in [StopBits::One, StopBits::Two] {
            let config = SerialPortConfig::new("COM1").stop_bits(stop_bits);
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_serial_port_config_validate_timeout_zero() {
        // 测试超时时间为 0
        let config = SerialPortConfig::new("COM1").timeout_ms(0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("超时时间必须大于 0"));
    }

    #[test]
    fn test_serial_port_config_validate_pack_timeout_zero() {
        // 测试打包超时时间为 0
        let config = SerialPortConfig::new("COM1").pack_timeout_ms(0);
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("打包超时时间必须大于 0"));
    }
}
