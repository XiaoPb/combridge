use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::at_commands::{ServiceInfo, CharInfo};

#[derive(Debug, Clone)]
pub struct DeviceCache {
    pub name: Option<String>,
    pub rssi: i16,
    pub services: HashMap<String, ServiceCache>,
}

#[derive(Debug, Clone)]
pub struct ServiceCache {
    pub primary: bool,
    pub characteristics: HashMap<String, CharCache>,
}

#[derive(Debug, Clone)]
pub struct CharCache {
    pub properties: u8,
}

impl Default for DeviceCache {
    fn default() -> Self {
        Self {
            name: None,
            rssi: -100,
            services: HashMap::new(),
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
        let mut devices = self.devices.write().unwrap();
        let device = devices.entry(address.to_string()).or_default();
        if let Some(n) = name {
            device.name = Some(n);
        }
        device.rssi = rssi;
    }

    pub fn get_device(&self, address: &str) -> Option<DeviceCache> {
        let devices = self.devices.read().unwrap();
        devices.get(address).cloned()
    }

    pub fn update_services(&self, address: &str, services: Vec<ServiceInfo>) {
        let mut devices = self.devices.write().unwrap();
        let device = devices.entry(address.to_string()).or_default();
        
        for svc in services {
            device.services.insert(
                svc.uuid.clone(),
                ServiceCache {
                    primary: svc.primary,
                    characteristics: HashMap::new(),
                },
            );
        }
    }

    pub fn update_characteristics(&self, address: &str, service_uuid: &str, characteristics: Vec<CharInfo>) {
        let mut devices = self.devices.write().unwrap();
        let device = devices.entry(address.to_string()).or_default();
        
        if let Some(service) = device.services.get_mut(service_uuid) {
            for ch in characteristics {
                service.characteristics.insert(
                    ch.uuid.clone(),
                    CharCache {
                        properties: ch.properties,
                    },
                );
            }
        }
    }

    pub fn get_services(&self, address: &str) -> Option<Vec<ServiceInfo>> {
        let devices = self.devices.read().unwrap();
        devices.get(address).map(|d| {
            d.services.iter()
                .map(|(uuid, svc)| ServiceInfo {
                    uuid: uuid.clone(),
                    primary: svc.primary,
                })
                .collect()
        })
    }

    pub fn get_characteristics(&self, address: &str, service_uuid: &str) -> Option<Vec<CharInfo>> {
        let devices = self.devices.read().unwrap();
        devices.get(address).and_then(|d| {
            d.services.get(service_uuid).map(|svc| {
                svc.characteristics.iter()
                    .map(|(uuid, ch)| CharInfo {
                        uuid: uuid.clone(),
                        service_uuid: service_uuid.to_string(),
                        properties: ch.properties,
                    })
                    .collect()
            })
        })
    }

    pub fn remove_device(&self, address: &str) {
        let mut devices = self.devices.write().unwrap();
        devices.remove(address);
    }

    pub fn clear(&self) {
        let mut devices = self.devices.write().unwrap();
        devices.clear();
    }

    pub fn get_all_devices(&self) -> Vec<(String, DeviceCache)> {
        let devices = self.devices.read().unwrap();
        devices.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

impl Default for AtCache {
    fn default() -> Self {
        Self::new()
    }
}
