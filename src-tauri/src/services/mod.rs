//! Source-independent read-only media operations.
use crate::{adapters::playlist, domain::DomainCommand, infra::source};
use serde_json::{json, Value};
pub async fn dispatch(command: DomainCommand) -> Result<Value, String> {
    if matches!(command, DomainCommand::Boot) {
        return Ok(
            json!({"base":source::base(),"gate":source::gate_state(),"user":null,"logged_in":false,"strict_security":true,"cli_relaxed":false}),
        );
    }
    let source = source::current()?;
    let list = &source.manifest;
    match command {
        DomainCommand::Feed { page, size } => {
            Ok(playlist::paged(list.items.iter().collect(), page, size))
        }
        DomainCommand::Video { id } => Ok(playlist::detail(list.item(id)?)),
        DomainCommand::Play { id, ep } => {
            let item = list.item(id)?;
            let e = item
                .episodes
                .get(ep.checked_sub(1).ok_or("无效集数")? as usize)
                .ok_or("没有这一集")?;
            let mut v = json!({"id":id,"ep_num":ep,"title":e.title,"duration":e.duration});
            if let Some(path) = source.local_files.get(&(id as u64)) {
                if !path.is_file() {
                    return Err("本机视频已移动或删除，请重新导入".into());
                }
                let address = format!("{}/local?id={id}", crate::media::LOCAL_ORIGIN);
                v["video_url"] = json!(address);
                v["streams"] = json!({"main":address});
            } else {
                let hls = e.format == "hls"
                    || (e.format == "auto"
                        && url::Url::parse(&e.url)
                            .is_ok_and(|u| u.path().to_ascii_lowercase().ends_with(".m3u8")));
                let route = if hls { "hls" } else { "stream" };
                let enc: String = url::form_urlencoded::byte_serialize(e.url.as_bytes()).collect();
                let address = format!("{}/{route}?u={enc}", crate::media::LOCAL_ORIGIN);
                v["video_url"] = json!(address);
                v["streams"] = json!({"main":address});
            }
            Ok(v)
        }
        DomainCommand::Search {
            q,
            page,
            size,
            kind,
        } => {
            if kind == "post" {
                return Err("当前媒体库不提供帖子".into());
            }
            let query = q.to_lowercase();
            let items = list
                .items
                .iter()
                .filter(|i| {
                    i.title.to_lowercase().contains(&query)
                        || i.description.to_lowercase().contains(&query)
                        || i.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .collect();
            let mut result = playlist::paged(items, page, size);
            result["keyword"] = json!(q);
            Ok(result)
        }
        DomainCommand::Tags => {
            let mut tags = std::collections::BTreeSet::new();
            for i in &list.items {
                for t in &i.tags {
                    tags.insert(t.clone());
                }
            }
            Ok(
                json!({"categories":[{"name":"媒体标签","tags":tags.into_iter().enumerate().map(|(i,n)|json!({"id":i+1,"name":n,"slug":n})).collect::<Vec<_>>()}]}),
            )
        }
        DomainCommand::Category { cate, page, size }
        | DomainCommand::TagVideos {
            tag: cate,
            page,
            size,
        } => Ok(playlist::paged(
            list.items
                .iter()
                .filter(|i| i.tags.contains(&cate))
                .collect(),
            page,
            size,
        )),
        DomainCommand::Related { id } => Ok(
            json!({"items":list.items.iter().filter(|i|i.id!=id as u64).take(12).map(playlist::card).collect::<Vec<_>>()}),
        ),
        DomainCommand::SearchHot => Ok(json!({"items":[]})),
        DomainCommand::Logout => Ok(json!({"ok":true})),
        _ => Err("当前媒体库不提供此能力".into()),
    }
}
