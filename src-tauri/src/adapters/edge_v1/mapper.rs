//! 上游 JSON → 领域 DTO 的映射。所有容错都集中在这里。

use super::dto::*;
use serde_json::Value;
use std::collections::BTreeMap;

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key).filter(|x| !x.is_null())
}

fn pick<'a>(v: &'a Value, keys: &[&str]) -> &'a Value {
    for key in keys {
        if let Some(child) = get(v, key) {
            return child;
        }
    }
    v
}

fn text(v: &Value, keys: &[&str]) -> String {
    for key in keys {
        match get(v, key) {
            Some(Value::String(s)) => return s.clone(),
            Some(Value::Number(n)) => return n.to_string(),
            Some(Value::Bool(b)) => return b.to_string(),
            _ => {}
        }
    }
    String::new()
}

fn opt_text(v: &Value, keys: &[&str]) -> Option<String> {
    let s = text(v, keys);
    if s.is_empty() { None } else { Some(s) }
}

fn i64v(v: &Value, keys: &[&str]) -> i64 {
    for key in keys {
        match get(v, key) {
            Some(Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    return i;
                }
                if let Some(f) = n.as_f64() {
                    return f as i64;
                }
            }
            Some(Value::String(s)) => {
                if let Ok(i) = s.trim().parse::<i64>() {
                    return i;
                }
                if let Ok(f) = s.trim().parse::<f64>() {
                    return f as i64;
                }
            }
            Some(Value::Bool(b)) => return i64::from(*b),
            _ => {}
        }
    }
    0
}

fn f64v(v: &Value, keys: &[&str]) -> f64 {
    for key in keys {
        match get(v, key) {
            Some(Value::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    return f;
                }
            }
            Some(Value::String(s)) => {
                if let Ok(f) = s.trim().parse::<f64>() {
                    return f;
                }
            }
            _ => {}
        }
    }
    0.0
}

fn boolv(v: &Value, keys: &[&str]) -> bool {
    for key in keys {
        match get(v, key) {
            Some(Value::Bool(b)) => return *b,
            Some(Value::Number(n)) => return n.as_i64().unwrap_or(0) != 0,
            Some(Value::String(s)) => {
                return matches!(s.trim(), "1" | "true" | "True" | "TRUE" | "yes" | "on")
            }
            _ => {}
        }
    }
    false
}

fn strings(v: &Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(Value::Array(items)) = get(v, key) {
            return items
                .iter()
                .filter_map(|item| match item {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Object(o) => o
                        .get("name")
                        .or_else(|| o.get("title"))
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    _ => None,
                })
                .collect();
        }
    }
    Vec::new()
}

fn array<'a>(v: &'a Value, keys: &[&str]) -> &'a [Value] {
    for key in keys {
        if let Some(Value::Array(items)) = get(v, key) {
            return items;
        }
    }
    &[]
}

fn root<'a>(v: &'a Value, keys: &[&str]) -> &'a Value {
    pick(v, keys)
}

fn map_author(v: &Value) -> Author {
    Author {
        id: i64v(v, &["id", "user_id", "uid"]),
        name: text(v, &["name", "nickname", "username"]),
        avatar: opt_text(v, &["avatar", "avatar_url"]),
        followers: Some(i64v(v, &["followers", "fans", "fans_count"])),
    }
}

pub fn map_video_card(v: &Value) -> VideoCard {
    let author = get(v, "author").or_else(|| get(v, "user")).map(map_author);
    VideoCard {
        id: i64v(v, &["id", "video_id"]),
        title: text(v, &["title", "name"]),
        cover: text(v, &["cover", "cover_url", "thumb"]),
        description: opt_text(v, &["description", "desc", "intro"]),
        tags: strings(v, &["tags", "tag_list"]),
        score: f64v(v, &["score", "rating"]),
        episode_count: i64v(v, &["episode_count", "ep_count", "episodes"]),
        total_episodes: i64v(v, &["total_episodes", "total_ep", "episode_total"]),
        duration: opt_text(v, &["duration"]),
        hot: i64v(v, &["hot", "hot_count", "play_count"]),
        like_count: i64v(v, &["like_count", "likes", "like"]),
        collect_count: i64v(v, &["collect_count", "collects", "collect"]),
        comment_count: i64v(v, &["comment_count", "comments", "comment"]),
        is_finished: boolv(v, &["is_finished", "finished", "is_end"]),
        is_original: i64v(v, &["is_original", "original"]),
        created_at: opt_text(v, &["created_at", "create_time"]),
        last_ep_at: Some(i64v(v, &["last_ep_at", "last_episode_at"])),
        new_at: Some(i64v(v, &["new_at", "updated_at"])),
        author,
    }
}

