//! 领域 DTO：上游 JSON 只在这里被翻译一次，之后前端看到的都是这些稳定类型。

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Author {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<i64>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct VideoCard {
    pub id: i64,
    pub title: String,
    pub cover: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub score: f64,
    pub episode_count: i64,
    pub total_episodes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    pub hot: i64,
    pub like_count: i64,
    pub collect_count: i64,
    pub comment_count: i64,
    pub is_finished: bool,
    pub is_original: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ep_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Pagination {
    pub page: i64,
    pub pages: i64,
    pub size: i64,
    pub total: i64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Paged<T> {
    pub items: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Episode {
    pub ep_num: i64,
    pub title: String,
    pub duration: i64,
    pub status: i64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Counts {
    pub collect: i64,
    pub comment: i64,
    pub hot: i64,
    pub like: i64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct DetailAuthor {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_following: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like_count: Option<i64>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Interaction {
    pub collected: bool,
    pub liked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked_ep: Option<i64>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Detail {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub cover: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_links: Option<Vec<TagLink>>,
    pub score: f64,
    pub episode_count: i64,
    pub total_episodes: i64,
    pub episodes: Vec<Episode>,
    pub counts: Counts,
    pub author: DetailAuthor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<Interaction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url_hevc: Option<String>,
    pub is_finished: bool,
    pub is_original: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_type: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct TagLink {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PlayStreams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hevc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PlayInfo {
    pub id: i64,
    pub ep_num: i64,
    pub duration: i64,
    pub title: String,
    pub video_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url_hevc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streams: Option<PlayStreams>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct RankMetrics {
    pub collect: i64,
    pub comment: i64,
    pub like: i64,
    pub play: i64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct RankItem {
    pub rank: i64,
    pub video_id: i64,
    pub title: String,
    pub cover: String,
    pub tags: Vec<String>,
    pub score: f64,
    pub cate_name: String,
    pub episode_count: i64,
    pub is_finished: bool,
    pub metric_label: String,
    pub metric_value: i64,
    pub metrics: RankMetrics,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Comment {
    pub id: i64,
    pub user_id: i64,
    pub nickname: String,
    pub avatar: String,
    pub avatar_color: String,
    pub text: String,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub like_count: i64,
    pub liked: bool,
    pub reply_count: i64,
    pub reply_id: i64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reply_list: Vec<Comment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_region: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CommentPage {
    pub ep: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<bool>,
    pub items: Vec<Comment>,
    pub pagination: Pagination,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PostCounts {
    pub like: i64,
    pub comment: i64,
    pub collect: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hot: Option<i64>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PostAuthor {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub cover: String,
    pub tags: Vec<String>,
    pub hot: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<PostAuthor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<PostCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_time: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct TopicCard {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub cover: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intro: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_count: Option<i64>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct TopicDetail {
    pub topic: TopicCard,
    pub videos: Paged<VideoCard>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<TopicCard>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct TagItem {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub cate_id: i64,
    pub cate_name: String,
    pub hot_score: i64,
    pub is_hot: bool,
    pub sort: i64,
    pub trend: f64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct TagGroup {
    pub code: String,
    pub id: i64,
    pub name: String,
    pub sort: i64,
    pub tags: Vec<TagItem>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct HotWord {
    pub heat: i64,
    pub word: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PostCategory {
    pub id: i64,
    pub name: String,
    pub route: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intro: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<i64>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct LikedEpisode {
    pub video_id: i64,
    pub ep: i64,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Interactions {
    pub like_video_ids: Vec<i64>,
    pub favorite_video_ids: Vec<i64>,
    pub follow_user_ids: Vec<i64>,
    pub like_video_episodes: Vec<LikedEpisode>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub nickname: String,
    pub avatar: String,
    pub avatar_color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
    pub counts: BTreeMap<String, i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactions: Option<Interactions>,
}

pub type UserProfile = User;

#[derive(Debug, Default, Clone, Serialize)]
pub struct AuthData {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PostVideo {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PostPlayback {
    pub videos: Vec<PostVideo>,
}

/// 写操作的统一返回；上游多给什么就带什么，不给就不序列化。
#[derive(Debug, Default, Clone, Serialize)]
pub struct Ack {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<bool>,
}
