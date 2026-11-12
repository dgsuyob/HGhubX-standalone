//! 目录 / 搜索 / 榜单服务。

use crate::services::{adapter, to_json};
use serde_json::{json, Value};

pub async fn feed(page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().feed(page, size).await?)
}

pub async fn topics() -> Result<Value, String> {
    Ok(json!({ "items": adapter().topics().await? }))
}

pub async fn topic(slug: String) -> Result<Value, String> {
    to_json(&adapter().topic(&slug).await?)
}

pub async fn tags() -> Result<Value, String> {
    Ok(json!({ "categories": adapter().tags().await? }))
}

pub async fn tag_videos(tag: String, page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().tag_videos(&tag, page, size).await?)
}

pub async fn category(cate: String, page: u32, size: u32) -> Result<Value, String> {
    to_json(&adapter().category(&cate, page, size).await?)
}

pub async fn rank(kind: String) -> Result<Value, String> {
    let kind = match kind.as_str() {
        "recommend" | "potential" => kind.as_str(),
        _ => "hot",
    };
    Ok(json!({ "type": kind, "items": adapter().rank(kind).await? }))
}

pub async fn search(q: String, page: u32, size: u32, kind: String) -> Result<Value, String> {
    adapter().search(&q, page, size, &kind).await
}

pub async fn search_hot() -> Result<Value, String> {
    Ok(json!({ "items": adapter().search_hot().await? }))
}
