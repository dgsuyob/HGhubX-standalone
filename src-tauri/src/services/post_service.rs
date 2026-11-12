//! 图文 / 帖子服务。

use crate::services::{adapter, to_json};
use serde_json::Value;

pub async fn posts(page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().posts(page, size).await?)
}

pub async fn post(id: i64) -> Result<Value, String> {
    to_json(&adapter().post(id).await?)
}

pub async fn post_playback(id: i64) -> Result<Value, String> {
    to_json(&adapter().post_playback(id).await?)
}

pub async fn post_like(id: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter().post_like(id, on).await?)
}

pub async fn post_collect(id: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter().post_collect(id, on).await?)
}

pub async fn post_comments(id: i64, page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().post_comments(id, page, size).await?)
}

pub async fn post_categories() -> Result<Value, String> {
    Ok(serde_json::json!({ "items": adapter().post_categories().await? }))
}

pub async fn post_category(route: String, page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().post_category(&route, page, size).await?)
}
