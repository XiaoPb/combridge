use std::sync::Arc;
use tauri::State;
use super::parser_scripts::{ParserScriptManager, ParserScriptInfo, JsonStructureInfo, FieldDefinition};

pub type ParserScriptManagerRef = Arc<ParserScriptManager>;

#[tauri::command]
pub async fn get_parser_scripts(
    manager: State<'_, ParserScriptManagerRef>,
) -> Result<Vec<ParserScriptInfo>, String> {
    Ok(manager.get_scripts())
}

#[tauri::command]
pub async fn get_parser_script_content(
    manager: State<'_, ParserScriptManagerRef>,
    name: String,
) -> Result<String, String> {
    manager.get_script_content(&name)
}

#[tauri::command]
pub async fn save_parser_script(
    manager: State<'_, ParserScriptManagerRef>,
    name: String,
    content: String,
) -> Result<(), String> {
    manager.save_script(&name, &content)
}

#[tauri::command]
pub async fn delete_parser_script(
    manager: State<'_, ParserScriptManagerRef>,
    name: String,
) -> Result<(), String> {
    manager.delete_script(&name)
}

#[tauri::command]
pub async fn execute_parser_script(
    manager: State<'_, ParserScriptManagerRef>,
    name: String,
    data: String,
) -> Result<std::collections::HashMap<String, f64>, String> {
    manager.execute_script(&name, &data)
}

#[tauri::command]
pub async fn init_default_parser_scripts(
    manager: State<'_, ParserScriptManagerRef>,
) -> Result<(), String> {
    manager.init_default_scripts()
}

#[tauri::command]
pub async fn analyze_json_structure(
    manager: State<'_, ParserScriptManagerRef>,
    json_content: String,
) -> Result<JsonStructureInfo, String> {
    manager.analyze_json_structure(&json_content)
}

#[tauri::command]
pub async fn generate_parser_from_json(
    manager: State<'_, ParserScriptManagerRef>,
    json_content: String,
    script_name: String,
    selected_fields: Vec<String>,
) -> Result<String, String> {
    manager.generate_parser_from_json(&json_content, &script_name, &selected_fields)
}

#[tauri::command]
pub async fn get_parser_defined_fields(
    _manager: State<'_, ParserScriptManagerRef>,
    _script_name: String,
) -> Result<Vec<FieldDefinition>, String> {
    Ok(vec![])
}

#[tauri::command]
pub async fn merge_json_to_parser(
    manager: State<'_, ParserScriptManagerRef>,
    json_content: String,
    script_name: String,
    selected_fields: Vec<String>,
) -> Result<String, String> {
    manager.merge_json_to_parser(&json_content, &script_name, &selected_fields)
}

pub fn create_parser_script_manager(app_data_dir: std::path::PathBuf) -> ParserScriptManagerRef {
    Arc::new(ParserScriptManager::new(app_data_dir))
}
