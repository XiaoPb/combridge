use mlua::{Lua, Value};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserScriptInfo {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub is_built_in: bool,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFieldInfo {
    pub path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_value: Option<serde_json::Value>,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonStructureInfo {
    pub fields: Vec<JsonFieldInfo>,
    pub is_array: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_item_type: Option<String>,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub key: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

pub struct ParserScriptManager {
    built_in_scripts_dir: PathBuf,
    user_scripts_dir: PathBuf,
    scripts_cache: Mutex<HashMap<String, ParserScriptInfo>>,
}

impl ParserScriptManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let built_in_scripts_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("parser_scripts");

        let user_scripts_dir = app_data_dir.join("parser_scripts");

        Self {
            built_in_scripts_dir,
            user_scripts_dir,
            scripts_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn init_default_scripts(&self) -> Result<(), String> {
        if !self.user_scripts_dir.exists() {
            fs::create_dir_all(&self.user_scripts_dir)
                .map_err(|e| format!("Failed to create user scripts directory: {}", e))?;
            info!(
                "Created user scripts directory: {:?}",
                self.user_scripts_dir
            );
        }

        if self.built_in_scripts_dir.exists() {
            info!(
                "Built-in scripts directory exists: {:?}",
                self.built_in_scripts_dir
            );
        } else {
            warn!(
                "Built-in scripts directory not found: {:?}",
                self.built_in_scripts_dir
            );
        }

        self.refresh_scripts_cache();

        Ok(())
    }

    fn refresh_scripts_cache(&self) {
        let mut cache = self.scripts_cache.lock();
        cache.clear();

        if self.built_in_scripts_dir.exists() {
            if let Ok(entries) = fs::read_dir(&self.built_in_scripts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "lua").unwrap_or(false) {
                        if let Some(info) = self.load_script_info(&path, true) {
                            cache.insert(info.name.clone(), info);
                        }
                    }
                }
            }
        }

        if self.user_scripts_dir.exists() {
            if let Ok(entries) = fs::read_dir(&self.user_scripts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "lua").unwrap_or(false) {
                        if let Some(info) = self.load_script_info(&path, false) {
                            cache.insert(info.name.clone(), info);
                        }
                    }
                }
            }
        }

        info!("Loaded {} parser scripts", cache.len());
    }

    fn load_script_info(&self, path: &PathBuf, is_built_in: bool) -> Option<ParserScriptInfo> {
        let content = fs::read_to_string(path).ok()?;

        let name = path.file_stem()?.to_string_lossy().to_string();

        let description = extract_lua_field(&content, "description")
            .unwrap_or_else(|| "No description".to_string());

        let author = extract_lua_field(&content, "author").unwrap_or_else(|| "Unknown".to_string());

        let version = extract_lua_field(&content, "version").unwrap_or_else(|| "1.0.0".to_string());

        Some(ParserScriptInfo {
            name,
            description,
            author,
            version,
            is_built_in,
            file_path: path.to_string_lossy().to_string(),
        })
    }

    pub fn get_scripts(&self) -> Vec<ParserScriptInfo> {
        let cache = self.scripts_cache.lock();
        cache.values().cloned().collect()
    }

    pub fn get_script_content(&self, name: &str) -> Result<String, String> {
        let cache = self.scripts_cache.lock();

        if let Some(info) = cache.get(name) {
            fs::read_to_string(&info.file_path).map_err(|e| format!("Failed to read script: {}", e))
        } else {
            Err(format!("Script not found: {}", name))
        }
    }

    pub fn save_script(&self, name: &str, content: &str) -> Result<(), String> {
        let file_path = self.user_scripts_dir.join(format!("{}.lua", name));

        fs::write(&file_path, content).map_err(|e| format!("Failed to write script: {}", e))?;

        info!("Saved parser script: {}", name);

        self.refresh_scripts_cache();

        Ok(())
    }

    pub fn delete_script(&self, name: &str) -> Result<(), String> {
        let cache = self.scripts_cache.lock();

        if let Some(info) = cache.get(name) {
            if info.is_built_in {
                return Err("Cannot delete built-in script".to_string());
            }

            let path = PathBuf::from(&info.file_path);
            drop(cache);

            fs::remove_file(&path).map_err(|e| format!("Failed to delete script: {}", e))?;

            info!("Deleted parser script: {}", name);

            self.refresh_scripts_cache();

            Ok(())
        } else {
            Err(format!("Script not found: {}", name))
        }
    }

    pub fn execute_script(&self, name: &str, data: &str) -> Result<HashMap<String, f64>, String> {
        let content = self.get_script_content(name)?;

        let lua = Lua::new();

        let json_lib = include_str!("../../parser_scripts/json.lua");
        if let Err(e) = lua.load(json_lib).set_name("json").exec() {
            warn!("Failed to load JSON library: {}", e);
        }

        let chunk = lua.load(&content).set_name(name);

        let parser: Value = chunk
            .eval()
            .map_err(|e| format!("Failed to execute script: {}", e))?;

        let table = parser.as_table().ok_or("Script must return a table")?;

        let parse_fn: mlua::Function = table
            .get("parse")
            .map_err(|_| "Script must have a 'parse' function")?;

        let result: Value = parse_fn
            .call(data)
            .map_err(|e| format!("Parse function failed: {}", e))?;

        let mut output = HashMap::new();

        if let Some(result_table) = result.as_table() {
            for (key, value) in result_table.clone().pairs::<String, Value>().flatten() {
                if let Some(num) = value_as_f64(&value) {
                    output.insert(key, num);
                }
            }
        }

        Ok(output)
    }

    pub fn analyze_json_structure(&self, json_content: &str) -> Result<JsonStructureInfo, String> {
        let value: serde_json::Value =
            serde_json::from_str(json_content).map_err(|e| format!("Invalid JSON: {}", e))?;

        let mut fields = Vec::new();
        let is_array = value.is_array();

        let sample_count = if is_array {
            value.as_array().map(|a| a.len() as u32).unwrap_or(0)
        } else {
            1
        };

        Self::extract_json_fields(&value, "", 0, &mut fields);

        let array_item_type = if is_array {
            value.as_array().and_then(|a| a.first()).map(|item| {
                match item {
                    serde_json::Value::Object(_) => "object",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::Bool(_) => "boolean",
                    serde_json::Value::Null => "null",
                }
                .to_string()
            })
        } else {
            None
        };

        Ok(JsonStructureInfo {
            fields,
            is_array,
            array_item_type,
            sample_count,
        })
    }

    fn extract_json_fields(
        value: &serde_json::Value,
        prefix: &str,
        depth: u32,
        fields: &mut Vec<JsonFieldInfo>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    let field_type = match val {
                        serde_json::Value::Object(_) => "object",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Null => "null",
                    };

                    let sample_value = if field_type != "object" && field_type != "array" {
                        Some(val.clone())
                    } else {
                        None
                    };

                    fields.push(JsonFieldInfo {
                        path: path.clone(),
                        name: key.clone(),
                        field_type: field_type.to_string(),
                        sample_value,
                        depth,
                    });

                    if field_type == "object" {
                        Self::extract_json_fields(val, &path, depth + 1, fields);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    if first.is_object() {
                        Self::extract_json_fields(first, &format!("{}[]", prefix), depth, fields);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn generate_parser_from_json(
        &self,
        json_content: &str,
        script_name: &str,
        selected_fields: &[String],
    ) -> Result<String, String> {
        let structure = self.analyze_json_structure(json_content)?;

        let fields_def: Vec<String> = structure
            .fields
            .iter()
            .filter(|f| selected_fields.contains(&f.path) && f.field_type == "number")
            .map(|f| format!("    {{ key = \"{}\", path = \"{}\" }}", f.name, f.path))
            .collect();

        let extract_statements: Vec<String> = structure
            .fields
            .iter()
            .filter(|f| selected_fields.contains(&f.path) && f.field_type == "number")
            .map(|f| {
                let path_parts: Vec<&str> = f.path.split('.').collect();
                let mut access = "json_obj".to_string();
                for part in &path_parts {
                    access = format!("{} and {}.\"{}\"", access, access, part);
                }
                format!("    result.{} = {}", f.name, access)
            })
            .collect();

        let script = format!(
            r#"-- Auto-generated parser script
-- Generated from JSON structure
-- Script name: {}

local parser = {{}}

parser.name = "{}"
parser.description = "Auto-generated from JSON"
parser.author = "Auto"
parser.version = "1.0.0"

parser.fields = {{
{}
}}

function parser.parse(data)
    local success, json_obj = pcall(json.decode, data)
    if not success or type(json_obj) ~= "table" then
        return nil
    end

    local result = {{}}

{}

    return result
end

function parser.validate(data)
    return data ~= nil and #data > 0
end

return parser
"#,
            script_name,
            script_name,
            fields_def.join(",\n"),
            extract_statements.join("\n")
        );

        Ok(script)
    }

    pub fn merge_json_to_parser(
        &self,
        json_content: &str,
        script_name: &str,
        selected_fields: &[String],
    ) -> Result<String, String> {
        let existing_content = self.get_script_content(script_name)?;

        let new_structure = self.analyze_json_structure(json_content)?;

        let existing_fields = Self::extract_existing_fields(&existing_content);

        let new_fields: Vec<&JsonFieldInfo> = new_structure
            .fields
            .iter()
            .filter(|f| {
                selected_fields.contains(&f.path)
                    && f.field_type == "number"
                    && !existing_fields.contains(&f.path)
            })
            .collect();

        if new_fields.is_empty() {
            return Ok(existing_content);
        }

        let new_field_defs: Vec<String> = new_fields
            .iter()
            .map(|f| format!("    {{ key = \"{}\", path = \"{}\" }}", f.name, f.path))
            .collect();

        let new_extract_statements: Vec<String> = new_fields
            .iter()
            .map(|f| {
                let path_parts: Vec<&str> = f.path.split('.').collect();
                let mut access = "json_obj".to_string();
                for part in &path_parts {
                    access = format!("{} and {}.\"{}\"", access, access, part);
                }
                format!("    result.{} = {}", f.name, access)
            })
            .collect();

        let fields_section_end = existing_content
            .find("}\n\nfunction parser.parse")
            .or_else(|| existing_content.find("}\nfunction parser.parse"))
            .ok_or("Cannot find fields section in existing script")?;

        let updated_fields =
            if existing_content[..fields_section_end].contains("parser.fields = {}") {
                existing_content.replacen(
                    "parser.fields = {}",
                    &format!("parser.fields = {{\n{}\n    }}", new_field_defs.join(",\n")),
                    1,
                )
            } else {
                let last_field_end = existing_content[..fields_section_end]
                    .rfind("}")
                    .ok_or("Cannot find last field definition")?;

                format!(
                    "{}\n{},\n{}",
                    &existing_content[..last_field_end + 1],
                    new_field_defs.join(",\n"),
                    &existing_content[last_field_end + 1..]
                )
            };

        let result_section_start = updated_fields
            .find("local result = {}")
            .ok_or("Cannot find result initialization in script")?;

        let result_section_end = updated_fields[result_section_start..]
            .find("\n\n    return result")
            .or_else(|| updated_fields[result_section_start..].find("\n    return result"))
            .map(|pos| result_section_start + pos)
            .ok_or("Cannot find return statement in script")?;

        let final_content = format!(
            "{}\n{}\n{}",
            &updated_fields[..result_section_end],
            new_extract_statements.join("\n"),
            &updated_fields[result_section_end..]
        );

        Ok(final_content)
    }

    fn extract_existing_fields(content: &str) -> Vec<String> {
        let mut fields = Vec::new();

        if let Some(fields_start) = content.find("parser.fields = {") {
            if let Some(fields_end) = content[fields_start..].find("}\n") {
                let fields_section = &content[fields_start..fields_start + fields_end];

                for line in fields_section.lines() {
                    if line.contains("path = \"") {
                        if let Some(path_start) = line.find("path = \"") {
                            let path_start = path_start + 8;
                            if let Some(path_end) = line[path_start..].find('"') {
                                fields.push(line[path_start..path_start + path_end].to_string());
                            }
                        }
                    }
                }
            }
        }

        fields
    }
}

fn extract_lua_field(content: &str, field_name: &str) -> Option<String> {
    let pattern = format!("parser.{} = \"", field_name);
    if let Some(start) = content.find(&pattern) {
        let start = start + pattern.len();
        if let Some(end) = content[start..].find('"') {
            return Some(content[start..start + end].to_string());
        }
    }
    None
}

fn value_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(i) => Some(*i as f64),
        Value::Number(n) => Some(*n),
        Value::String(s) => {
            let str = s.to_str().ok()?;
            str.parse::<f64>().ok()
        }
        _ => None,
    }
}
