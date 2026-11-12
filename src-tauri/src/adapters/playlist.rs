//! Public media-list protocol. No site endpoints or secret keys.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u8,
    pub id: String,
    pub name: String,
    pub items: Vec<MediaItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaItem {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub author: String,
    pub episodes: Vec<MediaEpisode>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaEpisode {
    #[serde(default)]
    pub title: String,
    pub url: String,
    #[serde(default = "auto_format")]
    pub format: String,
    #[serde(default)]
    pub duration: f64,
}
fn auto_format() -> String {
    "auto".into()
}
fn validate_url(raw: &mut String, base: Option<&url::Url>, optional: bool) -> Result<(), String> {
    if optional && raw.is_empty() {
        return Ok(());
    }
    if raw.trim().is_empty() {
        return Err("媒体地址不能为空".into());
    }
    let url = match base {
        Some(base) => base.join(raw),
        None => url::Url::parse(raw),
    }
    .map_err(|_| "清单中的媒体地址无效")?;
    crate::network_policy::parse_target_with_mode(url.as_str(), false)?;
    if url.fragment().is_some() {
        return Err("媒体地址不能包含片段标识".into());
    }
    *raw = url.to_string();
    Ok(())
}
impl Manifest {
    pub fn validate(&mut self, base: Option<&url::Url>) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("清单需要 schema_version: 1".into());
        }
        if self.id.trim().is_empty()
            || self.id.len() > 128
            || self.name.trim().is_empty()
            || self.name.len() > 256
        {
            return Err("媒体库需要有效的 id 与名称".into());
        }
        if self.items.is_empty() || self.items.len() > 2000 {
            return Err("清单需要包含 1 至 2000 个条目".into());
        }
        let mut ids = HashSet::new();
        let mut total = 0;
        for item in &mut self.items {
            if item.id == 0 || item.id > 9_007_199_254_740_991 || !ids.insert(item.id) {
                return Err("条目 id 必须是唯一的 JavaScript 安全正整数".into());
            }
            if item.title.trim().is_empty()
                || item.title.len() > 512
                || item.description.len() > 20_000
                || item.author.len() > 256
                || item.tags.len() > 32
                || item.tags.iter().any(|t| t.len() > 128)
            {
                return Err("条目标题、描述或标签无效".into());
            }
            if item.episodes.is_empty() || item.episodes.len() > 1000 {
                return Err("每个条目需要 1 至 1000 个媒体文件".into());
            }
            total += item.episodes.len();
            if total > 10_000 {
                return Err("清单总集数超过限制".into());
            }
            validate_url(&mut item.cover, base, true)?;
            for ep in &mut item.episodes {
                if !["auto", "file", "hls"].contains(&ep.format.as_str())
                    || !ep.duration.is_finite()
                    || ep.duration < 0.0
                    || ep.title.len() > 512
                {
                    return Err("媒体格式、标题或时长无效".into());
                }
                validate_url(&mut ep.url, base, false)?;
            }
        }
        Ok(())
    }
    pub fn item(&self, id: i64) -> Result<&MediaItem, String> {
        self.items
            .iter()
            .find(|v| v.id == id as u64)
            .ok_or_else(|| "当前媒体库没有这个条目".into())
    }
}
pub fn card(item: &MediaItem) -> Value {
    json!({"id":item.id,"title":item.title,"cover":item.cover,"description":item.description,"tags":item.tags,
        "score":0,"episode_count":item.episodes.len(),"total_episodes":item.episodes.len(),"hot":0,"like_count":0,
        "collect_count":0,"comment_count":0,"is_finished":true,"is_original":0})
}
pub fn paged(items: Vec<&MediaItem>, page: u32, size: u32) -> Value {
    let page = page.max(1) as usize;
    let size = size.clamp(1, 100) as usize;
    let total = items.len();
    json!({"items":items.into_iter().skip(page.saturating_sub(1).saturating_mul(size)).take(size).map(card).collect::<Vec<_>>(),
        "pagination":{"page":page,"pages":total.div_ceil(size).max(1),"size":size,"total":total}})
}
pub fn detail(item: &MediaItem) -> Value {
    let mut v = card(item);
    v["episodes"] = json!(item.episodes.iter().enumerate().map(|(i,e)| json!({"ep_num":i+1,"title":if e.title.is_empty(){format!("第 {} 集",i+1)}else{e.title.clone()},"duration":e.duration,"status":1})).collect::<Vec<_>>());
    v["counts"] = json!({"collect":0,"comment":0,"hot":0,"like":0});
    v["author"] = json!({"id":0,"name":item.author,"avatar":""});
    v
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Manifest {
        serde_json::from_value(json!({"schema_version":1,"id":"mine","name":"Mine","items":[{"id":1,"title":"Clip","episodes":[{"url":"https://example.com/a.mp4"}]}]})).unwrap()
    }
    #[test]
    fn rejects_duplicates_unsafe_urls_and_versions() {
        assert!(sample().validate(None).is_ok());
        let mut v = sample();
        v.items.push(v.items[0].clone());
        assert!(v.validate(None).is_err());
        for raw in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://u:p@example.com/a.mp4",
            "data:video/mp4;base64,AAAA",
        ] {
            let mut v = sample();
            v.items[0].episodes[0].url = raw.into();
            assert!(v.validate(None).is_err(), "{raw}");
        }
        let mut v = sample();
        v.schema_version = 2;
        assert!(v.validate(None).is_err());
    }
    #[test]
    fn relative_urls_preserve_queries() {
        let mut v = sample();
        v.items[0].episodes[0].url = "../clip.mp4?key=one".into();
        v.validate(Some(
            &url::Url::parse("https://example.com/lists/mine.json?token=two").unwrap(),
        ))
        .unwrap();
        assert_eq!(
            v.items[0].episodes[0].url,
            "https://example.com/clip.mp4?key=one"
        );
    }
    #[test]
    fn paging_is_bounded() {
        let v = sample();
        assert_eq!(
            paged(v.items.iter().collect(), 0, 999)["pagination"]["size"],
            100
        );
        assert!(paged(v.items.iter().collect(), 2, 1)["items"]
            .as_array()
            .unwrap()
            .is_empty());
    }
}
