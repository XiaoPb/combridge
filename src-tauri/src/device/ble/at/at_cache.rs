use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct DeviceCache {
    pub name: Option<String>,
    pub rssi: i16,
}

impl Default for DeviceCache {
    fn default() -> Self {
        Self {
            name: None,
            rssi: -100,
        }
    }
}

pub struct AtCache {
    devices: RwLock<HashMap<String, DeviceCache>>,
}

impl AtCache {
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(HashMap::new()),
        }
    }

    pub fn update_device(&self, address: &str, name: Option<String>, rssi: i16) {
        let mut devices = self.devices.write().unwrap_or_else(|e| e.into_inner());
        let device = devices.entry(address.to_string()).or_default();
        if let Some(n) = name {
            device.name = Some(n);
        }
        device.rssi = rssi;
    }

    pub fn get_device(&self, address: &str) -> Option<DeviceCache> {
        let devices = self.devices.read().unwrap_or_else(|e| e.into_inner());
        devices.get(address).cloned()
    }

    pub fn remove_device(&self, address: &str) {
        let mut devices = self.devices.write().unwrap_or_else(|e| e.into_inner());
        devices.remove(address);
    }

    pub fn clear(&self) {
        let mut devices = self.devices.write().unwrap_or_else(|e| e.into_inner());
        devices.clear();
    }

    pub fn get_all_devices(&self) -> Vec<(String, DeviceCache)> {
        let devices = self.devices.read().unwrap_or_else(|e| e.into_inner());
        devices.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

impl Default for AtCache {
    fn default() -> Self {
        Self::new()
    }
}
