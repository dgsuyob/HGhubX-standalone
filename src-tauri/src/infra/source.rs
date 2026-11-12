//! Versioned user-selected libraries. Legacy site configurations remain inactive.
use crate::adapters::playlist::{Manifest, MediaEpisode, MediaItem};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::RwLock,
};
pub const SOURCE_FILE: &str = "library-v2.json";
const MAX_BYTES: usize = 2 * 1024 * 1024;

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateState {
    pub id: String,
    pub name: String,
    pub base: String,
    pub adapter: String,
    pub activated: bool,
    pub capabilities: Capabilities,
    pub allow_private_network: bool,
}
impl SourceConfig {
    fn gate(&self) -> GateState {
        GateState {
            id: self.id.clone(),
            name: self.manifest.name.clone(),
            base: self.base.clone(),
            adapter: self.adapter.clone(),
            activated: true,
            allow_private_network: self.allow_private_network,
            capabilities: Capabilities {
                catalog: true,
                search: true,
                categories: true,
                related: true,
                share: false,
                ..Capabilities::default()
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
pub fn bind(dir: &Path) {
    let path = dir.join(SOURCE_FILE);
    if let Ok(bytes) = std::fs::read(&path) {
        if bytes.len() <= MAX_BYTES * 2 {
            if let Ok(mut c) = serde_json::from_slice::<SourceConfig>(&bytes) {
                let valid = c.version == 2
                    && match c.adapter.as_str() {
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
        return Err("请输入媒体清单地址".into());
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
pub async fn gate_open(raw: &str, allow_private_network: bool) -> Result<GateState, String> {
    let url = normalize_manifest_url(raw)?;
    let mut response = crate::net::get_manifest(url.as_str(), allow_private_network).await?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let final_url = response.url().clone();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "读取清单失败")? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return Err("清单超过 2 MiB 限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let mut manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|_| "响应不是有效的媒体清单 JSON")?;
    manifest.validate(Some(&final_url))?;
    let mut identity = url.clone();
    identity.set_query(None);
    save(SourceConfig {
        version: 2,
        id: format!("url:{}", URL_SAFE_NO_PAD.encode(identity.as_str())),
        adapter: "playlist_v1".into(),
        base: url.to_string(),
        allow_private_network,
        manifest,
        local_files: HashMap::new(),
    })
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
    async fn imports_are_transactional_private_networking_is_opt_in_and_local_files_are_scoped() {
        use std::io::{Read, Write};
        let dir = std::env::temp_dir().join(format!(
            "hghubx-library-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join("source.json");
        std::fs::write(
            &legacy,
            r#"{"base":"https://example.com/old-site","adapter":"edge_v1"}"#,
        )
        .unwrap();
        bind(&dir);
        assert!(
            !gate_state().activated,
            "Legacy addresses must not activate"
        );
        let manifest=serde_json::json!({"schema_version":1,"id":"owned","name":"Owned","items":[{"id":1,"title":"Clip","episodes":[{"url":"https://media.example.com/owned.mp4"}]}]}).to_string();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!(
            "http://{}/library.json?token=one",
            listener.local_addr().unwrap()
        );
        let body = manifest.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        assert!(
            crate::net::get_manifest(&address, false).await.is_err(),
            "Private addresses require explicit permission"
        );
        let remote = gate_open(&address, true).await.unwrap();
        server.join().unwrap();
        assert!(remote.activated && remote.allow_private_network);
        assert!(!remote.capabilities.account && !remote.capabilities.comments);
        let saved = std::fs::read(dir.join(SOURCE_FILE)).unwrap();
        assert!(import_manifest("{", false).is_err());
        assert_eq!(gate_state().id, remote.id);
        assert_eq!(std::fs::read(dir.join(SOURCE_FILE)).unwrap(), saved);
        let imported = import_manifest(&manifest, false).unwrap();
        assert_ne!(remote.id, imported.id);
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
        assert!(legacy.exists(), "Migration must preserve legacy files");
        *SOURCE_PATH.write().unwrap() = None;
        std::fs::remove_dir_all(dir).unwrap();
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
}
