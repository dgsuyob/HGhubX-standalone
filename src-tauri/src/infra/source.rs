//! 门牌（数据源）配置。当前唯一激活的适配器是 edge_v1：零校验激活，
//! 填裸域名（如 huangguoai.com）即归一化成 `https://…/` 并接通 edge_v1 数据链。
//! codex 分支的 manifest/playlist 激活代码暂不调用、暂不删，收口时再决定清理。
use crate::adapters::playlist::{Manifest, MediaEpisode, MediaItem};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use dom_query::Document;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::RwLock,
};
pub const SOURCE_FILE: &str = "library-v2.json";
pub const ADAPTER_EDGE_V1: &str = "edge_v1";
const MAX_BYTES: usize = 2 * 1024 * 1024;
const DISCOVERY_PATHS: [&str; 3] = ["/.well-known/hghubx.json", "/hghubx.json", "/library.json"];
const MAX_DISCOVERY_CANDIDATES: usize = 5;
const MANIFEST_LINK_TYPE: &str = "application/vnd.hghubx.manifest+json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    pub catalog: bool,
    pub search: bool,
    pub categories: bool,
    pub related: bool,
    pub account: bool,
    pub comments: bool,
    pub posts: bool,
    pub remote_interactions: bool,
    pub share: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub version: u8,
    pub id: String,
    pub adapter: String,
    pub base: String,
    pub allow_private_network: bool,
    pub manifest: Manifest,
    #[serde(default)]
    pub local_files: HashMap<u64, PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateState {
    pub id: String,
    pub name: String,
    pub base: String,
    pub adapter: String,
    pub activated: bool,
    pub capabilities: Capabilities,
    pub allow_private_network: bool,
}
impl Default for GateState {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            base: String::new(),
            adapter: ADAPTER_EDGE_V1.to_string(),
            activated: false,
            capabilities: Capabilities::default(),
            allow_private_network: false,
        }
    }
}
impl SourceConfig {
    fn gate(&self) -> GateState {
        let edge = self.adapter == ADAPTER_EDGE_V1;
        GateState {
            id: self.id.clone(),
            name: if edge {
                String::new()
            } else {
                self.manifest.name.clone()
            },
            base: self.base.clone(),
            adapter: self.adapter.clone(),
            activated: true,
            allow_private_network: self.allow_private_network,
            capabilities: if edge {
                Capabilities {
                    catalog: true,
                    search: true,
                    categories: true,
                    related: true,
                    account: true,
                    comments: true,
                    posts: true,
                    remote_interactions: true,
                    share: true,
                }
            } else {
                Capabilities {
                    catalog: true,
                    search: true,
                    categories: true,
                    related: true,
                    share: false,
                    ..Capabilities::default()
                }
            },
        }
    }
}
static SOURCE: Lazy<RwLock<Option<SourceConfig>>> = Lazy::new(|| RwLock::new(None));
static SOURCE_PATH: Lazy<RwLock<Option<PathBuf>>> = Lazy::new(|| RwLock::new(None));
pub fn current() -> Result<SourceConfig, String> {
    SOURCE
        .read()
        .map_err(|_| "媒体库状态不可用")?
        .clone()
        .ok_or_else(|| "请先选择媒体库".into())
}
pub fn gate_state() -> GateState {
    current().map(|s| s.gate()).unwrap_or_default()
}
pub fn base() -> String {
    current().map(|s| s.base).unwrap_or_default()
}
/// 判断某个请求 origin 是否就是当前门牌（用于凭据只发给当前源）。
pub fn is_current_origin(origin: &str) -> bool {
    let current = base();
    if current.is_empty() {
        return false;
    }
    let a = url::Url::parse(origin)
        .ok()
        .map(|u| u.origin().ascii_serialization());
    let b = url::Url::parse(&current)
        .ok()
        .map(|u| u.origin().ascii_serialization());
    a.is_some() && a == b
}
pub fn bind(dir: &Path) {
    let path = dir.join(SOURCE_FILE);
    if let Ok(bytes) = std::fs::read(&path) {
        if bytes.len() <= MAX_BYTES * 2 {
            if let Ok(mut c) = serde_json::from_slice::<SourceConfig>(&bytes) {
                let valid = c.version == 2
                    && match c.adapter.as_str() {
                        ADAPTER_EDGE_V1 => !c.base.trim().is_empty(),
                        "playlist_v1" => {
                            c.local_files.is_empty() && c.manifest.validate(None).is_ok()
                        }
                        "local_files" | "demo" => {
                            c.base.is_empty()
                                && !c.local_files.is_empty()
                                && c.local_files.values().all(|p| p.is_absolute())
                        }
                        _ => false,
                    };
                if valid {
                    if let Ok(mut s) = SOURCE.write() {
                        *s = Some(c);
                    }
                }
            }
        }
    }
    if let Ok(mut p) = SOURCE_PATH.write() {
        *p = Some(path);
    }
}
fn save(c: SourceConfig) -> Result<GateState, String> {
    let mut active = SOURCE.write().map_err(|_| "媒体库状态不可用")?;
    if let Some(path) = SOURCE_PATH.read().map_err(|_| "存储不可用")?.as_ref() {
        let temp = path.with_extension("json.tmp");
        std::fs::write(
            &temp,
            serde_json::to_vec(&c).map_err(|_| "无法序列化媒体库")?,
        )
        .map_err(|_| "无法保存媒体库")?;
        std::fs::rename(temp, path).map_err(|_| "无法更新媒体库")?;
    }
    let state = c.gate();
    *active = Some(c);
    drop(active);
    crate::media::clear_caches();
    crate::net::refresh_client();
    Ok(state)
}
pub fn normalize_manifest_url(raw: &str) -> Result<url::Url, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("请输入网站或媒体清单地址".into());
    }
    let value = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let u = crate::network_policy::parse_target_with_mode(&value, false)?;
    if u.fragment().is_some() {
        return Err("清单地址不能包含片段标识".into());
    }
    Ok(u)
}

