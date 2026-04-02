pub mod hook_executor;
pub mod lua_engine;
pub mod plugin_manager;
pub mod script_loader;

pub use hook_executor::{HookExecutor, HookType};
pub use lua_engine::LuaEngine;
pub use plugin_manager::{PluginInfo, PluginManager, PluginState};
pub use script_loader::ScriptLoader;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub hooks: Vec<String>,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            hooks: Vec::new(),
        }
    }
}
