use crate::error::{ComBridgeError, Result};
use crate::protocol::{HookExecutor, HookType, LuaEngine, ProtocolConfig, ScriptLoader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    Unloaded,
    Loaded,
    Enabled,
    Disabled,
    Error,
}

impl std::fmt::Display for PluginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginState::Unloaded => write!(f, "未加载"),
            PluginState::Loaded => write!(f, "已加载"),
            PluginState::Enabled => write!(f, "已启用"),
            PluginState::Disabled => write!(f, "已禁用"),
            PluginState::Error => write!(f, "错误"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub path: PathBuf,
    pub state: PluginState,
    pub hooks: Vec<String>,
    pub bound_devices: Vec<String>,
    pub error_message: Option<String>,
}

struct PluginInternal {
    engine: LuaEngine,
    executor: HookExecutor,
    config: ProtocolConfig,
    state: PluginState,
    bound_devices: Vec<String>,
    error_message: Option<String>,
}

pub struct PluginManager {
    plugins: Arc<Mutex<HashMap<String, PluginInternal>>>,
    plugin_infos: Arc<Mutex<HashMap<String, PluginInfo>>>,
    loader: Arc<Mutex<ScriptLoader>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
            plugin_infos: Arc::new(Mutex::new(HashMap::new())),
            loader: Arc::new(Mutex::new(ScriptLoader::new())),
        }
    }

    pub fn load_plugin(&self, plugin_id: &str, path: PathBuf) -> Result<PluginInfo> {
        let mut plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        if plugins.contains_key(plugin_id) {
            return Err(ComBridgeError::protocol(format!(
                "Plugin '{}' already loaded",
                plugin_id
            )));
        }

        let mut loader = self.loader.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock loader: {}", e))
        })?;

        let engine = loader.load_and_compile(&path)?;

        let config = ScriptLoader::validate_script(&loader.load_from_file(&path)?)?;

        let mut executor = HookExecutor::new(engine.clone());
        for hook_name in &config.hooks {
            if let Ok(hook_type) = hook_name.parse::<HookType>() {
                executor.register_hook(hook_type, hook_name.clone())?;
            }
        }

        let plugin = PluginInternal {
            engine,
            executor,
            config: config.clone(),
            state: PluginState::Loaded,
            bound_devices: Vec::new(),
            error_message: None,
        };

        plugins.insert(plugin_id.to_string(), plugin);

        let info = PluginInfo {
            id: plugin_id.to_string(),
            name: config.name.clone(),
            version: config.version.clone(),
            description: config.description.clone(),
            author: config.author.clone(),
            path,
            state: PluginState::Loaded,
            hooks: config.hooks.clone(),
            bound_devices: Vec::new(),
            error_message: None,
        };

        let mut plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;
        plugin_infos.insert(plugin_id.to_string(), info.clone());

        Ok(info)
    }

    pub fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        let mut plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        if let Some(_plugin) = plugins.remove(plugin_id) {
            let path = plugin_infos.get(plugin_id).map(|info| info.path.clone());
            plugin_infos.remove(plugin_id);

            if let Some(path) = path {
                let mut loader = self.loader.lock().map_err(|e| {
                    ComBridgeError::protocol(format!("Failed to lock loader: {}", e))
                })?;
                loader.remove_from_cache(&path);
            }
        }

        Ok(())
    }

    pub fn enable_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        let mut plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.state = PluginState::Enabled;
            if let Some(info) = plugin_infos.get_mut(plugin_id) {
                info.state = PluginState::Enabled;
            }
        } else {
            return Err(ComBridgeError::protocol(format!(
                "Plugin '{}' not found",
                plugin_id
            )));
        }

        Ok(())
    }

    pub fn disable_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        let mut plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.state = PluginState::Disabled;
            if let Some(info) = plugin_infos.get_mut(plugin_id) {
                info.state = PluginState::Disabled;
            }
        } else {
            return Err(ComBridgeError::protocol(format!(
                "Plugin '{}' not found",
                plugin_id
            )));
        }

        Ok(())
    }

    pub fn bind_protocol(&self, plugin_id: &str, device_id: &str) -> Result<()> {
        let mut plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        let mut plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        if let Some(plugin) = plugins.get_mut(plugin_id) {
            if !plugin.bound_devices.contains(&device_id.to_string()) {
                plugin.bound_devices.push(device_id.to_string());
            }
            if let Some(info) = plugin_infos.get_mut(plugin_id) {
                if !info.bound_devices.contains(&device_id.to_string()) {
                    info.bound_devices.push(device_id.to_string());
                }
            }
        } else {
            return Err(ComBridgeError::protocol(format!(
                "Plugin '{}' not found",
                plugin_id
            )));
        }

        Ok(())
    }

    pub fn unbind_protocol(&self, plugin_id: &str, device_id: &str) -> Result<()> {
        let mut plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        let mut plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.bound_devices.retain(|id| id != device_id);
            if let Some(info) = plugin_infos.get_mut(plugin_id) {
                info.bound_devices.retain(|id| id != device_id);
            }
        } else {
            return Err(ComBridgeError::protocol(format!(
                "Plugin '{}' not found",
                plugin_id
            )));
        }

        Ok(())
    }

    pub fn get_plugin(&self, plugin_id: &str) -> Result<PluginInfo> {
        let plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        plugin_infos
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| ComBridgeError::protocol(format!("Plugin '{}' not found", plugin_id)))
    }

    pub fn list_protocols(&self) -> Result<Vec<PluginInfo>> {
        let plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        Ok(plugin_infos.values().cloned().collect())
    }

    pub fn execute_hook(&self, plugin_id: &str, hook_type: HookType, data: &[u8]) -> Result<Vec<u8>> {
        let plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| ComBridgeError::protocol(format!("Plugin '{}' not found", plugin_id)))?;

        if plugin.state != PluginState::Enabled {
            return Err(ComBridgeError::protocol(format!(
                "Plugin '{}' is not enabled",
                plugin_id
            )));
        }

        let result = plugin.executor.execute_data_hook(hook_type, data)?;

        result.data.ok_or_else(|| {
            ComBridgeError::protocol("Hook execution returned no data".to_string())
        })
    }

    pub fn execute_event(&self, plugin_id: &str, hook_type: HookType) -> Result<()> {
        let plugins = self.plugins.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugins: {}", e))
        })?;

        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| ComBridgeError::protocol(format!("Plugin '{}' not found", plugin_id)))?;

        if plugin.state != PluginState::Enabled {
            return Err(ComBridgeError::protocol(format!(
                "Plugin '{}' is not enabled",
                plugin_id
            )));
        }

        plugin.executor.execute_event_hook(hook_type)?;

        Ok(())
    }

    pub fn get_bound_plugins(&self, device_id: &str) -> Result<Vec<PluginInfo>> {
        let plugin_infos = self.plugin_infos.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock plugin infos: {}", e))
        })?;

        let bound: Vec<PluginInfo> = plugin_infos
            .values()
            .filter(|info| info.bound_devices.contains(&device_id.to_string()))
            .cloned()
            .collect();

        Ok(bound)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PluginManager {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
            plugin_infos: Arc::clone(&self.plugin_infos),
            loader: Arc::clone(&self.loader),
        }
    }
}