async fn read_remote_body(
    mut response: reqwest::Response,
) -> Result<(url::Url, String, Vec<u8>), String> {
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let final_url = response.url().clone();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "读取网站或清单失败")? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return Err("网站或清单超过 2 MiB 限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok((final_url, content_type, bytes))
}

fn looks_like_html(content_type: &str, bytes: &[u8]) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim() == "text/html" || value.trim() == "application/xhtml+xml")
        || matches!(
            bytes
                .strip_prefix(&[0xef, 0xbb, 0xbf])
                .unwrap_or(bytes)
                .iter()
                .find(|byte| !byte.is_ascii_whitespace()),
            Some(b'<')
        )
}

fn parse_manifest(bytes: &[u8]) -> Option<Manifest> {
    serde_json::from_slice(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes)).ok()
}

fn same_origin_url(raw: &str, base: &url::Url) -> Option<url::Url> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let candidate = base.join(raw).ok()?;
    if candidate.fragment().is_some()
        || candidate.origin() != base.origin()
        || crate::network_policy::parse_target_with_mode(candidate.as_str(), false).is_err()
    {
        return None;
    }
    Some(candidate)
}

fn declared_manifest_candidates(html: &str, page_url: &url::Url) -> Vec<url::Url> {
    let document = Document::from(html);
    let declaration_base = document
        .select("base")
        .iter()
        .find_map(|node| node.attr("href"))
        .and_then(|href| same_origin_url(&href, page_url))
        .unwrap_or_else(|| page_url.clone());
    let mut raw_candidates = Vec::new();
    for node in document.select("link").iter() {
        let rel: Vec<String> = node
            .attr("rel")
            .unwrap_or_default()
            .split_ascii_whitespace()
            .map(|value| value.to_ascii_lowercase())
            .collect();
        let media_type = node
            .attr("type")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let is_manifest_link = rel.iter().any(|value| value == "hghubx-manifest")
            || (rel.iter().any(|value| value == "alternate") && media_type == MANIFEST_LINK_TYPE);
        if is_manifest_link {
            if let Some(href) = node.attr("href") {
                raw_candidates.push(href.to_string());
            }
        }
    }
    for node in document.select("meta").iter() {
        let name = node.attr("name").unwrap_or_default();
        if name.trim().eq_ignore_ascii_case("hghubx-manifest") {
            if let Some(content) = node.attr("content") {
                raw_candidates.push(content.to_string());
            }
        }
    }
    let mut seen = HashSet::new();
    raw_candidates
        .into_iter()
        .filter_map(|raw| same_origin_url(&raw, &declaration_base))
        .filter(|candidate| seen.insert(candidate.to_string()))
        .take(MAX_DISCOVERY_CANDIDATES)
        .collect()
}

