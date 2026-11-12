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
    // Android's resource directory lives inside the APK. Materialize the embedded
    // clip in app-owned storage so the same local range server works everywhere.
    const DEMO: &[u8] = include_bytes!("../../resources/demo.mp4");
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "无法定位演示存储目录")?
        .join("demo");
    std::fs::create_dir_all(&directory).map_err(|_| "无法创建演示存储目录")?;
    let path = directory.join("HGhubX-demo.mp4");
    if std::fs::read(&path).ok().as_deref() != Some(DEMO) {
        let temporary = directory.join("HGhubX-demo.tmp");
        std::fs::write(&temporary, DEMO).map_err(|_| "无法保存演示视频")?;
        std::fs::rename(temporary, &path).map_err(|_| "无法更新演示视频")?;
    }
    let state = source::activate_files(vec![path], true)?;
    Ok(state)
}
