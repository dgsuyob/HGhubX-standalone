//! Validated HTTP transport for user-selected media; no site gateway.
use crate::network_policy;
use once_cell::sync::Lazy;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 默认上游：门牌化后没有内置源，必须由用户输入地址激活。
pub const DEFAULT_BASE: &str = "";

/// 当前门牌基址（不带结尾斜杠）。唯一来源是 [`crate::infra::source`]。
pub fn base() -> String {
    crate::infra::source::base()
}

/// Bound decoded bodies too: compressed/chunked responses cannot grow without limit.
pub async fn bounded_body(mut response: reqwest::Response, max: usize) -> Result<Vec<u8>, String> {
    if response.content_length().is_some_and(|n| n > max as u64) {
        return Err("响应超过大小限制".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| err_text(&e))? {
        if bytes.len() + chunk.len() > max {
            return Err("响应超过大小限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub async fn get_range(raw: &str, range: &str) -> Result<reqwest::Response, String> {
    get_with_dns(raw, Duration::from_secs(30), None, None, false, Some(range)).await
}

/// The initial connection precedes activation. Private networking is an explicit
/// user choice and, during import, redirects must stay on the selected origin.
pub async fn get_manifest(raw: &str, allow_private: bool) -> Result<reqwest::Response, String> {
    let initial = network_policy::parse_target_with_mode(raw, !allow_private)?;
    let mut next = initial.clone();
    for hop in 0..=5 {
        let addresses = network_policy::resolve_target(&next, !allow_private).await?;
        let client = build_client(next.host_str().ok_or("目标缺少主机名")?, &addresses)?;
        let response = client
            .get(next.clone())
            .timeout(Duration::from_secs(20))
            .header("Accept", "application/json, text/html;q=0.9")
            .send()
            .await
            .map_err(|e| err_text(&e))?;
        if !response.status().is_redirection() {
            return Ok(response);
        }
        if hop == 5 {
            return Err("重定向次数超过限制".into());
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|h| h.to_str().ok())
            .ok_or("重定向缺少 Location")?;
        let candidate = network_policy::redirect_target_with_mode(&next, location, !allow_private)?;
        if next.scheme() == "https" && candidate.scheme() != "https" {
            return Err("禁止 HTTPS 降级重定向".into());
        }
        if allow_private && candidate.origin() != initial.origin() {
            return Err("局域网清单重定向必须保持同一来源".into());
        }
        next = candidate;
    }
    Err("重定向次数超过限制".into())
}

pub const UA: &str = concat!("HGhubX/", env!("CARGO_PKG_VERSION"));

/// 手动指定代理（排查 / 公司内网用）：`HGHUBX_PROXY=http://127.0.0.1:7897`。
/// 不设时交给 reqwest 的 `system-proxy` 探测（注册表 + 环境变量）。
fn proxy_override() -> Option<String> {
    std::env::var("HGHUBX_PROXY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 按**当前**系统代理设置新建一个客户端。
///
/// 注意：reqwest 的 `system-proxy` 只在构建时读一次注册表 / 环境变量。代理软件被关掉后，
/// 旧客户端仍会往 `127.0.0.1:7897` 建隧道，症状是满屏 `tunnel error: failed to create
/// underlying connection`（浏览器却是通的，因为它每次都重读系统代理）。所以这里允许重建。
fn build_client(host: &str, addrs: &[SocketAddr]) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .referer(false)
        .gzip(true)
        .brotli(true)
        .pool_idle_timeout(Duration::from_secs(60));
    if !addrs.is_empty() {
        builder = builder.resolve_to_addrs(host, addrs);
    }
    if let Some(p) = proxy_override() {
        // 错误信息不能含原始代理 URL（其中可能有用户名/密码）。
        let proxy = reqwest::Proxy::all(&p).map_err(|_| "HGHUBX_PROXY 配置无效（已隐藏原始值）")?;
        builder = builder.proxy(proxy);
    }
    builder
        .build()
        .map_err(|_| "无法创建 HTTP 客户端".to_string())
}

const CLIENT_TTL: Duration = Duration::from_secs(30);
type ClientCache = HashMap<String, (Instant, Arc<reqwest::Client>)>;
static CLIENTS: Lazy<Mutex<ClientCache>> = Lazy::new(|| Mutex::new(HashMap::new()));
static MEDIA_DNS_ROUTES: Lazy<Mutex<crate::media_dns::Routes>> =
    Lazy::new(|| Mutex::new(crate::media_dns::Routes::default()));

fn cached_client(cache: &ClientCache, key: &str, now: Instant) -> Option<Arc<reqwest::Client>> {
    cache
        .get(key)
        .filter(|(expires, _)| now < *expires)
        .map(|(_, http)| http.clone())
}

/// 缓存按模式、主机和端口隔离；有效期内仅复用已经检查并固定地址的客户端。
/// 到期后重新检查全部 DNS 答案，不使用旧连接绕过新的目标检查。
async fn client_for(raw: &str) -> Result<(url::Url, Arc<reqwest::Client>), String> {
    client_for_dns(raw, false).await
}

async fn client_for_dns(
    raw: &str,
    public_dns: bool,
) -> Result<(url::Url, Arc<reqwest::Client>), String> {
    let strict = network_policy::strict_security() && !crate::infra::source::allow_private_url(raw);
    let target = network_policy::parse_target_with_mode(raw, strict)?;
    let host = target.host_str().ok_or("目标缺少主机名")?;
    let port = target.port_or_known_default().ok_or("目标端口无效")?;
    let key = format!("{strict}:{host}:{port}:{public_dns}");
    {
        let cache = CLIENTS.lock().map_err(|_| "HTTP 客户端锁失败")?;
        if let Some(http) = cached_client(&cache, &key, Instant::now()) {
            return Ok((target, http));
        }
    }
    let (addrs, ttl) = if public_dns {
        let resolver = build_client(crate::media_dns::HOST, &crate::media_dns::BOOTSTRAP)?;
        let resolution = crate::media_dns::resolve(&resolver, host, port).await?;
        network_policy::check_addresses_with_mode(&resolution.addresses, strict)?;
        (resolution.addresses, CLIENT_TTL.min(resolution.ttl))
    } else if strict {
        (
            network_policy::resolve_target(&target, strict).await?,
            CLIENT_TTL,
        )
    } else {
        (Vec::new(), CLIENT_TTL)
    };
    let http = Arc::new(build_client(host, &addrs)?);
    let now = Instant::now();
    let mut cache = CLIENTS.lock().map_err(|_| "HTTP 客户端锁失败")?;
    // 同时到达的请求可能已完成解析；不延长已有结果的有效期。
    let http = cached_client(&cache, &key, now).unwrap_or(http);
    if !cache.get(&key).is_some_and(|(expires, _)| now < *expires) {
        cache.retain(|_, (expires, _)| now < *expires);
        if cache.len() >= 64 {
            cache.clear();
        }
        cache.insert(key, (now + ttl, http.clone()));
    }
    Ok((target, http))
}

pub fn refresh_client() {
    if let Ok(mut cache) = CLIENTS.lock() {
        cache.clear();
    }
    if let Ok(mut routes) = MEDIA_DNS_ROUTES.lock() {
        *routes = crate::media_dns::Routes::default();
    }
}

/// 不可信 GET 的统一入口；自动重定向被关闭，每一跳重新检查 URL、DNS 并固定解析地址。
pub async fn get(
    raw: &str,
    timeout: Duration,
    accept: Option<&str>,
    referer: Option<&str>,
) -> Result<reqwest::Response, String> {
    get_with_dns(raw, timeout, accept, referer, false, None).await
}

async fn get_with_dns(
    raw: &str,
    timeout: Duration,
    accept: Option<&str>,
    referer: Option<&str>,
    public_dns: bool,
    range: Option<&str>,
) -> Result<reqwest::Response, String> {
    tokio::time::timeout(timeout, async {
        let policy_error = |phase: &str, error: String| {
            crate::playback_log::record("network.target_failed", json!({"resource": crate::playback_log::resource_id(raw), "phase": phase, "error_kind": crate::playback_log::error_kind(&error)}));
            error
        };
        let mut next = network_policy::parse_target_with_mode(raw, network_policy::strict_security() && !crate::infra::source::allow_private_url(raw)).map_err(|e| policy_error("parse", e))?;
        for hop in 0..=5 {
            let resolving = Instant::now();
            let (target, http) = client_for_dns(next.as_str(), public_dns).await.map_err(|e| policy_error("dns_or_client", e))?;
            crate::playback_log::record("network.target_ready", json!({"resource": crate::playback_log::resource_id(raw), "target_resource": crate::playback_log::resource_id(target.as_str()), "dns_or_client_ms": resolving.elapsed().as_millis() as u64, "hop": hop}));
            let mut request = http.get(target.clone()).timeout(timeout);
            if let Some(value) = accept {
                request = request.header("Accept", value);
            }
            if let Some(value) = referer {
                request = request.header("Referer", value);
            }
            if let Some(value) = range { request = request.header("Range", value); }
            let response = request.send().await.map_err(|e| err_text(&e))?;
            if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                return Ok(response);
            }
            if hop == 5 {
                return Err("重定向次数超过限制".into());
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .ok_or("重定向缺少有效 Location")?;
            let candidate = target.join(location).map_err(|_| "重定向地址无效")?;
            if target.scheme() == "https" && candidate.scheme() != "https" { return Err("禁止 HTTPS 降级重定向".into()); }
            next = network_policy::redirect_target_with_mode(&target, location,
                network_policy::strict_security() && !crate::infra::source::allow_private_url(candidate.as_str()))
                .map_err(|e| policy_error("redirect", e))?;
            crate::playback_log::record("network.redirect", json!({"resource": crate::playback_log::resource_id(raw), "from": crate::playback_log::resource_id(target.as_str()), "to": crate::playback_log::resource_id(next.as_str()), "status": response.status().as_u16(), "hop": hop + 1}));
        }
        Err("重定向次数超过限制".into())
    })
    .await
    .map_err(|_| "请求超时".to_string())?
}

/// 只返回代理的协议/主机/端口；解析失败时隐藏整个值，绝不回传认证或查询参数。
pub fn redact_proxy(raw: &str) -> String {
    let parsed = url::Url::parse(raw.trim())
        .ok()
        .filter(|u| u.host_str().is_some());
    match parsed {
        Some(url) => {
            let mut safe = format!("{}://{}", url.scheme(), url.host_str().unwrap());
            if let Some(port) = url.port() {
                safe.push_str(&format!(":{port}"));
            }
            safe
        }
        None => "[已配置，原始值已隐藏]".into(),
    }
}

/// 看起来像代理 / 隧道问题（而不是目标站点本身不可达）。
pub fn is_proxy_error(e: &str) -> bool {
    let e = e.to_ascii_lowercase();
    e.contains("tunnel") || e.contains("proxy")
}

static LAST_HEAL: Lazy<std::sync::Mutex<Option<Instant>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

/// 请求失败后按需重建客户端：代理隧道错误、连接被拒 / 超时都算「网络环境可能变了」。
/// 3 秒内只重建一次，批量探测不会反复重建。
pub fn heal_client(reason: &str) {
    let connectish = reason.contains("Connect")
        || reason.contains("timed out")
        || reason.contains("refused")
        || reason.contains("dns");
    if !is_proxy_error(reason) && !connectish {
        return;
    }
    let mut last = match LAST_HEAL.lock() {
        Ok(l) => l,
        Err(_) => return,
    };
    if let Some(t) = *last {
        if t.elapsed() < Duration::from_secs(3) {
            return;
        }
    }
    *last = Some(Instant::now());
    drop(last);
    refresh_client();
}

/// reqwest 的 `Display` 只有「error sending request for url (...)」，
/// 真正的原因（dns error / connection refused / invalid peer certificate）
/// 藏在 `source()` 链里。展开成一行，界面和 debug.log 才有排查价值。
pub fn err_text(e: &reqwest::Error) -> String {
    let mut out = e.to_string();
    let mut src = std::error::Error::source(e);
    while let Some(s) = src {
        let t = s.to_string();
        if !t.is_empty() && !out.contains(&t) {
            out.push_str(" ← ");
            out.push_str(&t);
        }
        src = s.source();
    }
    out
}

#[cfg(test)]
fn dns_summary(addrs: impl IntoIterator<Item = SocketAddr>) -> Value {
    let mut ips: Vec<_> = addrs.into_iter().map(|addr| addr.ip()).collect();
    ips.sort_unstable();
    ips.dedup();
    let has_fake_ip = ips.iter().any(|ip| {
        let v4 = match ip {
            std::net::IpAddr::V4(v4) => Some(*v4),
            std::net::IpAddr::V6(v6) => v6.to_ipv4_mapped(),
        };
        v4.is_some_and(|ip| {
            let [a, b, _, _] = ip.octets();
            a == 198 && (b == 18 || b == 19)
        })
    });
    json!({ "count": ips.len(), "has_fake_ip": has_fake_ip })
}

/// IPv4 地址分类。诊断要能回答「解析到的是正常公网，还是被投毒/被接管」。
fn ipv4_kind(o: [u8; 4]) -> &'static str {
    let v4 = std::net::Ipv4Addr::from(o);
    if v4.is_loopback() {
        "loopback"
    } else if o == [0, 0, 0, 0] {
        "unspecified"
    } else if o[0] == 198 && (o[1] == 18 || o[1] == 19) {
        // Clash / Surge 系 TUN 的 fake-ip 网段：TUN 一关这个地址就不可路由。
        "fake_ip"
    } else if v4.is_private() {
        "private"
    } else if v4.is_link_local() {
        "link_local"
    } else if v4.is_broadcast() {
        "broadcast"
    } else {
        "public"
    }
}

fn ip_kind(ip: std::net::IpAddr) -> &'static str {
    match ip {
        std::net::IpAddr::V4(v4) => ipv4_kind(v4.octets()),
        std::net::IpAddr::V6(v6) => {
            if v6.is_loopback() {
                "loopback"
            } else if v6.is_unspecified() {
                "unspecified"
            } else if let Some(v4) = v6.to_ipv4_mapped() {
                ipv4_kind(v4.octets())
            } else {
                "public"
            }
        }
    }
}

/// DNS 报告：数量 / 耗时 / 地址种类 / 是否 fake-ip。
///
/// **只把可疑地址原样列出**（回环、内网、保留段、fake-ip），公网答案只给数量——
/// 贴到公开 issue 里也不必泄露完整解析结果，但「被投毒到 127.0.0.1」这类
/// 一眼就能看见。
fn dns_report(addrs: &[SocketAddr], ms: u128) -> Value {
    let mut ips: Vec<std::net::IpAddr> = addrs.iter().map(|a| a.ip()).collect();
    ips.sort_unstable();
    ips.dedup();
    let mut kinds: Vec<&str> = ips.iter().map(|ip| ip_kind(*ip)).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let suspicious: Vec<String> = ips
        .iter()
        .filter(|ip| ip_kind(**ip) != "public")
        .map(|ip| ip.to_string())
        .collect();
    json!({
        "ms": ms as u64,
        "count": ips.len(),
        "has_fake_ip": kinds.contains(&"fake_ip"),
        "kinds": kinds,
        "suspicious": suspicious,
    })
}

/// 拿到解析结果后直连一次，用来区分「DNS 能解析但 SYN 被黑洞」和「HTTP 层出错」。
async fn tcp_probe(addr: Option<SocketAddr>, timeout: Duration) -> Value {
    let Some(addr) = addr else {
        return json!({ "skipped": true, "reason": "没有可用的解析结果" });
    };
    let started = Instant::now();
    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr)).await {
        Ok(Ok(_)) => json!({
            "addr": addr.to_string(),
            "ok": true,
            "ms": started.elapsed().as_millis() as u64,
        }),
        Ok(Err(e)) => json!({
            "addr": addr.to_string(),
            "ok": false,
            "ms": started.elapsed().as_millis() as u64,
            "error": e.to_string(),
        }),
        Err(_) => json!({
            "addr": addr.to_string(),
            "ok": false,
            "timeout": true,
            "ms": timeout.as_millis() as u64,
            "error": "连接超时（SYN 无响应）",
        }),
    }
}

/// 把三组探测结果压成一句人话结论 + 可执行的建议，issue 里一眼能看懂。
fn verdict(dns: &Value, tcp: &Value, http: &Value) -> (&'static str, String, Vec<String>) {
    let dns_err = dns.get("error").and_then(|v| v.as_str());
    let fake = dns
        .get("has_fake_ip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let kinds = dns
        .get("kinds")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let suspicious = dns
        .get("suspicious")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let status = http.get("status").and_then(|v| v.as_u64());
    let http_err = http.get("error").and_then(|v| v.as_str()).unwrap_or("");
    let tcp_ok = tcp.get("ok").and_then(|v| v.as_bool());

    // 系统 resolver 报错、但真实请求能通：诊断别把人带偏。
    if dns_err.is_some() && status.is_some() {
        return (
            "dns_false_alarm",
            "DNS 探测自身失败，但 HTTP 实际可以连通（系统 resolver 异常或被代理接管），不影响使用"
                .into(),
            vec![],
        );
    }
    if let Some(e) = dns_err {
        return (
            "dns_fail",
            format!("域名解析失败：{e}"),
            vec![
                "关掉代理 / 加速器 / VPN 的 TUN 模式后重试（最常见的元凶）".into(),
                "换网络对比：Wi-Fi 与流量互切".into(),
                "安卓「私有 DNS」改为自动，或填 223.5.5.5 / 1.1.1.1".into(),
            ],
        );
    }
    if fake {
        // 代理正常工作时 HTTP 也是通的，这种情况不能报故障，否则每个开 Clash 的用户都会被吓一跳。
        let http_ok = status.is_some_and(|c| (200..400).contains(&c));
        if http_ok {
            return (
                "ok_proxy",
                "DNS 被代理/TUN 接管（fake-ip），但连接正常，可继续使用".into(),
                vec!["若要排查网络问题，请先退出代理再诊断一次".into()],
            );
        }
        return (
            "fake_ip",
            format!(
                "解析到 fake-ip（{}），DNS 流量被代理/TUN 接管；代理一断开这个地址不可路由",
                suspicious
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("198.18.x.x")
            ),
            vec![
                "关掉 Clash / Surge 等代理的 TUN（增强）模式，或直接退出代理".into(),
                "若必须用代理，请保证代理常驻且节点稳定后再开 App".into(),
            ],
        );
    }
    if kinds
        .iter()
        .any(|k| matches!(k.as_str(), Some("loopback" | "unspecified" | "private")))
    {
        return (
            "dns_hijack",
            "解析结果落在回环 / 内网 / 保留地址，DNS 被劫持或线路被污染".into(),
            vec![
                "换一个 DNS（223.5.5.5、1.1.1.1）后重启 App".into(),
                "关闭省电模式或系统的「网络加速」类功能".into(),
            ],
        );
    }
    if tcp_ok == Some(false) {
        let timeout = tcp
            .get("timeout")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        return (
            if timeout {
                "tcp_timeout"
            } else {
                "tcp_refused"
            },
            if timeout {
                "DNS 正常，但 TCP 连接无响应（SYN 被丢弃）：运营商或防火墙阻断".to_string()
            } else {
                format!(
                    "DNS 正常，但 TCP 连接被拒绝：{}",
                    tcp.get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("未知原因")
                )
            },
            vec![
                "换网络（尤其换运营商 / 关掉网络加速器）后重试".into(),
                "确认没有防火墙或家长控制拦截该域名".into(),
            ],
        );
    }
    if !http_err.is_empty() {
        let lower = http_err.to_lowercase();
        let tls = lower.contains("certificate")
            || lower.contains("tls")
            || lower.contains("ssl")
            || lower.contains("handshake");
        return (
            if tls { "tls_intercept" } else { "http_fail" },
            if tls {
                "TLS 握手/证书校验失败：多半是代理做了中间人，或系统时间不对".to_string()
            } else {
                format!("HTTP 请求失败：{http_err}")
            },
            vec![
                "关闭代理的「HTTPS 解密 / 中间人」功能".into(),
                "检查系统时间是否准确".into(),
            ],
        );
    }
    match status {
        Some(code) if (200..400).contains(&code) => {
            ("ok", format!("连通正常（HTTP {code}），线路可用"), vec![])
        }
        Some(code) => (
            "http_status",
            format!("域名与网络都通，但服务端返回 HTTP {code}"),
            vec!["稍后重试，或换一条线路".into()],
        ),
        None => ("unknown", "诊断信息不足".into(), vec![]),
    }
}

/// 网络自检：DNS / TCP / HTTP / 代理环境变量，用于远程定位「打不开」。
///
/// 结论直接给在 `verdict` / `summary` / `advice` 里，用户复制出去就能看懂。
pub async fn diagnose() -> Value {
    let cur = base();
    if cur.is_empty() {
        return json!({"summary":"本机媒体库无需网络连接","verdict":"local_library"});
    }
    let port = url::Url::parse(&cur)
        .ok()
        .and_then(|u| u.port_or_known_default())
        .unwrap_or(443);
    let host = url::Url::parse(&cur)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default();

    // DNS：必须和真实请求走同一条解析路径（tokio lookup_host），
    // 否则会出现「诊断报解析失败、实际却打得开」的假阴性。
    let dns_started = Instant::now();
    let mut first_addr: Option<SocketAddr> = None;
    let dns = if host.is_empty() {
        json!({ "error": "当前线路没有主机名" })
    } else {
        match tokio::time::timeout(
            Duration::from_secs(6),
            tokio::net::lookup_host((host.as_str(), port)),
        )
        .await
        {
            Ok(Ok(iter)) => {
                let addrs: Vec<SocketAddr> = iter.collect();
                let ms = dns_started.elapsed().as_millis();
                first_addr = addrs.first().copied();
                dns_report(&addrs, ms)
            }
            Ok(Err(e)) => json!({
                "error": "DNS 解析失败",
                "detail": e.to_string(),
                "ms": dns_started.elapsed().as_millis() as u64,
            }),
            Err(_) => json!({
                "error": "DNS 解析超时",
                "timeout": true,
                "ms": 6000,
            }),
        }
    };

    // TCP：拿到地址就直连一次，把「DNS 通但连不上」和「HTTP 层出错」分开。
    let tcp = tcp_probe(first_addr, Duration::from_secs(5)).await;

    let started = Instant::now();
    let http = match get(&cur, Duration::from_secs(20), None, None).await {
        Ok(r) => json!({
            "status": r.status().as_u16(),
            "ms": started.elapsed().as_millis() as u64,
        }),
        Err(e) => json!({
            "kind": crate::playback_log::error_kind(&e),
            "error": e,
            "ms": started.elapsed().as_millis() as u64,
        }),
    };

    let (code, summary, advice) = verdict(&dns, &tcp, &http);

    json!({
        "verdict": code,
        "summary": summary,
        "advice": advice,
        "base": cur,
        "host": host,
        "dns": dns,
        "tcp": tcp,
        "http": http,
        "env": {
            "HTTP_PROXY": std::env::var("HTTP_PROXY").ok().map(|p| redact_proxy(&p)),
            "HTTPS_PROXY": std::env::var("HTTPS_PROXY").ok().map(|p| redact_proxy(&p)),
            "ALL_PROXY": std::env::var("ALL_PROXY").ok().map(|p| redact_proxy(&p)),
            "NO_PROXY": std::env::var("NO_PROXY").ok().map(|_| "[已配置，列表已隐藏]"),
            "HGHUBX_PROXY": proxy_override().map(|p| redact_proxy(&p)),
        },
        "runtime": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "ver": env!("CARGO_PKG_VERSION"),
        },
    })
}

fn retryable_error(error: &str) -> bool {
    if error == "401" {
        return false;
    }
    if let Some(status) = error
        .strip_prefix("HTTP ")
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u16>().ok())
    {
        return matches!(status, 408 | 425 | 429 | 500..=599);
    }
    !matches!(
        crate::playback_log::error_kind(error),
        "https_downgrade_denied"
            | "private_address_denied"
            | "target_policy_denied"
            | "redirect_error"
    )
}