fn conventional_manifest_candidates(page_url: &url::Url) -> Vec<url::Url> {
    DISCOVERY_PATHS
        .iter()
        .filter_map(|path| page_url.join(path).ok())
        .filter(|candidate| {
            candidate.fragment().is_none()
                && candidate.origin() == page_url.origin()
                && crate::network_policy::parse_target_with_mode(candidate.as_str(), false).is_ok()
        })
        .collect()
}

fn activate_remote_manifest(
    mut manifest: Manifest,
    manifest_url: &url::Url,
    allow_private_network: bool,
) -> Result<GateState, String> {
    manifest.validate(Some(manifest_url))?;
    let mut identity = manifest_url.clone();
    identity.set_query(None);
    identity.set_fragment(None);
    save(SourceConfig {
        version: 2,
        id: format!("url:{}", URL_SAFE_NO_PAD.encode(identity.as_str())),
        adapter: "playlist_v1".into(),
        base: manifest_url.to_string(),
        allow_private_network,
        manifest,
        local_files: HashMap::new(),
    })
}

async fn discover_manifest_from_html(
    html: &str,
    page_url: &url::Url,
    allow_private_network: bool,
) -> Result<GateState, String> {
    let mut candidates = declared_manifest_candidates(html, page_url);
    let mut seen: HashSet<String> = candidates.iter().map(ToString::to_string).collect();
    for candidate in conventional_manifest_candidates(page_url) {
        if seen.insert(candidate.to_string()) {
            candidates.push(candidate);
        }
    }
    for candidate in candidates.into_iter().take(MAX_DISCOVERY_CANDIDATES) {
        let Ok(response) =
            crate::net::get_manifest(candidate.as_str(), allow_private_network).await
        else {
            continue;
        };
        let Ok((final_url, _content_type, bytes)) = read_remote_body(response).await else {
            continue;
        };
        let Some(manifest) = parse_manifest(&bytes) else {
            continue;
        };
        if let Ok(state) = activate_remote_manifest(manifest, &final_url, allow_private_network) {
            return Ok(state);
        }
    }
    Err("网站没有公开的媒体清单。请让网站声明清单，或直接输入 JSON 清单地址".into())
}

/// 零校验规整：trim → 无协议补 `https://` → 补结尾 `/`。
pub fn normalize_door(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let without_trailing = with_scheme.trim_end_matches('/');
    format!("{without_trailing}/")
}

