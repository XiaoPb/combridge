use crate::error::{ComBridgeError, Result};
use crate::protocol::{LuaEngine, ProtocolConfig};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ScriptLoader {
    cache: HashMap<PathBuf, String>,
    compiled_cache: HashMap<PathBuf, LuaEngine>,
}

impl ScriptLoader {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            compiled_cache: HashMap::new(),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<String> {
        let path = path.as_ref().to_path_buf();

        if let Some(cached) = self.cache.get(&path) {
            return Ok(cached.clone());
        }

        let content = fs::read_to_string(&path).map_err(|e| {
            ComBridgeError::protocol(format!("Failed to read script file '{}': {}", path.display(), e))
        })?;

        self.cache.insert(path.clone(), content.clone());

        Ok(content)
    }

    pub fn load_and_compile<P: AsRef<Path>>(&mut self, path: P) -> Result<LuaEngine> {
        let path = path.as_ref().to_path_buf();

        if let Some(engine) = self.compiled_cache.get(&path) {
            return Ok(engine.clone());
        }

        let content = self.load_from_file(&path)?;

        let engine = LuaEngine::new()?;
        engine.register_api()?;
        engine.execute_script(&content)?;

        self.compiled_cache.insert(path.clone(), engine.clone());

        Ok(engine)
    }

    pub fn validate_script(content: &str) -> Result<ProtocolConfig> {
        let engine = LuaEngine::new()?;
        engine.register_api()?;
        engine.execute_script(content)?;

        let mut config = ProtocolConfig::default();

        if engine.has_function("get_config") {
            if let Ok(name) = engine.get_global_string("PROTOCOL_NAME") {
                config.name = name;
            }
            if let Ok(version) = engine.get_global_string("PROTOCOL_VERSION") {
                config.version = version;
            }
            if let Ok(description) = engine.get_global_string("PROTOCOL_DESCRIPTION") {
                config.description = Some(description);
            }
            if let Ok(author) = engine.get_global_string("PROTOCOL_AUTHOR") {
                config.author = Some(author);
            }
        }

        let mut hooks = Vec::new();
        if engine.has_function("on_data_received") {
            hooks.push("on_data_received".to_string());
        }
        if engine.has_function("on_data_send") {
            hooks.push("on_data_send".to_string());
        }
        if engine.has_function("on_connect") {
            hooks.push("on_connect".to_string());
        }
        if engine.has_function("on_disconnect") {
            hooks.push("on_disconnect".to_string());
        }
        if engine.has_function("on_error") {
            hooks.push("on_error".to_string());
        }

        config.hooks = hooks;

        Ok(config)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.compiled_cache.clear();
    }

    pub fn remove_from_cache<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        self.cache.remove(&path);
        self.compiled_cache.remove(&path);
    }

    pub fn is_cached<P: AsRef<Path>>(&self, path: P) -> bool {
        self.cache.contains_key(path.as_ref())
    }

    pub fn scan_directory<P: AsRef<Path>>(&self, dir: P) -> Result<Vec<PathBuf>> {
        let dir = dir.as_ref();

        if !dir.exists() {
            return Err(ComBridgeError::protocol(format!(
                "Directory '{}' does not exist",
                dir.display()
            )));
        }

        let mut scripts = Vec::new();

        let entries = fs::read_dir(dir).map_err(|e| {
            ComBridgeError::protocol(format!("Failed to read directory '{}': {}", dir.display(), e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                ComBridgeError::protocol(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "lua") {
                scripts.push(path);
            }
        }

        Ok(scripts)
    }
}

impl Default for ScriptLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "x = 10 + 20").unwrap();

        let mut loader = ScriptLoader::new();
        let content = loader.load_from_file(temp_file.path()).unwrap();
        assert!(content.contains("x = 10 + 20"));
    }

    #[test]
    fn test_validate_script() {
        let script = r#"
            PROTOCOL_NAME = "TestProtocol"
            PROTOCOL_VERSION = "1.0.0"
            PROTOCOL_DESCRIPTION = "A test protocol"
            PROTOCOL_AUTHOR = "Test Author"

            function on_data_received(data)
                return data
            end
        "#;

        let config = ScriptLoader::validate_script(script).unwrap();
        assert_eq!(config.name, "TestProtocol");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.description, Some("A test protocol".to_string()));
        assert_eq!(config.author, Some("Test Author".to_string()));
        assert!(config.hooks.contains(&"on_data_received".to_string()));
    }

    #[test]
    fn test_cache() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "x = 10").unwrap();

        let mut loader = ScriptLoader::new();
        assert!(!loader.is_cached(temp_file.path()));

        loader.load_from_file(temp_file.path()).unwrap();
        assert!(loader.is_cached(temp_file.path()));

        loader.remove_from_cache(temp_file.path());
        assert!(!loader.is_cached(temp_file.path()));
    }
}