/// 只重试可能恢复的错误；404/权限错误/安全拒绝直接结束，避免无效等待。
/// 站点偶发 TLS 抖动；碰上代理隧道错误就先重建客户端再试
/// （代理软件开关后旧客户端会一直卡在 tunnel error 上）。
async fn with_retry<F, Fut, T>(retry: bool, mut f: F) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut last = String::from("network error");
    let attempts = if retry { 3 } else { 1 };
    for attempt in 0..attempts {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                last = e;
                if !retryable_error(&last) {
                    return Err(last);
                }
                heal_client(&last);
                if attempt + 1 < attempts {
                    tokio::time::sleep(Duration::from_millis(350 * (attempt as u64 + 1))).await;
                }
            }
        }
    }
    Err(last)
}

// 只记录固定的操作类型，不写搜索词、账号、内容 ID 或查询参数。
fn edge_operation(path: &str) -> &'static str {
    let path = path.split('?').next().unwrap_or("");
    if path.starts_with("/api/videos/category/") {
        "category"
    } else if path.starts_with("/api/videos/") && path.ends_with("/play") {
        "play"
    } else if path == "/api/videos" {
        "feed"
    } else if path.starts_with("/api/videos/") {
        "video"
    } else if path == "/api/posts" {
        "posts"
    } else if path.starts_with("/api/posts/") {
        "post"
    } else if path.starts_with("/api/ranks/") {
        "rank"
    } else if path == "/api/search" {
        "search"
    } else if path.starts_with("/api/account/") {
        "account"
    } else {
        "other"
    }
}