/// 激活门牌：任何非空输入都放行，不探活、不校验，直接存 `{base, adapter:"edge_v1"}`。
/// codex 分支的 manifest/playlist 激活路径（上方各 helper）暂不调用、暂不删。
pub fn gate_open(raw: &str) -> Result<GateState, String> {
    let base = normalize_door(raw);
    if base.is_empty() {
        return Err("请输入接入地址".into());
    }
    let state = save(SourceConfig {
        version: 2,
        id: "edge-v1".into(),
        adapter: ADAPTER_EDGE_V1.into(),
        base: base.clone(),
        allow_private_network: false,
        manifest: Manifest {
            schema_version: 1,
            id: String::new(),
            name: String::new(),
            items: Vec::new(),
        },
        local_files: HashMap::new(),
    })?;
    crate::playback_log::record(
        "source.changed",
        serde_json::json!({
            "adapter": ADAPTER_EDGE_V1,
            "host": url::Url::parse(&base).ok().and_then(|u| u.host_str().map(str::to_owned)),
        }),
    );
    Ok(state)
}
pub fn import_manifest(text: &str, allow_private_network: bool) -> Result<GateState, String> {
    if text.len() > MAX_BYTES {
        return Err("清单超过 2 MiB 限制".into());
    }
    let mut manifest: Manifest =
        serde_json::from_str(text).map_err(|_| "请选择有效的媒体清单 JSON 文件")?;
    manifest.validate(None)?;
    save(SourceConfig {
        version: 2,
        id: format!("import:{}", URL_SAFE_NO_PAD.encode(&manifest.id)),
        adapter: "playlist_v1".into(),
        base: String::new(),
        allow_private_network,
        manifest,
        local_files: HashMap::new(),
    })
}
pub fn activate_files(paths: Vec<PathBuf>, demo: bool) -> Result<GateState, String> {
    if paths.is_empty() || paths.len() > 2000 {
        return Err("请选择 1 至 2000 个视频文件".into());
    }
    let mut local_files = HashMap::new();
    let mut items = Vec::new();
    for path in paths {
        let path = path
            .canonicalize()
            .map_err(|_| "视频文件不存在或无法访问")?;
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !path.is_file() || !["mp4", "m4v", "webm", "mov"].contains(&extension.as_str()) {
            return Err("本机媒体支持 MP4、M4V、WebM、MOV，实际解码能力取决于系统".into());
        }
        let mut hash = 0xcbf29ce484222325u64;
        for byte in path.to_string_lossy().as_bytes() {
            hash = (hash ^ (*byte as u64)).wrapping_mul(0x100000001b3);
        }
        let id = (hash & ((1u64 << 53) - 1)).max(1);
        if let Some(previous) = local_files.get(&id) {
            if previous == &path {
                continue;
            }
            return Err("文件标识冲突，请分批导入".into());
        }
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "本机视频".into());
        items.push(MediaItem {
            id,
            title,
            cover: String::new(),
            description: String::new(),
            tags: vec![if demo { "示例" } else { "本机" }.into()],
            author: String::new(),
            episodes: vec![MediaEpisode {
                title: "播放".into(),
                url: String::new(),
                format: "file".into(),
                duration: 0.0,
            }],
        });
        local_files.insert(id, path);
    }
    save(SourceConfig {
        version: 2,
        id: if demo { "demo-v1" } else { "local-files-v1" }.into(),
        adapter: if demo { "demo" } else { "local_files" }.into(),
        base: String::new(),
        allow_private_network: false,
        manifest: Manifest {
            schema_version: 1,
            id: "local".into(),
            name: if demo { "本地演示" } else { "本机视频" }.into(),
            items,
        },
        local_files,
    })
}
pub fn allow_private_url(raw: &str) -> bool {
    let Ok(s) = current() else {
        return false;
    };
    if !s.allow_private_network {
        return false;
    }
    let Ok(target) = url::Url::parse(raw) else {
        return false;
    };
    let origin = target.origin();
    s.manifest.items.iter().any(|i| {
        std::iter::once(i.cover.as_str())
            .chain(i.episodes.iter().map(|e| e.url.as_str()))
            .any(|v| url::Url::parse(v).is_ok_and(|u| u.origin() == origin))
    }) || url::Url::parse(&s.base).is_ok_and(|u| u.origin() == origin)
}
pub fn local_file(id: u64) -> Result<PathBuf, String> {
    SOURCE
        .read()
        .map_err(|_| "媒体库状态不可用")?
        .as_ref()
        .and_then(|s| s.local_files.get(&id))
        .cloned()
        .ok_or_else(|| "当前媒体库没有授权这个本机文件".into())
}
pub fn gate_reset() -> Result<GateState, String> {
    let mut s = SOURCE.write().map_err(|_| "媒体库状态不可用")?;
    if let Some(path) = SOURCE_PATH.read().map_err(|_| "存储不可用")?.as_ref() {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("无法移除媒体库配置".into()),
        }
    }
    *s = None;
    drop(s);
    crate::media::clear_caches();
    crate::net::refresh_client();
    Ok(GateState::default())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn edge_gate_is_zero_validation_and_imports_stay_transactional_and_scoped() {
        let dir = std::env::temp_dir().join(format!(
            "hghubx-edge-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        bind(&dir);

        // 零校验激活：任意非空输入放行，归一化成 edge_v1 门牌。
        let door = gate_open(" example.com ").unwrap();
        assert!(door.activated);
        assert_eq!(door.base, "https://example.com/");
        assert_eq!(door.adapter, ADAPTER_EDGE_V1);
        assert_eq!(gate_state().base, "https://example.com/");
        assert!(gate_open("   ").is_err());

        // 清单/本机导入保持事务性：解析失败不落盘。
        let manifest = serde_json::json!({"schema_version":1,"id":"owned","name":"Owned","items":[{"id":1,"title":"Clip","episodes":[{"url":"https://media.example.com/owned.mp4"}]}]}).to_string();
        let saved = std::fs::read(dir.join(SOURCE_FILE)).unwrap();
        assert!(import_manifest("{", false).is_err());
        assert_eq!(std::fs::read(dir.join(SOURCE_FILE)).unwrap(), saved);

        let imported = import_manifest(&manifest, false).unwrap();
        assert_eq!(imported.name, "Owned");
        let clip = dir.join("my-video.mp4");
        let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/demo.mp4"));
        std::fs::write(&clip, bytes).unwrap();
        activate_files(vec![clip], false).unwrap();
        let id = *current().unwrap().local_files.keys().next().unwrap();
        let response = crate::media::handle(
            "/local",
            &format!("id={id}"),
            crate::media::LOCAL_ORIGIN,
            Some("bytes=2-9"),
        )
        .await;
        assert_eq!(response.status().as_u16(), 206);
        assert_eq!(response.body().as_slice(), &bytes[2..10]);
        gate_reset().unwrap();
        assert!(!gate_state().activated);
        assert_eq!(
            crate::media::handle(
                "/local",
                &format!("id={id}"),
                crate::media::LOCAL_ORIGIN,
                None
            )
            .await
            .status()
            .as_u16(),
            416
        );
        *SOURCE_PATH.write().unwrap() = None;
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn door_normalization_is_zero_validation() {
        assert_eq!(normalize_door(" example.com "), "https://example.com/");
        assert_eq!(normalize_door("https://example.com"), "https://example.com/");
        assert_eq!(
            normalize_door("https://example.com/path/?x=1"),
            "https://example.com/path/?x=1/"
        );
        assert_eq!(normalize_door("不会解析的地址"), "https://不会解析的地址/");
        assert_eq!(normalize_door("   "), "");
    }

    #[test]
    fn urls_keep_path_and_query() {
        assert_eq!(
            normalize_manifest_url("https://example.com/library.json?token=abc")
                .unwrap()
                .as_str(),
            "https://example.com/library.json?token=abc"
        );
        for v in [
            "",
            "file:///etc/passwd",
            "https://name:secret@example.com/list.json",
            "https://example.com/#x",
        ] {
            assert!(normalize_manifest_url(v).is_err(), "{v}");
        }
    }

    #[test]
    fn html_declarations_are_same_origin_deduplicated_and_bounded() {
        let page = url::Url::parse("https://example.com/shows/index.html").unwrap();
        let html = r#"
            <base href="/catalog/">
            <link rel="alternate" type="application/vnd.hghubx.manifest+json" href="one.json">
            <link rel="hghubx-manifest" href="one.json">
            <link rel="hghubx-manifest" href="/two.json">
            <meta name="HGHUBX-MANIFEST" content="three.json">
            <link rel="hghubx-manifest" href="https://cdn.example.net/four.json">
            <link rel="hghubx-manifest" href="javascript:alert(1)">
            <link rel="hghubx-manifest" href="five.json">
            <link rel="hghubx-manifest" href="six.json">
        "#;
        assert_eq!(
            declared_manifest_candidates(html, &page)
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
            [
                "https://example.com/catalog/one.json",
                "https://example.com/two.json",
                "https://example.com/catalog/five.json",
                "https://example.com/catalog/six.json",
                "https://example.com/catalog/three.json",
            ]
        );
        assert_eq!(
            conventional_manifest_candidates(&page)
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
            [
                "https://example.com/.well-known/hghubx.json",
                "https://example.com/hghubx.json",
                "https://example.com/library.json",
            ]
        );
    }
}
