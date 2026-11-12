//! Edge V1 适配器：把领域请求翻译成上游信封，再把响应翻译成领域 DTO。

mod client;
pub mod dto;
mod mapper;

pub use client::EdgeV1Client;
pub use dto::*;

use crate::ports::SourcePort;
use serde_json::json;

pub struct EdgeV1Adapter {
    client: EdgeV1Client,
}

fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

impl EdgeV1Adapter {
    pub fn new(origin: String, token: Option<String>) -> Self {
        Self {
            client: EdgeV1Client::new(origin, token),
        }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<AuthData, String> {
        let body = json!({ "username": username, "password": password });
        let data = self
            .client
            .post("/api/account/loginByPassword", Some(&body))
            .await?;
        Ok(mapper::map_auth(&data))
    }

    pub async fn register(
        &self,
        username: &str,
        password: &str,
        confirm_password: &str,
        email: Option<&str>,
    ) -> Result<AuthData, String> {
        let mut body = json!({
            "username": username,
            "password": password,
            "confirm_password": confirm_password,
        });
        if let Some(email) = email.filter(|value| !value.trim().is_empty()) {
            body["email"] = json!(email);
        }
        let data = self
            .client
            .post("/api/account/registerByPassword", Some(&body))
            .await?;
        Ok(mapper::map_auth(&data))
    }

    pub async fn logout(&self) -> Result<Ack, String> {
        self.client.require_token()?;
        let data = self.client.post("/api/account/logout", None).await?;
        Ok(mapper::map_ack(&data))
    }

    pub async fn me(&self) -> Result<User, String> {
        self.client.require_token()?;
        let data = self.client.get("/api/account/me").await?;
        Ok(mapper::map_user(&data))
    }

    pub async fn feed(&self, page: u32, size: u32) -> Result<Paged<VideoCard>, String> {
        let data = self
            .client
            .get(&format!("/api/videos?page={page}&size={size}"))
            .await?;
        Ok(mapper::map_paged_videos(&data))
    }

    pub async fn topics(&self) -> Result<Vec<TopicCard>, String> {
        let data = self.client.get("/api/topics?size=100").await?;
        Ok(mapper::map_topics(&data))
    }

    pub async fn topic(&self, slug: &str) -> Result<TopicDetail, String> {
        let data = self
            .client
            .get(&format!("/api/topics/{}", encode(slug)))
            .await?;
        Ok(mapper::map_topic_detail(&data))
    }

    pub async fn tags(&self) -> Result<Vec<TagGroup>, String> {
        let data = self.client.get("/api/tags").await?;
        Ok(mapper::map_tag_groups(&data))
    }

    pub async fn tag_videos(
        &self,
        tag: &str,
        page: u32,
        size: u32,
    ) -> Result<Paged<VideoCard>, String> {
        let data = self
            .client
            .get(&format!(
                "/api/videos/tag/{}?page={page}&size={size}",
                encode(tag)
            ))
            .await?;
        Ok(mapper::map_paged_videos(&data))
    }

    pub async fn category(
        &self,
        cate: &str,
        page: u32,
        size: u32,
    ) -> Result<Paged<VideoCard>, String> {
        let data = self
            .client
            .get(&format!(
                "/api/videos/category/{}?page={page}&size={size}",
                encode(cate)
            ))
            .await?;
        Ok(mapper::map_paged_videos(&data))
    }

    pub async fn rank(&self, kind: &str) -> Result<Vec<RankItem>, String> {
        let kind = match kind {
            "recommend" | "potential" => kind,
            _ => "hot",
        };
        let data = self.client.get(&format!("/api/ranks/{kind}")).await?;
        Ok(mapper::map_rank_items(&data))
    }

    pub async fn search(
        &self,
        query: &str,
        page: u32,
        size: u32,
        kind: &str,
    ) -> Result<serde_json::Value, String> {
        let scope = match kind {
            "post" => "&type=post",
            "video" => "&type=video",
            _ => "",
        };
        let data = self
            .client
            .get(&format!(
                "/api/search?q={}&page={page}&size={size}{scope}",
                encode(query)
            ))
            .await?;
        if kind == "post" {
            Ok(serde_json::to_value(mapper::map_posts(&data)).map_err(|e| e.to_string())?)
        } else {
            Ok(serde_json::to_value(mapper::map_paged_videos(&data)).map_err(|e| e.to_string())?)
        }
    }

    pub async fn search_hot(&self) -> Result<Vec<HotWord>, String> {
        let data = self.client.get("/api/search/hot").await?;
        Ok(mapper::map_hot_words(&data))
    }

    pub async fn posts(&self, page: u32, size: u32) -> Result<Paged<Post>, String> {
        let data = self
            .client
            .get(&format!("/api/posts?page={page}&size={size}"))
            .await?;
        Ok(mapper::map_posts(&data))
    }

    pub async fn post(&self, id: i64) -> Result<Post, String> {
        let data = self.client.get(&format!("/api/posts/{id}")).await?;
        Ok(mapper::map_post(&data))
    }

    pub async fn post_playback(&self, id: i64) -> Result<PostPlayback, String> {
        let data = self
            .client
            .post(&format!("/api/posts/{id}/playback"), Some(&json!({})))
            .await?;
        Ok(mapper::map_post_playback(&data))
    }

    pub async fn post_like(&self, id: i64, on: bool) -> Result<Ack, String> {
        self.client.require_token()?;
        let path = format!("/api/posts/{id}/like");
        let data = if on {
            self.client.post(&path, None).await?
        } else {
            self.client.delete(&path).await?
        };
        Ok(mapper::map_ack(&data))
    }

    pub async fn post_collect(&self, id: i64, on: bool) -> Result<Ack, String> {
        self.client.require_token()?;
        let path = format!("/api/posts/{id}/collect");
        let data = if on {
            self.client.post(&path, None).await?
        } else {
            self.client.delete(&path).await?
        };
        Ok(mapper::map_ack(&data))
    }

    pub async fn post_comments(
        &self,
        id: i64,
        page: u32,
        size: u32,
    ) -> Result<Paged<Comment>, String> {
        let data = self
            .client
            .get(&format!("/api/posts/{id}/comments?page={page}&size={size}"))
            .await?;
        let items = mapper::map_comment_page(&data);
        Ok(Paged {
            items: items.items,
            pagination: items.pagination,
        })
    }

    pub async fn post_categories(&self) -> Result<Vec<PostCategory>, String> {
        let data = self.client.get("/api/site/categories/post").await?;
        Ok(mapper::map_post_categories(&data))
    }

    pub async fn post_category(
        &self,
        route: &str,
        page: u32,
        size: u32,
    ) -> Result<Paged<Post>, String> {
        let data = self
            .client
            .get(&format!(
                "/api/posts/category/{}?page={page}&size={size}",
                encode(route)
            ))
            .await?;
        Ok(mapper::map_posts(&data))
    }

    pub async fn video(&self, id: i64) -> Result<Detail, String> {
        let data = self.client.get(&format!("/api/videos/{id}")).await?;
        Ok(mapper::map_detail(&data))
    }

    pub async fn play(&self, id: i64, ep: i64) -> Result<PlayInfo, String> {
        let body = json!({});
        let primary = self
            .client
            .post(&format!("/api/videos/{id}/episodes/{ep}/playback"), Some(&body))
            .await
            .and_then(|data| {
                let info = mapper::map_play_info(&data);
                if info.video_url.is_empty() {
                    Err("播放接口未返回地址".to_string())
                } else {
                    Ok(info)
                }
            });
        match primary {
            Ok(info) => Ok(info),
            Err(_) => {
                let data = self
                    .client
                    .get(&format!("/api/videos/{id}/play?ep={ep}&page_texts=0"))
                    .await?;
                Ok(mapper::map_play_info(&data))
            }
        }
    }

    pub async fn comments(
        &self,
        id: i64,
        ep: i64,
        page: u32,
        size: u32,
    ) -> Result<CommentPage, String> {
        let data = self
            .client
            .get(&format!(
                "/api/videos/{id}/comments?ep={ep}&page={page}&size={size}"
            ))
            .await?;
        Ok(mapper::map_comment_page(&data))
    }

    pub async fn comment_send(
        &self,
        id: i64,
        ep: i64,
        text: &str,
        reply_id: i64,
    ) -> Result<Ack, String> {
        self.client.require_token()?;
        let body = if reply_id > 0 {
            json!({ "text": text, "reply_id": reply_id })
        } else {
            json!({ "text": text })
        };
        let path = if reply_id > 0 {
            format!("/api/videos/{id}/comments")
        } else {
            format!("/api/videos/{id}/comments?ep={ep}")
        };
        let data = self.client.post(&path, Some(&body)).await?;
        Ok(mapper::map_ack(&data))
    }

    pub async fn like(&self, id: i64, ep: i64, on: bool) -> Result<Ack, String> {
        self.client.require_token()?;
        let path = format!("/api/videos/{id}/like?ep={ep}");
        let data = if on {
            self.client.post(&path, None).await?
        } else {
            self.client.delete(&path).await?
        };
        Ok(mapper::map_ack(&data))
    }

    pub async fn collect(&self, id: i64, on: bool) -> Result<Ack, String> {
        self.client.require_token()?;
        let path = format!("/api/videos/{id}/collect");
        let data = if on {
            self.client.post(&path, None).await?
        } else {
            self.client.delete(&path).await?
        };
        Ok(mapper::map_ack(&data))
    }

    pub async fn follow(&self, uid: i64, on: bool) -> Result<Ack, String> {
        self.client.require_token()?;
        let path = format!("/api/users/{uid}/follow");
        let data = if on {
            self.client.post(&path, None).await?
        } else {
            self.client.delete(&path).await?
        };
        Ok(mapper::map_ack(&data))
    }

    pub async fn related(&self, id: i64) -> Result<Vec<VideoCard>, String> {
        let data = self.client.get(&format!("/api/videos/{id}/related")).await?;
        Ok(mapper::map_paged_videos(&data).items)
    }

    pub async fn following(&self, page: u32, size: u32) -> Result<Paged<VideoCard>, String> {
        self.client.require_token()?;
        let data = self
            .client
            .get(&format!("/api/videos/following?page={page}&size={size}"))
            .await?;
        Ok(mapper::map_paged_videos(&data))
    }

    pub async fn user(&self, id: i64) -> Result<UserProfile, String> {
        let data = self.client.get(&format!("/api/users/{id}")).await?;
        Ok(mapper::map_user(&data))
    }

    pub async fn user_stats(&self, id: i64) -> Result<std::collections::BTreeMap<String, i64>, String> {
        let data = self.client.get(&format!("/api/users/{id}/stats")).await?;
        let mut stats = mapper::map_user_stats(&data);
        stats.insert("user_id".to_string(), id);
        Ok(stats)
    }

    pub async fn user_videos(
        &self,
        id: i64,
        page: u32,
        size: u32,
    ) -> Result<Paged<VideoCard>, String> {
        let data = self
            .client
            .get(&format!("/api/videos/user/{id}?page={page}&size={size}"))
            .await?;
        Ok(mapper::map_paged_videos(&data))
    }
}

impl SourcePort for EdgeV1Adapter {
    fn adapter(&self) -> &'static str {
        "edge_v1"
    }

    fn origin(&self) -> &str {
        self.client.origin()
    }
}