unsafe impl Send for PluginManager {}
unsafe impl Sync for PluginManager {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_plugin_file() -> NamedTempFile {
        let mut temp_file = NamedTempFile::with_suffix(".lua").unwrap();
        writeln!(
            temp_file,
            r#"
            PROTOCOL_NAME = "TestProtocol"
            PROTOCOL_VERSION = "1.0.0"
            PROTOCOL_DESCRIPTION = "Test Description"
            PROTOCOL_AUTHOR = "Test Author"

            function on_data_received(data)
                return data
            end

            function on_connect()
                log("Connected")
            end
        "#
        )
        .unwrap();
        temp_file
    }

    #[test]
    fn test_load_plugin() {
        let temp_file = create_test_plugin_file();
        let manager = PluginManager::new();

        let info = manager.load_plugin("test", temp_file.path().to_path_buf()).unwrap();
        assert_eq!(info.id, "test");
        assert_eq!(info.name, "TestProtocol");
        assert_eq!(info.state, PluginState::Loaded);
    }

    #[test]
    fn test_enable_disable_plugin() {
        let temp_file = create_test_plugin_file();
        let manager = PluginManager::new();

        manager.load_plugin("test", temp_file.path().to_path_buf()).unwrap();

        manager.enable_plugin("test").unwrap();
        let info = manager.get_plugin("test").unwrap();
        assert_eq!(info.state, PluginState::Enabled);

        manager.disable_plugin("test").unwrap();
        let info = manager.get_plugin("test").unwrap();
        assert_eq!(info.state, PluginState::Disabled);
    }

    #[test]
    fn test_bind_unbind_protocol() {
        let temp_file = create_test_plugin_file();
        let manager = PluginManager::new();

        manager.load_plugin("test", temp_file.path().to_path_buf()).unwrap();
        manager.bind_protocol("test", "device1").unwrap();

        let info = manager.get_plugin("test").unwrap();
        assert!(info.bound_devices.contains(&"device1".to_string()));

        manager.unbind_protocol("test", "device1").unwrap();
        let info = manager.get_plugin("test").unwrap();
        assert!(!info.bound_devices.contains(&"device1".to_string()));
    }

    #[test]
    fn test_unload_plugin() {
        let temp_file = create_test_plugin_file();
        let manager = PluginManager::new();

        manager.load_plugin("test", temp_file.path().to_path_buf()).unwrap();
        assert!(manager.get_plugin("test").is_ok());

        manager.unload_plugin("test").unwrap();
        assert!(manager.get_plugin("test").is_err());
    }

    #[test]
    fn test_list_protocols() {
        let temp_file = create_test_plugin_file();
        let manager = PluginManager::new();

        manager.load_plugin("test1", temp_file.path().to_path_buf()).unwrap();
        manager.load_plugin("test2", temp_file.path().to_path_buf()).unwrap();

        let list = manager.list_protocols().unwrap();
        assert_eq!(list.len(), 2);
    }
}