/// 仅无请求体、无凭据的逻辑 GET 可以迁移网关；信封本身仍使用 POST。
fn edge_redirect_target(
    current: &url::Url,
    status: u16,
    location: &str,
    guest_get: bool,
) -> Result<url::Url, String> {
    if !guest_get || !matches!(status, 301 | 302 | 303) {
        return Err("业务接口重定向已拒绝".into());
    }
    network_policy::redirect_target(current, location)
}

/// 业务请求：兼容旧调用方，内部拼标准信封。
pub async fn edge(
    origin: &str,
    method: &str,
    path: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> Result<Value, String> {
    let body_is_none = body.is_none();
    let (encoded, ctype) = match &body {
        Some(v) => (
            B64.encode(serde_json::to_vec(v).map_err(|e| e.to_string())?),
            "application/json",
        ),
        None => (String::new(), ""),
    };
    let envelope = json!({
        "method": method,
        "path": path,
        "body": encoded,
        "content_type": ctype,
    });
    edge_envelope(origin, envelope, method, path, body_is_none, token).await
}

/// 适配器已拼好信封、只需传输的入口；`body_is_none` 决定是否允许安全重试。
pub async fn edge_envelope(
    origin: &str,
    envelope: Value,
    method: &str,
    path: &str,
    body_is_none: bool,
    token: Option<&str>,
) -> Result<Value, String> {
    let cur = origin.trim().trim_end_matches('/').to_string();
    if cur.is_empty() {
        return Err("尚未设置接入地址".into());
    }
    if network_policy::strict_security()
        && (token.is_some()
            || matches!(
                path,
                "/api/account/loginByPassword" | "/api/account/registerByPassword"
            ))
        && !crate::infra::source::is_current_origin(&cur)
    {
        return Err("凭据只能发送到当前接入地址".into());
    }

    let url = format!("{cur}/api/edge/request");
    let referer = format!("{cur}/");
    let token = token.map(|s| s.to_string());

    static NEXT_EDGE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let request = NEXT_EDGE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let line_host = url::Url::parse(&cur)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_default();
    let operation = edge_operation(path);
    let mut attempt = 0;
    let guest_get = method.eq_ignore_ascii_case("GET") && body_is_none && token.is_none();
    let can_retry = method.eq_ignore_ascii_case("GET") && body_is_none;
    let value = with_retry(can_retry, || {
        attempt += 1;
        let attempt = attempt;
        let envelope = envelope.clone();
        let token = token.clone();
        let url = url.clone();
        let referer = referer.clone();
        let line_host = line_host.clone();
        async move {
            let started = Instant::now();
            crate::playback_log::record("api.begin", json!({"request": request, "attempt": attempt, "operation": operation, "method": method, "line_host": line_host}));
            let mut status_code = None;
            let mut headers_ms = None;
            let mut target_host = line_host.clone();
            let mut response_kind = "unknown";
            let mut gateway = "other";
            let result = async {
                let mut next = network_policy::parse_target(&url)?;
                for hop in 0..=5 {
                    let (target, client) = client_for(next.as_str()).await?;
                    target_host = target.host_str().unwrap_or("").to_string();
                    let mut req = client
                        .post(target.clone())
                        .header("Accept", "application/json")
                        .header("Referer", &referer)
                        .json(&envelope);
                    if let Some(t) = token.as_deref() {
                        if !t.is_empty() {
                            req = req.header("Authorization", format!("Bearer {t}"));
                        }
                    }
                    let resp = req.send().await.map_err(|e| err_text(&e))?;
                    let status = resp.status();
                    status_code = Some(status.as_u16());
                    headers_ms = Some(started.elapsed().as_millis() as u64);
                    let ctype = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
                    response_kind = if ctype.contains("json") { "json" } else if ctype.contains("html") { "html" } else { "other" };
                    gateway = if resp.headers().contains_key("x-amz-cf-id") { "cloudfront" } else if resp.headers().contains_key("cf-ray") { "cloudflare" } else { "other" };
                    // 逐跳重建并检查客户端；不自动把信封 POST 改成无信封 GET。
                    if status.is_redirection() {
                        if hop == 5 {
                            return Err("重定向次数超过限制".into());
                        }
                        let location = resp
                            .headers()
                            .get(reqwest::header::LOCATION)
                            .and_then(|h| h.to_str().ok())
                            .ok_or("重定向缺少有效 Location")?;
                        next = edge_redirect_target(&target, status.as_u16(), location, guest_get)?;
                        continue;
                    }
                    let text = resp.text().await.map_err(|e| err_text(&e))?;
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        return Err("401".to_string());
                    }
                    if !status.is_success() {
                        return Err(format!(
                            "HTTP {status}: {}",
                            text.chars().take(160).collect::<String>()
                        ));
                    }
                    if text.trim().is_empty() {
                        return Ok(Value::Null);
                    }
                    return serde_json::from_str::<Value>(&text)
                        .map_err(|e| format!("{e}: {}", text.chars().take(160).collect::<String>()));
                }
                Err("重定向次数超过限制".into())
            }.await;
            crate::playback_log::record("api.end", json!({
                "request": request, "attempt": attempt, "operation": operation, "method": method,
                "line_host": line_host, "target_host": target_host, "status": status_code,
                "headers_ms": headers_ms, "elapsed_ms": started.elapsed().as_millis() as u64,
                "response_kind": response_kind, "gateway": gateway,
                "business_status": result.as_ref().ok().and_then(|v: &Value| v.get("status")).and_then(Value::as_i64),
                "error_kind": result.as_ref().err().map(|e: &String| crate::playback_log::error_kind(e)),
                "retryable": can_retry && result.as_ref().err().is_some_and(|e| retryable_error(e)),
            }));
            result
        }
    })
    .await?;

    if value.is_null() {
        return Ok(Value::Null);
    }

    if value.get("status").and_then(|v| v.as_i64()) == Some(1) {
        Ok(value.get("data").cloned().unwrap_or(Value::Null))
    } else {
        Err(value
            .get("msg")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("接口返回失败")
            .to_string())
    }
}

