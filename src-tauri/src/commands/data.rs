//! 统一数据入口：前端只调用 `data_call`，壳层按领域命令分发。

use crate::domain::DomainCommand;
use crate::services;
use serde_json::Value;

#[tauri::command]
pub async fn data_call(command: DomainCommand) -> Result<Value, String> {
    services::dispatch(command).await
}
