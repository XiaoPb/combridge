use crate::error::{ComBridgeError, Result};
use crate::protocol::LuaEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookType {
    OnDataReceived,
    OnDataSend,
    OnConnect,
    OnDisconnect,
    OnError,
}

impl std::fmt::Display for HookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookType::OnDataReceived => write!(f, "on_data_received"),
            HookType::OnDataSend => write!(f, "on_data_send"),
            HookType::OnConnect => write!(f, "on_connect"),
            HookType::OnDisconnect => write!(f, "on_disconnect"),
            HookType::OnError => write!(f, "on_error"),
        }
    }
}

impl std::str::FromStr for HookType {
    type Err = ComBridgeError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "on_data_received" => Ok(HookType::OnDataReceived),
            "on_data_send" => Ok(HookType::OnDataSend),
            "on_connect" => Ok(HookType::OnConnect),
            "on_disconnect" => Ok(HookType::OnDisconnect),
            "on_error" => Ok(HookType::OnError),
            _ => Err(ComBridgeError::protocol(format!(
                "Unknown hook type: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub success: bool,
    pub data: Option<Vec<u8>>,
    pub message: Option<String>,
}

impl Default for HookResult {
    fn default() -> Self {
        Self {
            success: true,
            data: None,
            message: None,
        }
    }
}

pub struct HookExecutor {
    engine: LuaEngine,
    registered_hooks: HashMap<HookType, String>,
}

impl HookExecutor {
    pub fn new(engine: LuaEngine) -> Self {
        Self {
            engine,
            registered_hooks: HashMap::new(),
        }
    }

    pub fn register_hook(&mut self, hook_type: HookType, function_name: String) -> Result<()> {
        if !self.engine.has_function(&function_name) {
            return Err(ComBridgeError::protocol(format!(
                "Hook function '{}' not found",
                function_name
            )));
        }

        self.registered_hooks.insert(hook_type, function_name);
        Ok(())
    }

    pub fn unregister_hook(&mut self, hook_type: &HookType) {
        self.registered_hooks.remove(hook_type);
    }

    pub fn has_hook(&self, hook_type: &HookType) -> bool {
        self.registered_hooks.contains_key(hook_type)
    }

    pub fn execute_data_hook(&self, hook_type: HookType, data: &[u8]) -> Result<HookResult> {
        let function_name = self.registered_hooks.get(&hook_type).ok_or_else(|| {
            ComBridgeError::protocol(format!("Hook {:?} not registered", hook_type))
        })?;

        match hook_type {
            HookType::OnDataReceived | HookType::OnDataSend => {
                let result_data = self.engine.call_function_with_data(function_name, data)?;
                Ok(HookResult {
                    success: true,
                    data: Some(result_data),
                    message: None,
                })
            }
            _ => {
                self.engine.call_void_function(function_name, vec![])?;
                Ok(HookResult::default())
            }
        }
    }

    pub fn execute_event_hook(&self, hook_type: HookType) -> Result<HookResult> {
        let function_name = self.registered_hooks.get(&hook_type).ok_or_else(|| {
            ComBridgeError::protocol(format!("Hook {:?} not registered", hook_type))
        })?;

        self.engine.call_void_function(function_name, vec![])?;

        Ok(HookResult::default())
    }

    pub fn execute_error_hook(&self, error_message: &str) -> Result<HookResult> {
        let function_name = self
            .registered_hooks
            .get(&HookType::OnError)
            .ok_or_else(|| ComBridgeError::protocol("Error hook not registered"))?;

        self.engine.set_global_string("LAST_ERROR", error_message)?;
        self.engine.call_void_function(function_name, vec![])?;

        Ok(HookResult::default())
    }

    pub fn get_registered_hooks(&self) -> Vec<HookType> {
        self.registered_hooks.keys().copied().collect()
    }

    pub fn clear_hooks(&mut self) {
        self.registered_hooks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> LuaEngine {
        let engine = LuaEngine::new().unwrap();
        engine.register_api().unwrap();
        engine
            .execute_script(
                r#"
            function on_data_received(data)
                local result = {}
                for i, v in ipairs(data) do
                    result[i] = v + 1
                end
                return result
            end

            function on_connect()
                log("Connected!")
            end

            function on_error()
                log("Error occurred!")
            end
        "#,
            )
            .unwrap();
        engine
    }

    #[test]
    fn test_register_hook() {
        let engine = create_test_engine();
        let mut executor = HookExecutor::new(engine);

        executor
            .register_hook(HookType::OnDataReceived, "on_data_received".to_string())
            .unwrap();
        assert!(executor.has_hook(&HookType::OnDataReceived));
    }

    #[test]
    fn test_execute_data_hook() {
        let engine = create_test_engine();
        let mut executor = HookExecutor::new(engine);

        executor
            .register_hook(HookType::OnDataReceived, "on_data_received".to_string())
            .unwrap();

        let data = vec![1, 2, 3];
        let result = executor
            .execute_data_hook(HookType::OnDataReceived, &data)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data, Some(vec![2, 3, 4]));
    }

    #[test]
    fn test_execute_event_hook() {
        let engine = create_test_engine();
        let mut executor = HookExecutor::new(engine);

        executor
            .register_hook(HookType::OnConnect, "on_connect".to_string())
            .unwrap();

        let result = executor.execute_event_hook(HookType::OnConnect).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_unregister_hook() {
        let engine = create_test_engine();
        let mut executor = HookExecutor::new(engine);

        executor
            .register_hook(HookType::OnDataReceived, "on_data_received".to_string())
            .unwrap();
        assert!(executor.has_hook(&HookType::OnDataReceived));

        executor.unregister_hook(&HookType::OnDataReceived);
        assert!(!executor.has_hook(&HookType::OnDataReceived));
    }
}
