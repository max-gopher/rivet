//! HTTP клиент для выполнения запросов
//!
//! Поддерживает:
//! - HTTP/1.1, HTTP/2, HTTP/3
//! - Таймауты
//! - Retry с экспоненциальной задержкой
//! - Настраиваемые заголовки
//! - Cookies

use std::time::Duration;
use std::collections::HashMap;
use reqwest::{Client};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::parsers::config::HttpConfig;
use crate::error::{CoreError, CoreResult};

/// HTTP клиент
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    config: HttpConfig,  // ← Теперь используем HttpConfig напрямую
}

impl HttpClient {
    /// Создает новый HTTP клиент из конфига
    pub fn new(config: HttpConfig) -> Self {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .user_agent(&config.user_agent)
            .pool_max_idle_per_host(100)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90));

        if !config.verify_ssl {
            builder = builder.danger_accept_invalid_certs(true);
        }

        // HTTP/3 опционально
        #[cfg(feature = "http3")]
        if config.enable_http3 {
            builder = builder.http3_prior_knowledge();
        }

        let client = builder.build()
            .expect("Failed to build HTTP client");

        HttpClient { client, config }
    }

    /// Создает клиент из опционального конфига
    pub fn from_optional_config(config: Option<HttpConfig>) -> Self {
        match config {
            Some(cfg) => Self::new(cfg),
            None => Self::default(),
        }
    }

    /// Выполняет HTTP запрос
    pub async fn execute(&self, request: HttpRequest) -> CoreResult<HttpResponse> {
        self.execute_with_retry(request).await
    }

    /// Выполняет HTTP запрос с повторными попытками
    pub async fn execute_with_retry(&self, request: HttpRequest) -> CoreResult<HttpResponse> {
        let mut last_error = None;
        let mut delay = self.config.retry_delay;

        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                tracing::debug!(
                "Retry attempt {}/{} for {} {}",
                attempt,
                self.config.retry_count,
                request.method,
                request.url
            );

                tokio::time::sleep(Duration::from_millis(delay)).await;

                if self.config.exponential_backoff {
                    delay *= 2;
                }
            }

            match self.execute_once(&request).await {
                Ok(response) => {
                    let is_server_error = response.status >= 500 && response.status < 600;

                    if is_server_error && attempt < self.config.retry_count {
                        tracing::warn!(
                        "Server error {}, retrying...",
                        response.status
                    );
                        continue;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.retry_count {
                        tracing::warn!(
                        "Request failed: {}, retrying...",
                        last_error.as_ref().unwrap()
                    );
                    }
                }
            }
        }

        // Возвращаем ошибку с текстом
        Err(CoreError::HttpError(
            format!(
                "Request failed after {} retries: {:?}",
                self.config.retry_count,
                last_error
            )
        ))
    }

    /// Выполняет запрос один раз (без retry)
    async fn execute_once(&self, request: &HttpRequest) -> CoreResult<HttpResponse> {
        // Добавляем query параметры к URL
        let mut url = request.url.clone();
        if !request.params.is_empty() {
            let mut params = Vec::new();
            for (key, value) in &request.params {
                params.push(format!("{}={}", key, value));
            }
            let query_string = params.join("&");
            if url.contains('?') {
                url.push_str(&format!("&{}", query_string));
            } else {
                url.push_str(&format!("?{}", query_string));
            }
        }

        let mut request_builder = self.client.request(
            reqwest::Method::from_bytes(request.method.as_bytes())
                .map_err(|e| CoreError::HttpError(e.to_string()))?,
            &url,
        );

        for (name, value) in &request.headers {
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|e| CoreError::HttpError(e.to_string()))?;
            let header_value = HeaderValue::from_str(value)
                .map_err(|e| CoreError::HttpError(e.to_string()))?;
            request_builder = request_builder.header(header_name, header_value);
        }

        if let Some(body) = &request.body {
            // Проверяем, что это Form
            if let Some(form_data) = request.form_data.as_ref() {
                // Отправляем как форму
                request_builder = request_builder.form(form_data);
            } else {
                // Отправляем как JSON
                request_builder = request_builder.json(body);
            }
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?;

        let status = response.status();
        let headers_map = response.headers().clone();  // ← Сохраняем оригинал
        let headers = headers_to_hashmap(&headers_map);
        let body = response
            .text()
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?;

        let json_body = serde_json::from_str(&body)
            .unwrap_or(serde_json::Value::Null);

        Ok(HttpResponse {
            status: status.as_u16(),
            headers,
            headers_map,
            body,
            json_body,
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new(HttpConfig::default())
    }
}

/// HTTP запрос
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub form_data: Option<HashMap<String, String>>
}

/// HTTP ответ
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub headers_map: HeaderMap,
    pub body: String,
    pub json_body: serde_json::Value,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }
}

/// Преобразует reqwest заголовки в HashMap
fn headers_to_hashmap(headers: &HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (name, value) in headers {
        if let Ok(value_str) = value.to_str() {
            map.insert(name.to_string(), value_str.to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_default() {
        let client = HttpClient::default();
        assert_eq!(client.config.timeout, 30);
        assert_eq!(client.config.retry_count, 3);
    }

    #[tokio::test]
    async fn test_http_request_building() {
        let request = HttpRequest {
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            form_data: None
        };

        let client = HttpClient::default();
        let response = client.execute(request).await;

        assert!(response.is_ok());
        let resp = response.unwrap();
        assert!(resp.is_success());

        // Проверяем, что тело содержит правильный URL
        let body_json: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(body_json["url"], "https://httpbin.org/get");
    }

    #[test]
    fn test_http_response_helpers() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            headers_map: HeaderMap::new(),
            body: "OK".to_string(),
            json_body: serde_json::Value::Null,
        };

        assert!(response.is_success());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());

        let response = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            headers_map: HeaderMap::new(),
            body: "Not Found".to_string(),
            json_body: serde_json::Value::Null,
        };

        assert!(!response.is_success());
        assert!(response.is_client_error());
        assert!(!response.is_server_error());

        let response = HttpResponse {
            status: 500,
            headers: HashMap::new(),
            headers_map: HeaderMap::new(),
            body: "Server Error".to_string(),
            json_body: serde_json::Value::Null,
        };

        assert!(!response.is_success());
        assert!(!response.is_client_error());
        assert!(response.is_server_error());
    }
}