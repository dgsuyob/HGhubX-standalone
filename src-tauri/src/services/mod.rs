//! 业务服务：只编排领域命令，所有上游细节交给适配器。
//!
//! 当前只启用 edge_v1 一个适配器（与 1.2.19 行为一致）；codex 分支原有的
//! manifest/playlist 代码暂不调用、暂不删，收口时再决定清理。

mod account_service;
mod catalog_service;
mod playback_service;
mod post_service;

use crate::adapters::edge_v1::EdgeV1Adapter;
use crate::domain::DomainCommand;
use crate::infra::source;
use serde_json::{json, Value};

pub(crate) fn adapter() -> EdgeV1Adapter {
    // 登录态暂不恢复：token 固定为 None，登录相关命令统一返回「请先登录」。
    EdgeV1Adapter::new(source::base(), None)
}

pub(crate) fn to_json<T: serde::Serialize>(value: &T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

/// `data_call` 的总入口：领域命令 → edge_v1 service → EdgeV1Adapter。
pub async fn dispatch(command: DomainCommand) -> Result<Value, String> {
    match command {
        DomainCommand::Boot => Ok(json!({
            "base": source::base(),
            "gate": source::gate_state(),
            "user": null,
            "logged_in": false,
            "strict_security": crate::network_policy::strict_security(),
            "cli_relaxed": false,
        })),
        // 本版不恢复登录态：账号相关入口先给出明确提示，后续版本再接回。
        DomainCommand::Login { .. }
        | DomainCommand::Register { .. }
        | DomainCommand::Logout
        | DomainCommand::Me => Err("请先登录".into()),
        DomainCommand::Feed { page, size } => catalog_service::feed(page, size).await,
        DomainCommand::Topics => catalog_service::topics().await,
        DomainCommand::Topic { slug } => catalog_service::topic(slug).await,
        DomainCommand::Tags => catalog_service::tags().await,
        DomainCommand::TagVideos { tag, page, size } => {
            catalog_service::tag_videos(tag, page, size).await
        }
        DomainCommand::Category { cate, page, size } => {
            catalog_service::category(cate, page, size).await
        }
        DomainCommand::Rank { kind } => catalog_service::rank(kind).await,
        DomainCommand::Search { q, page, size, kind } => {
            catalog_service::search(q, page, size, kind).await
        }
        DomainCommand::SearchHot => catalog_service::search_hot().await,
        DomainCommand::Posts { page, size } => post_service::posts(page, size).await,
        DomainCommand::Post { id } => post_service::post(id).await,
        DomainCommand::PostPlayback { id } => post_service::post_playback(id).await,
        DomainCommand::PostLike { id, on } => post_service::post_like(id, on).await,
        DomainCommand::PostCollect { id, on } => post_service::post_collect(id, on).await,
        DomainCommand::PostComments { id, page, size } => {
            post_service::post_comments(id, page, size).await
        }
        DomainCommand::PostCategories => post_service::post_categories().await,
        DomainCommand::PostCategory { route, page, size } => {
            post_service::post_category(route, page, size).await
        }
        DomainCommand::Video { id } => playback_service::video(id).await,
        DomainCommand::Play { id, ep } => playback_service::play(id, ep).await,
        DomainCommand::Comments { id, ep, page, size } => {
            playback_service::comments(id, ep, page, size).await
        }
        DomainCommand::CommentSend {
            id,
            ep,
            text,
            reply_id,
        } => playback_service::comment_send(id, ep, text, reply_id).await,
        DomainCommand::Like { id, ep, on } => playback_service::like(id, ep, on).await,
        DomainCommand::Collect { id, on } => playback_service::collect(id, on).await,
        DomainCommand::Follow { uid, on } => playback_service::follow(uid, on).await,
        DomainCommand::Related { id } => playback_service::related(id).await,
        DomainCommand::Following { page, size } => account_service::following(page, size).await,
        DomainCommand::User { id } => account_service::user(id).await,
        DomainCommand::UserStats { id } => account_service::user_stats(id).await,
        DomainCommand::UserVideos { id, page, size } => {
            account_service::user_videos(id, page, size).await
        }
    }
}
