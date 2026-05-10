use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    B115200 = 115200,
    #[serde(rename = "230400")]
    B230400 = 230400,
    #[serde(rename = "460800")]
    B460800 = 460800,
    #[serde(rename = "921600")]
    B921600 = 921600,
}

impl Default for BaudRate {
    fn default() -> Self {
        BaudRate::B115200
    }
}

impl From<BaudRate> for u32 {
    fn from(baud: BaudRate) -> Self {
        baud as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataBits {
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
}

impl Default for DataBits {
    fn default() -> Self {
        DataBits::Eight
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl Default for Parity {
    fn default() -> Self {
        Parity::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopBits {
    One = 1,
    Two = 2,
}

impl Default for StopBits {
    fn default() -> Self {
        StopBits::One
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

impl Default for FlowControl {
    fn default() -> Self {
        FlowControl::None
    }
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
}
