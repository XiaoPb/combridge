use crate::error::{ComBridgeError, Result};
use mlua::{FromLua, Function, Lua, LuaOptions, StdLib, Value};
use std::sync::{Arc, Mutex};

pub struct LuaEngine {
    lua: Arc<Mutex<Lua>>,
}

impl LuaEngine {
    pub fn new() -> Result<Self> {
        let lua = Lua::new_with(
            StdLib::ALL,
            LuaOptions::default(),
        ).map_err(|e| ComBridgeError::protocol(format!("Failed to create Lua VM: {}", e)))?;

        Ok(Self {
            lua: Arc::new(Mutex::new(lua)),
        })
    }

    pub fn execute_script(&self, script: &str) -> Result<()> {
        let lua = self.lua.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock Lua VM: {}", e))
        })?;

        lua.load(script)
            .exec()
            .map_err(|e| ComBridgeError::protocol(format!("Script execution failed: {}", e)))?;

        Ok(())
    }

    pub fn call_function<R>(&self, name: &str, args: Vec<Value>) -> Result<R>
    where
        R: for<'lua> FromLua<'lua> + 'static,
    {
        let lua = self.lua.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock Lua VM: {}", e))
        })?;

        let func: Function = lua
            .globals()
            .get(name)
            .map_err(|e| ComBridgeError::protocol(format!("Function '{}' not found: {}", name, e)))?;

        let result: R = func
            .call(args)
            .map_err(|e| ComBridgeError::protocol(format!("Function call failed: {}", e)))?;

        Ok(result)
    }

    pub fn call_function_with_data(&self, name: &str, data: &[u8]) -> Result<Vec<u8>> {
        let lua = self.lua.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock Lua VM: {}", e))
        })?;

        let func: Function = lua
            .globals()
            .get(name)
            .map_err(|e| ComBridgeError::protocol(format!("Function '{}' not found: {}", name, e)))?;

        let data_table = lua.create_table().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to create table: {}", e))
        })?;

        for (i, byte) in data.iter().enumerate() {
            data_table
                .set(i + 1, *byte)
                .map_err(|e| ComBridgeError::protocol(format!("Failed to set table value: {}", e)))?;
        }

        let result_table: mlua::Table = func
            .call(vec![Value::Table(data_table)])
            .map_err(|e| ComBridgeError::protocol(format!("Function call failed: {}", e)))?;

        let mut result = Vec::new();
        let len: i64 = result_table
            .raw_get("n")
            .unwrap_or_else(|_| result_table.len().unwrap_or(0) as i64);

        for i in 1..=len {
            let byte: u8 = result_table
                .get(i)
                .map_err(|e| ComBridgeError::protocol(format!("Failed to get table value: {}", e)))?;
            result.push(byte);
        }

        Ok(result)
    }

    pub fn has_function(&self, name: &str) -> bool {
        if let Ok(lua) = self.lua.lock() {
            lua.globals().get::<_, Function>(name).is_ok()
        } else {
            false
        }
    }

    pub fn get_global_string(&self, name: &str) -> Result<String> {
        let lua = self.lua.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock Lua VM: {}", e))
        })?;

        let value: String = lua
            .globals()
            .get(name)
            .map_err(|e| ComBridgeError::protocol(format!("Global '{}' not found: {}", name, e)))?;

        Ok(value)
    }

    pub fn set_global_string(&self, name: &str, value: &str) -> Result<()> {
        let lua = self.lua.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock Lua VM: {}", e))
        })?;

        lua.globals()
            .set(name, value)
            .map_err(|e| ComBridgeError::protocol(format!("Failed to set global: {}", e)))?;

        Ok(())
    }

    pub fn register_api(&self) -> Result<()> {
        let lua = self.lua.lock().map_err(|e| {
            ComBridgeError::protocol(format!("Failed to lock Lua VM: {}", e))
        })?;

        let log_fn = lua.create_function(|_, msg: String| {
            tracing::info!("[Lua] {}", msg);
            Ok(())
        }).map_err(|e| ComBridgeError::protocol(format!("Failed to create log function: {}", e)))?;

        lua.globals()
            .set("log", log_fn)
            .map_err(|e| ComBridgeError::protocol(format!("Failed to register log function: {}", e)))?;

        let warn_fn = lua.create_function(|_, msg: String| {
            tracing::warn!("[Lua] {}", msg);
            Ok(())
        }).map_err(|e| ComBridgeError::protocol(format!("Failed to create warn function: {}", e)))?;

        lua.globals()
            .set("warn", warn_fn)
            .map_err(|e| ComBridgeError::protocol(format!("Failed to register warn function: {}", e)))?;

        let error_fn = lua.create_function(|_, msg: String| {
            tracing::error!("[Lua] {}", msg);
            Ok(())
        }).map_err(|e| ComBridgeError::protocol(format!("Failed to create error function: {}", e)))?;

        lua.globals()
            .set("error_log", error_fn)
            .map_err(|e| ComBridgeError::protocol(format!("Failed to register error_log function: {}", e)))?;

        Ok(())
    }
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default LuaEngine")
    }
}

impl Clone for LuaEngine {
    fn clone(&self) -> Self {
        Self {
            lua: Arc::clone(&self.lua),
        }
    }
}

unsafe impl Send for LuaEngine {}
unsafe impl Sync for LuaEngine {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_engine_creation() {
        let engine = LuaEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_execute_script() {
        let engine = LuaEngine::new().unwrap();
        let result = engine.execute_script("x = 10 + 20");
        assert!(result.is_ok());
        assert_eq!(engine.get_global_string("x").unwrap(), "30");
    }

    #[test]
    fn test_call_function() {
        let engine = LuaEngine::new().unwrap();
        engine.execute_script("function add(a, b) return a + b end").unwrap();
        let result: i32 = engine.call_function("add", vec![Value::Integer(5), Value::Integer(3)]).unwrap();
        assert_eq!(result, 8);
    }

    #[test]
    fn test_has_function() {
        let engine = LuaEngine::new().unwrap();
        assert!(!engine.has_function("test_func"));
        engine.execute_script("function test_func() end").unwrap();
        assert!(engine.has_function("test_func"));
    }
}