/// 拉原始字节（图片 / m3u8 / 分片）。
pub async fn fetch_bytes(url: &str) -> Result<(Option<String>, Vec<u8>), String> {
    static NEXT_REQUEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let request = NEXT_REQUEST.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let resource = crate::playback_log::resource_id(url);
    let mut attempt = 0;
    let url = url.to_string();
    let target = url::Url::parse(&url).ok();
    let host = target
        .as_ref()
        .and_then(|target| target.host_str())
        .unwrap_or("")
        .to_string();
    let public_dns = AtomicBool::new(
        MEDIA_DNS_ROUTES
            .lock()
            .ok()
            .is_some_and(|routes| routes.prefers_public(&host, Instant::now())),
    );
    let referer: Option<String> = None;
    with_retry(true, || {
        attempt += 1;
        let attempt = attempt;
        let resource = resource.clone();
        let url = url.clone();
        let referer = referer.clone();
        let host = &host;
        let target = &target;
        let public_dns = &public_dns;
        async move {
            let using_public_dns = public_dns.load(Ordering::Relaxed);
            let started = Instant::now();
            crate::playback_log::record("network.begin", json!({"request": request, "resource": resource, "attempt": attempt}));
            let mut status = None;
            let mut headers_ms = None;
            let result = async {
                let resp = get_with_dns(&url, Duration::from_secs(20), None, referer.as_deref(), using_public_dns, None).await?;
                status = Some(resp.status().as_u16());
                headers_ms = Some(started.elapsed().as_millis() as u64);
                if !resp.status().is_success() {
                    return Err(format!("HTTP {}", resp.status()));
                }
                let ctype = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let bytes = bounded_body(resp, 32 * 1024 * 1024).await?;
                Ok((ctype, bytes))
            }.await;
            if using_public_dns {
                if let Ok(mut routes) = MEDIA_DNS_ROUTES.lock() {
                    if result.is_ok() {
                        routes.remember(host, Instant::now());
                    } else {
                        routes.forget(host);
                    }
                }
            }
            if let Err(error) = &result {
                if !crate::infra::source::allow_private_url(&url) && target.as_ref().is_some_and(|target| crate::media_dns::eligible(target, error)) {
                    public_dns.store(!using_public_dns, Ordering::Relaxed);
                    crate::playback_log::record("network.dns_fallback", json!({"resource": resource, "attempt": attempt, "public_dns": !using_public_dns, "error_kind": crate::playback_log::error_kind(error)}));
                }
            }
            crate::playback_log::record("network.end", json!({"request": request, "resource": resource, "attempt": attempt, "public_dns": using_public_dns, "status": status, "headers_ms": headers_ms, "elapsed_ms": started.elapsed().as_millis() as u64, "bytes": result.as_ref().ok().map(|(_, bytes)| bytes.len()), "error_kind": result.as_ref().err().map(|e: &String| crate::playback_log::error_kind(e))}));
            result
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "联网测试：需要 HGHUBX_TEST_MEDIA_URL 为当前有效的媒体地址"]
    async fn media_recovers_from_failed_system_dns() {
        let raw = std::env::var("HGHUBX_TEST_MEDIA_URL").expect("HGHUBX_TEST_MEDIA_URL");
        let target = url::Url::parse(&raw).unwrap();
        let host = target.host_str().unwrap();
        let port = target.port_or_known_default().unwrap();
        let failed_client = reqwest::Client::builder()
            .no_proxy()
            .resolve_to_addrs(host, &["127.0.0.1:9".parse().unwrap()])
            .connect_timeout(Duration::from_millis(100))
            .build()
            .unwrap();
        refresh_client();
        CLIENTS.lock().unwrap().insert(
            format!("{}:{host}:{port}:false", network_policy::strict_security()),
            (Instant::now() + CLIENT_TTL, Arc::new(failed_client)),
        );
        let (_, bytes) = fetch_bytes(&raw)
            .await
            .expect("备用 DNS 应恢复真实媒体下载");
        assert!(!bytes.is_empty());
        assert!(MEDIA_DNS_ROUTES
            .lock()
            .unwrap()
            .prefers_public(host, Instant::now()));
        let (_, again) = fetch_bytes(&raw).await.expect("后续请求应复用恢复的路径");
        if bytes.starts_with(b"#EXTM3U") {
            assert!(again.starts_with(b"#EXTM3U"));
        } else {
            assert_eq!(bytes, again);
        }
        if let Ok(path) = std::env::var("HGHUBX_TEST_MEDIA_OUTPUT") {
            std::fs::write(path, bytes).unwrap();
        }
    }

    #[tokio::test]
    async fn terminal_errors_do_not_retry_but_transient_failures_can_recover() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        for error in [
            "HTTP 404 Not Found: <html>nginx</html>",
            "HTTP 403 Forbidden",
            "401",
            "禁止访问内网",
            "重定向次数超过限制",
        ] {
            let calls = AtomicUsize::new(0);
            assert!(with_retry(true, || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(error.to_string())
            })
            .await
            .is_err());
            assert_eq!(calls.load(Ordering::SeqCst), 1, "{error}");
        }
        for error in ["HTTP 503 Service Unavailable", "请求超时"] {
            let calls = AtomicUsize::new(0);
            let value = with_retry(true, || {
                let attempt = calls.fetch_add(1, Ordering::SeqCst);
                async move {
                    if attempt == 0 {
                        Err(error.to_string())
                    } else {
                        Ok(42)
                    }
                }
            })
            .await
            .unwrap();
            assert_eq!(value, 42);
            assert_eq!(calls.load(Ordering::SeqCst), 2);
        }
    }

    #[tokio::test]
    async fn non_idempotent_operations_are_attempted_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        for (retry, expected) in [(false, 1), (true, 3)] {
            let calls = AtomicUsize::new(0);
            let result = with_retry(retry, || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>("rejected".to_string())
            })
            .await;
            assert!(result.is_err());
            assert_eq!(calls.load(Ordering::SeqCst), expected);
        }
    }

    #[test]
    fn dns_diagnostics_only_expose_counts_and_fake_ip_presence() {
        let addrs: Vec<SocketAddr> = ["10.0.0.1:443", "198.18.0.2:443", "[::ffff:198.18.0.3]:443"]
            .into_iter()
            .map(|raw| raw.parse().unwrap())
            .collect();
        let summary = dns_summary(addrs);
        assert_eq!(summary, json!({"count": 3, "has_fake_ip": true}));
        assert!(!summary.to_string().contains("10.0.0.1"));
        assert!(!summary.to_string().contains("198.18"));
        assert_eq!(
            dns_summary(Vec::<SocketAddr>::new()),
            json!({"count":0,"has_fake_ip":false})
        );
    }

    #[test]
    fn pinned_clients_expire_without_extending_the_dns_ttl() {
        let now = Instant::now();
        let client = Arc::new(reqwest::Client::new());
        let mut cache = ClientCache::new();
        cache.insert(
            "true:example.com:443".into(),
            (now + CLIENT_TTL, client.clone()),
        );
        assert!(Arc::ptr_eq(
            &cached_client(
                &cache,
                "true:example.com:443",
                now + Duration::from_secs(29)
            )
            .unwrap(),
            &client
        ));
        assert!(cached_client(&cache, "true:example.com:443", now + CLIENT_TTL).is_none());
        assert!(cached_client(&cache, "false:example.com:443", now).is_none());
        assert!(cached_client(&cache, "true:example.com:8443", now).is_none());
    }
}
