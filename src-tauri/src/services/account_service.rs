//! 账号与用户资料服务。本版不恢复登录态：token 固定为 None，
//! 需要凭据的命令由适配器统一返回「请先登录」。

use crate::services::{adapter, to_json};
use serde_json::{json, Value};

pub async fn following(page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().following(page, size).await?)
}

pub async fn user(id: i64) -> Result<Value, String> {
    to_json(&adapter().user(id).await?)
}

pub async fn user_stats(id: i64) -> Result<Value, String> {
    Ok(json!(adapter().user_stats(id).await?))
}

pub async fn user_videos(id: i64, page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().user_videos(id, page, size).await?)
}