pub fn map_pagination(v: &Value) -> Pagination {
    let p = get(v, "pagination").or_else(|| get(v, "meta")).unwrap_or(v);
    let page = i64v(p, &["page", "current_page", "current"]).max(1);
    let size = i64v(p, &["size", "page_size", "per_page"]).max(1);
    let total = i64v(p, &["total", "count", "total_count"]);
    let mut pages = i64v(p, &["pages", "total_pages", "last_page"]);
    if pages == 0 && total > 0 {
        pages = (total + size - 1) / size;
    }
    Pagination { page, pages, size, total }
}

pub fn map_paged_videos(v: &Value) -> Paged<VideoCard> {
    let items_value = get(v, "items")
        .or_else(|| get(v, "list"))
        .or_else(|| get(v, "videos"))
        .or_else(|| get(v, "data"));
    let items = match items_value {
        Some(Value::Array(items)) => items.iter().map(map_video_card).collect(),
        Some(Value::Object(obj)) => obj
            .get("items")
            .or_else(|| obj.get("list"))
            .or_else(|| obj.get("videos"))
            .and_then(Value::as_array)
            .map(|items| items.iter().map(map_video_card).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    Paged { items, pagination: map_pagination(v) }
}

pub fn map_topics(v: &Value) -> Vec<TopicCard> {
    array(v, &["items", "topics", "list"])
        .iter()
        .map(|item| TopicCard {
            id: i64v(item, &["id"]),
            title: text(item, &["title", "name"]),
            slug: text(item, &["slug", "route", "key"]),
            cover: text(item, &["cover", "cover_url"]),
            intro: opt_text(item, &["intro", "description", "desc"]),
            video_count: Some(i64v(item, &["video_count", "count", "videos"])),
        })
        .collect()
}

pub fn map_topic_detail(v: &Value) -> TopicDetail {
    let topic = get(v, "topic").unwrap_or(v);
    let videos_value = get(v, "videos").unwrap_or(v);
    let related = array(v, &["related", "recommend", "related_topics"])
        .iter()
        .map(|item| TopicCard {
            id: i64v(item, &["id"]),
            title: text(item, &["title", "name"]),
            slug: text(item, &["slug", "route", "key"]),
            cover: text(item, &["cover", "cover_url"]),
            intro: opt_text(item, &["intro", "description", "desc"]),
            video_count: Some(i64v(item, &["video_count", "count", "videos"])),
        })
        .collect();
    TopicDetail {
        topic: TopicCard {
            id: i64v(topic, &["id"]),
            title: text(topic, &["title", "name"]),
            slug: text(topic, &["slug", "route", "key"]),
            cover: text(topic, &["cover", "cover_url"]),
            intro: opt_text(topic, &["intro", "description", "desc"]),
            video_count: Some(i64v(topic, &["video_count", "count", "videos"])),
        },
        videos: map_paged_videos(videos_value),
        related,
    }
}

pub fn map_tag_groups(v: &Value) -> Vec<TagGroup> {
    array(v, &["categories", "groups", "items"])
        .iter()
        .map(|group| TagGroup {
            code: text(group, &["code"]),
            id: i64v(group, &["id"]),
            name: text(group, &["name", "title"]),
            sort: i64v(group, &["sort"]),
            tags: array(group, &["tags", "items"])
                .iter()
                .map(|tag| TagItem {
                    id: i64v(tag, &["id"]),
                    name: text(tag, &["name", "title"]),
                    slug: text(tag, &["slug", "route"]),
                    cate_id: i64v(tag, &["cate_id"]),
                    cate_name: text(tag, &["cate_name"]),
                    hot_score: i64v(tag, &["hot_score", "hot"]),
                    is_hot: boolv(tag, &["is_hot"]),
                    sort: i64v(tag, &["sort"]),
                    trend: f64v(tag, &["trend"]),
                })
                .collect(),
        })
        .collect()
}

pub fn map_rank_items(v: &Value) -> Vec<RankItem> {
    array(v, &["items", "ranks", "list"])
        .iter()
        .map(|item| {
            let metrics = get(item, "metrics").unwrap_or(item);
            RankItem {
                rank: i64v(item, &["rank", "position"]),
                video_id: i64v(item, &["video_id", "id"]),
                title: text(item, &["title", "name"]),
                cover: text(item, &["cover", "cover_url"]),
                tags: strings(item, &["tags"]),
                score: f64v(item, &["score"]),
                cate_name: text(item, &["cate_name", "category"]),
                episode_count: i64v(item, &["episode_count", "ep_count"]),
                is_finished: boolv(item, &["is_finished", "finished"]),
                metric_label: text(item, &["metric_label", "label"]),
                metric_value: i64v(item, &["metric_value", "value"]),
                metrics: RankMetrics {
                    collect: i64v(metrics, &["collect", "collect_count"]),
                    comment: i64v(metrics, &["comment", "comment_count"]),
                    like: i64v(metrics, &["like", "like_count"]),
                    play: i64v(metrics, &["play", "play_count", "hot"]),
                },
            }
        })
        .collect()
}

pub fn map_hot_words(v: &Value) -> Vec<HotWord> {
    array(v, &["items", "words", "list"])
        .iter()
        .map(|item| HotWord {
            heat: i64v(item, &["heat", "hot", "count"]),
            word: text(item, &["word", "name", "keyword"]),
        })
        .collect()
}

pub fn map_post_categories(v: &Value) -> Vec<PostCategory> {
    array(v, &["items", "categories", "list"])
        .iter()
        .map(|item| PostCategory {
            id: i64v(item, &["id"]),
            name: text(item, &["name", "title"]),
            route: text(item, &["route", "slug"]),
            intro: opt_text(item, &["intro", "description"]),
            sort: Some(i64v(item, &["sort"])),
        })
        .collect()
}

pub fn map_post(v: &Value) -> Post {
    let author = get(v, "author").or_else(|| get(v, "user")).map(|u| PostAuthor {
        id: i64v(u, &["id", "user_id"]),
        name: text(u, &["name", "nickname", "username"]),
        avatar: opt_text(u, &["avatar", "avatar_url"]),
    });
    let counts = get(v, "counts").or_else(|| get(v, "stats")).map(|c| PostCounts {
        like: i64v(c, &["like", "like_count"]),
        comment: i64v(c, &["comment", "comment_count"]),
        collect: i64v(c, &["collect", "collect_count"]),
        share: Some(i64v(c, &["share", "share_count"])),
        hot: Some(i64v(c, &["hot", "hot_count"])),
    });
    Post {
        id: i64v(v, &["id", "post_id"]),
        title: text(v, &["title", "name"]),
        desc: opt_text(v, &["desc", "description"]),
        content: opt_text(v, &["content", "body"]),
        cover: text(v, &["cover", "cover_url"]),
        tags: strings(v, &["tags"]),
        hot: i64v(v, &["hot", "hot_count"]),
        video_url: opt_text(v, &["video_url", "url"]),
        video_id: get(v, "video_id")
            .and_then(Value::as_i64)
            .or_else(|| get(v, "video").map(|x| i64v(x, &["id"]))),
        author,
        counts,
        created_at: opt_text(v, &["created_at", "create_time"]),
        publish_time: opt_text(v, &["publish_time", "published_at"]),
    }
}

pub fn map_posts(v: &Value) -> Paged<Post> {
    let items = array(v, &["items", "posts", "list"])
        .iter()
        .map(map_post)
        .collect();
    Paged { items, pagination: map_pagination(v) }
}

pub fn map_post_playback(v: &Value) -> PostPlayback {
    let videos = array(v, &["videos", "items", "playlist"])
        .iter()
        .filter_map(|item| match item {
            Value::String(url) => Some(PostVideo { url: url.clone(), ..Default::default() }),
            Value::Object(_) => Some(PostVideo {
                url: text(item, &["url", "video_url", "src"]),
                title: opt_text(item, &["title", "name"]),
                duration: Some(i64v(item, &["duration"])),
                cover: opt_text(item, &["cover", "cover_url"]),
            }),
            _ => None,
        })
        .filter(|video| !video.url.is_empty())
        .collect();
    PostPlayback { videos }
}

pub fn map_comment(v: &Value) -> Comment {
    Comment {
        id: i64v(v, &["id", "comment_id"]),
        user_id: i64v(v, &["user_id", "uid"]),
        nickname: text(v, &["nickname", "name", "username"]),
        avatar: text(v, &["avatar", "avatar_url"]),
        avatar_color: text(v, &["avatar_color", "color"]),
        text: text(v, &["text", "content"]),
        time: text(v, &["time", "created_at", "create_time"]),
        created_at: opt_text(v, &["created_at", "create_time"]),
        like_count: i64v(v, &["like_count", "likes"]),
        liked: boolv(v, &["liked"]),
        reply_count: i64v(v, &["reply_count", "replies"]),
        reply_id: i64v(v, &["reply_id"]),
        reply_list: array(v, &["reply_list", "replies"])
            .iter()
            .map(map_comment)
            .collect(),
        is_pinned: Some(boolv(v, &["is_pinned", "pinned"])),
        ip_region: opt_text(v, &["ip_region", "region"]),
    }
}

pub fn map_comment_page(v: &Value) -> CommentPage {
    CommentPage {
        ep: i64v(v, &["ep", "ep_num"]),
        revision: opt_text(v, &["revision"]),
        reset: Some(boolv(v, &["reset"])),
        items: array(v, &["items", "comments", "list"])
            .iter()
            .map(map_comment)
            .collect(),
        pagination: map_pagination(v),
    }
}

pub fn map_detail(v: &Value) -> Detail {
    let detail = root(v, &["video", "detail"]);
    let counts = get(detail, "counts").or_else(|| get(detail, "stats"));
    let author = get(detail, "author").or_else(|| get(detail, "user"));
    let interaction = get(detail, "interaction").or_else(|| get(detail, "user_interaction"));
    Detail {
        id: i64v(detail, &["id", "video_id"]),
        title: text(detail, &["title", "name"]),
        description: text(detail, &["description", "desc", "intro"]),
        cover: text(detail, &["cover", "cover_url"]),
        tags: strings(detail, &["tags"]),
        tag_links: get(detail, "tag_links").and_then(Value::as_array).map(|items| {
            items
                .iter()
                .map(|item| TagLink {
                    name: text(item, &["name"]),
                    url: text(item, &["url"]),
                })
                .collect()
        }),
        score: f64v(detail, &["score", "rating"]),
        episode_count: i64v(detail, &["episode_count", "ep_count"]),
        total_episodes: i64v(detail, &["total_episodes", "total_ep"]),
        episodes: array(detail, &["episodes", "episode_list"])
            .iter()
            .map(|ep| Episode {
                ep_num: i64v(ep, &["ep_num", "ep", "num"]),
                title: text(ep, &["title", "name"]),
                duration: i64v(ep, &["duration"]),
                status: i64v(ep, &["status"]),
            })
            .collect(),
        counts: Counts {
            collect: counts.map(|c| i64v(c, &["collect", "collect_count"])).unwrap_or(0),
            comment: counts.map(|c| i64v(c, &["comment", "comment_count"])).unwrap_or(0),
            hot: counts.map(|c| i64v(c, &["hot", "hot_count"])).unwrap_or(0),
            like: counts.map(|c| i64v(c, &["like", "like_count"])).unwrap_or(0),
        },
        author: DetailAuthor {
            id: author.map(|a| i64v(a, &["id", "user_id"])).unwrap_or(0),
            name: author.map(|a| text(a, &["name", "nickname", "username"])).unwrap_or_default(),
            avatar: author.and_then(|a| opt_text(a, &["avatar", "avatar_url"])),
            followers: author.map(|a| i64v(a, &["followers", "fans"])),
            is_following: author.map(|a| boolv(a, &["is_following", "following"])),
            following: author.map(|a| i64v(a, &["following"])),
            like_count: author.map(|a| i64v(a, &["like_count", "likes"])),
        },
        interaction: interaction.map(|i| Interaction {
            collected: boolv(i, &["collected"]),
            liked: boolv(i, &["liked"]),
            liked_ep: get(i, "liked_ep").and_then(Value::as_i64),
        }),
        video_url: opt_text(detail, &["video_url"]),
        video_url_hevc: opt_text(detail, &["video_url_hevc"]),
        is_finished: boolv(detail, &["is_finished", "finished"]),
        is_original: i64v(detail, &["is_original", "original"]),
        video_type: get(detail, "video_type").and_then(Value::as_i64),
        duration: get(detail, "duration").and_then(Value::as_i64),
        created_at: opt_text(detail, &["created_at", "create_time"]),
    }
}

pub fn map_play_info(v: &Value) -> PlayInfo {
    let info = root(v, &["play", "info", "playback"]);
    let streams = get(info, "streams").or_else(|| get(v, "streams"));
    let stream_main = streams.and_then(|s| opt_text(s, &["main", "video_url"]));
    let stream_hevc = streams.and_then(|s| opt_text(s, &["hevc", "video_url_hevc"]));
    PlayInfo {
        id: i64v(info, &["id", "video_id"]),
        ep_num: i64v(info, &["ep_num", "ep"]),
        duration: i64v(info, &["duration"]),
        title: text(info, &["title", "name"]),
        video_url: opt_text(info, &["video_url"]).or(stream_main.clone()).unwrap_or_default(),
        video_url_hevc: opt_text(info, &["video_url_hevc"]).or(stream_hevc.clone()),
        streams: streams.map(|_| PlayStreams {
            main: stream_main,
            hevc: stream_hevc,
            preview: streams.and_then(|s| opt_text(s, &["preview"])),
        }),
    }
}

fn map_counts(v: &Value) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    let source = get(v, "counts").or_else(|| get(v, "stats")).unwrap_or(v);
    for (key, value) in source.as_object().into_iter().flatten() {
        if let Some(n) = value.as_i64() {
            out.insert(key.clone(), n);
        }
    }
    for key in [
        "like_count", "following", "followers", "video_count", "post_count", "fans", "likes",
    ] {
        if let Some(n) = get(v, key).and_then(Value::as_i64) {
            out.entry(key.to_string()).or_insert(n);
        }
    }
    out
}

pub fn map_user(v: &Value) -> User {
    let user = root(v, &["user", "profile"]);
    User {
        id: i64v(user, &["id", "user_id", "uid"]),
        username: text(user, &["username", "name"]),
        nickname: text(user, &["nickname", "name", "username"]),
        avatar: text(user, &["avatar", "avatar_url"]),
        avatar_color: text(user, &["avatar_color", "color"]),
        sign: opt_text(user, &["sign", "signature"]),
        desc: opt_text(user, &["desc", "description", "intro"]),
        user_type: opt_text(user, &["user_type", "type"]),
        counts: map_counts(user),
        interactions: get(user, "interactions").map(|i| Interactions {
            like_video_ids: array(i, &["like_video_ids"])
                .iter()
                .filter_map(Value::as_i64)
                .collect(),
            favorite_video_ids: array(i, &["favorite_video_ids", "collect_video_ids"])
                .iter()
                .filter_map(Value::as_i64)
                .collect(),
            follow_user_ids: array(i, &["follow_user_ids"])
                .iter()
                .filter_map(Value::as_i64)
                .collect(),
            like_video_episodes: array(i, &["like_video_episodes"])
                .iter()
                .map(|ep| LikedEpisode {
                    video_id: i64v(ep, &["video_id"]),
                    ep: i64v(ep, &["ep", "ep_num"]),
                })
                .collect(),
        }),
    }
}

pub fn map_auth(v: &Value) -> AuthData {
    let data = root(v, &["data"]);
    AuthData {
        token: text(data, &["token", "access_token"]),
        user: map_user(get(data, "user").unwrap_or(data)),
    }
}

pub fn map_ack(v: &Value) -> Ack {
    Ack {
        ok: true,
        id: get(v, "id").and_then(Value::as_i64),
        count: get(v, "count").and_then(Value::as_i64),
        liked: get(v, "liked").and_then(Value::as_bool),
        collected: get(v, "collected").and_then(Value::as_bool),
        following: get(v, "following").and_then(Value::as_bool),
    }
}

pub fn map_user_stats(v: &Value) -> BTreeMap<String, i64> {
    map_counts(v)
}
