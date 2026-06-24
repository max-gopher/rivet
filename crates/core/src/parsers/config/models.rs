use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub name: String,

    #[serde(default)]
    pub version: Option<String>,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub metadata: HashMap<String, String>,

    #[serde(default)]
    pub http: Option<HttpConfig>,

    pub stages: Vec<Stage>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub id: Option<String>,

    pub name: String,

    pub description: Option<String>,

    #[serde(default)]
    pub depends_on: Vec<String>,

    #[serde(default)]
    pub skip: Option<bool>,

    #[serde(default)]
    pub retry: Option<RetryConfig>,

    #[serde(default)]
    pub timeout: Option<u64>,

    pub request: Request,

    #[serde(default)]
    pub extract: Vec<Extract>,

    #[serde(default)]
    pub assert: Vec<Assert>,

    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub count: u32,
    pub delay: u64,
    #[serde(default)]
    pub backoff: Option<BackoffStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential { factor: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub method: HttpMethod,
    pub url: String,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub params: HashMap<String, String>,

    #[serde(default)]
    pub body: Option<RequestBody>,

    #[serde(default)]
    pub auth: Option<Auth>,

    #[serde(default)]
    pub validate_ssl: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Trace
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestBody {
    Json(serde_json::Value),
    Form(Vec<(String, String)>),
    Text(String),
    Raw(Vec<u8>),
    Multipart(Vec<Part>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub name: String,

    #[serde(default)]
    pub filename: Option<String>,

    #[serde(default)]
    pub content_type: Option<String>,

    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Auth {
    #[serde(rename = "bearer")]
    Bearer { token: String },

    #[serde(rename = "basic")]
    Basic { username: String, password: String },

    #[serde(rename = "api_key")]
    ApiKey {
        key: String,
        value: String,
        #[serde(default)]
        in_header: bool,
        #[serde(default)]
        prefix: Option<String>,
    },

    #[serde(rename = "oauth2")]
    OAuth2 {
        token_url: String,
        client_id: String,
        client_secret: String,
        #[serde(default)]
        scope: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extract {
    pub name: String,
    pub source: ExtractSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtractSource {
    Body { path: String },
    Header { name: String },
    Status,
    Cookie { name: String },
    Regex { pattern: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Assert {
    Status(StatusAssert),
    Body(BodyAssert),
    Header(HeaderAssert),
    Custom(CustomAssert),
    And(Vec<Assert>),
    Or(Vec<Assert>),
    Not(Box<Assert>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusAssert {
    pub status: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyAssert {
    pub path: String,
    #[serde(default)]
    pub equals: Option<serde_json::Value>,
    #[serde(default)]
    pub not_null: Option<bool>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub r#type: Option<ValueType>,
    #[serde(default)]
    pub contains: Option<serde_json::Value>,
    #[serde(default)]
    pub in_range: Option<(serde_json::Value, serde_json::Value)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Null,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderAssert {
    pub header: String,
    #[serde(default)]
    pub equals: Option<String>,
    #[serde(default)]
    pub exists: Option<bool>,
    #[serde(default)]
    pub regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAssert {
    pub expression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Таймаут в секундах
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Количество повторных попыток
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,

    /// Задержка между попытками в миллисекундах
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,

    /// Использовать экспоненциальную задержку
    #[serde(default = "default_exponential_backoff")]
    pub exponential_backoff: bool,

    /// Проверять SSL сертификаты
    #[serde(default = "default_verify_ssl")]
    pub verify_ssl: bool,

    /// User-Agent
    #[serde(default = "default_user_agent")]
    pub user_agent: String
}

fn default_timeout() -> u64 { 30 }
fn default_retry_count() -> u32 { 3 }
fn default_retry_delay() -> u64 { 1000 }
fn default_exponential_backoff() -> bool { true }
fn default_verify_ssl() -> bool { true }
fn default_user_agent() -> String {
    format!("Rivet/{}", env!("CARGO_PKG_VERSION"))
}

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            timeout: default_timeout(),
            retry_count: default_retry_count(),
            retry_delay: default_retry_delay(),
            exponential_backoff: default_exponential_backoff(),
            verify_ssl: default_verify_ssl(),
            user_agent: default_user_agent()
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Trace => "TRACE",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_serialization() {
        // Проверяем, что enum правильно сериализуется/десериализуется
        let yaml = r#"
status: 200
"#;
        let assert: Assert = serde_yml::from_str(yaml).unwrap();
        match assert {
            Assert::Status(s) => assert_eq!(s.status, 200),
            _ => panic!("Expected Status assert"),
        }
    }

    #[test]
    fn test_body_assert_serialization() {
        let yaml = r#"
path: "data.token"
not_null: true
type: string
"#;
        let assert: BodyAssert = serde_yml::from_str(yaml).unwrap();
        assert_eq!(assert.path, "data.token");
        assert_eq!(assert.not_null, Some(true));
        assert_eq!(assert.r#type, Some(ValueType::String));
    }

    #[test]
    fn test_stage_with_all_fields() {
        let yaml = r#"
name: "Test Stage"
description: "This is a test"
depends_on:
  - "Login"
skip: false
request:
  method: GET
  url: "https://api.example.com"
  headers:
    Authorization: "Bearer {{token}}"
"#;
        let stage: Stage = serde_yml::from_str(yaml).unwrap();
        assert_eq!(stage.name, "Test Stage");
        assert_eq!(stage.depends_on, vec!["Login"]);
        assert_eq!(stage.request.method, HttpMethod::Get);
    }
}