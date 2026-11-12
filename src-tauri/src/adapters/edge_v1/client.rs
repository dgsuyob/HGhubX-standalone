//! Edge V1 传输客户端：负责拼 `POST /api/edge/request` 信封。

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::Serialize;
use serde_json::Value;

/// 上游网关唯一入口的信封。方法、路径与 body 都由适配器填写。
#[derive(Debug, Serialize)]
pub struct EdgeEnvelope {
    pub method: String,
    pub path: String,
    pub body: String,
    pub content_type: String,
}

pub struct EdgeV1Client {
    origin: String,
    token: Option<String>,
}

impl EdgeV1Client {
    pub fn new(origin: String, token: Option<String>) -> Self {
        Self { origin, token }
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub async fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, String> {
        let (encoded, content_type) = match body {
            Some(value) => (
                B64.encode(serde_json::to_vec(value).map_err(|e| e.to_string())?),
                "application/json",
            ),
            None => (String::new(), ""),
        };
        let envelope = EdgeEnvelope {
            method: method.to_string(),
            path: path.to_string(),
            body: encoded,
            content_type: content_type.to_string(),
        };
        let envelope = serde_json::to_value(envelope).map_err(|e| e.to_string())?;
        crate::net::edge_envelope(
            &self.origin,
            envelope,
            method,
            path,
            body.is_none(),
            self.token.as_deref(),
        )
        .await
    }

    pub async fn get(&self, path: &str) -> Result<Value, String> {
        self.call("GET", path, None).await
    }

    pub async fn post(&self, path: &str, body: Option<&Value>) -> Result<Value, String> {
        self.call("POST", path, body).await
    }

    pub async fn delete(&self, path: &str) -> Result<Value, String> {
        self.call("DELETE", path, None).await
    }

    pub fn require_token(&self) -> Result<&str, String> {
        self.token
            .as_deref()
            .filter(|token| !token.is_empty())
            .ok_or_else(|| "请先登录".to_string())
    }
}
