use crate::update::{DownloadResult, UpdateAsset, UpdateInfo, UpdateService};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn check_for_update(
    service: State<'_, UpdateService>,
) -> Result<Option<UpdateInfo>, String> {
    service.check_for_update().await
}

#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    service: State<'_, UpdateService>,
    asset: UpdateAsset,
) -> Result<DownloadResult, String> {
    let app = app.clone();
    service
        .download_update(&asset, move |progress| {
            let _ = app.emit("update-download-progress", progress);
        })
        .await
}

#[tauri::command]
pub async fn open_update_installer(
    service: State<'_, UpdateService>,
    path: String,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(path);
    service.validate_cached_path(&path)?;
    open::that(&path).map_err(|e| format!("failed to open installer: {e}"))
}
