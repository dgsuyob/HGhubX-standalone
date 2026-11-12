use crate::infra::source::{self, GateState};
use tauri::Manager;
#[tauri::command]
pub async fn gate_open(
    address: String,
    allow_private_network: Option<bool>,
) -> Result<GateState, String> {
    source::gate_open(&address, allow_private_network.unwrap_or(false)).await
}
#[tauri::command]
pub async fn gate_state() -> Result<GateState, String> {
    Ok(source::gate_state())
}
#[tauri::command]
pub async fn gate_reset() -> Result<GateState, String> {
    source::gate_reset()
}
#[tauri::command]
pub async fn source_import_manifest(
    text: String,
    allow_private_network: Option<bool>,
) -> Result<GateState, String> {
    source::import_manifest(&text, allow_private_network.unwrap_or(false))
}
#[tauri::command]
pub async fn source_reload() -> Result<GateState, String> {
    let s = source::current()?;
    if s.base.is_empty() {
        Ok(source::gate_state())
    } else {
        source::gate_open(&s.base, s.allow_private_network).await
    }
}
#[tauri::command]
pub async fn source_open_files(app: tauri::AppHandle) -> Result<Option<GateState>, String> {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        use tauri_plugin_dialog::DialogExt;
        let picker = app.clone();
        let selection = tauri::async_runtime::spawn_blocking(move || {
            picker
                .dialog()
                .file()
                .add_filter("视频文件", &["mp4", "m4v", "webm", "mov"])
                .blocking_pick_files()
        })
        .await
        .map_err(|_| "文件选择器不可用")?;
        let Some(files) = selection else {
            return Ok(None);
        };
        let paths = files
            .into_iter()
            .map(|f| f.into_path().map_err(|_| "无法读取文件路径".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let state = source::activate_files(paths, false)?;
        Ok(Some(state))
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = app;
        Err("此平台请通过媒体清单连接视频，本机视频选择暂仅支持桌面端".into())
    }
}
#[tauri::command]
pub async fn source_open_demo(app: tauri::AppHandle) -> Result<GateState, String> {
    let path = app
        .path()
        .resource_dir()
        .map_err(|_| "无法定位演示资源")?
        .join("resources/demo.mp4");
    let state = source::activate_files(vec![path], true)?;
    Ok(state)
}
