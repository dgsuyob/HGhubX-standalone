//! 视频详情 / 播放 / 评论 / 互动服务。

use crate::services::{adapter, to_json};
use serde_json::Value;

pub async fn video(id: i64) -> Result<Value, String> {
    to_json(&adapter().video(id).await?)
}

pub async fn play(id: i64, ep: i64) -> Result<Value, String> {
    to_json(&adapter().play(id, ep).await?)
}

pub async fn comments(id: i64, ep: i64, page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().comments(id, ep, page, size).await?)
}

pub async fn comment_send(
    id: i64,
    ep: i64,
    text: String,
    reply_id: i64,
) -> Result<Value, String> {
    to_json(&adapter().comment_send(id, ep, &text, reply_id).await?)
}

pub async fn like(id: i64, ep: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter().like(id, ep, on).await?)
}

pub async fn collect(id: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter().collect(id, on).await?)
}

pub async fn follow(uid: i64, on: bool) -> Result<Value, String> {
    to_json(&adapter().follow(uid, on).await?)
}

pub async fn related(id: i64) -> Result<Value, String> {
    Ok(serde_json::json!({ "items": adapter().related(id).await? }))
}
