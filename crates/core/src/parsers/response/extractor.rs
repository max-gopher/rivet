//! Экстрактор данных из HTTP ответов
//!
//! Извлекает данные из:
//! - JSON тела (через JSONPath)
//! - Заголовков
//! - Статуса ответа
//! - Cookies

use std::collections::HashMap;
use serde_json::Value;
use reqwest::header::HeaderMap;

use crate::parsers::config::{Extract, ExtractSource};
use crate::parsers::response::json_path::JsonPathParser;
use crate::error::{CoreError, CoreResult};

/// Экстрактор данных из ответов
///
/// Извлекает данные по правилам из разных источников
#[derive(Debug, Default)]
pub struct ResponseExtractor;

impl ResponseExtractor {
    /// Создает новый экстрактор
    pub fn new() -> Self {
        ResponseExtractor
    }

    /// Извлекает данные из ответа по правилам
    ///
    /// # Аргументы
    /// * `body` - JSON тело ответа
    /// * `headers` - Заголовки ответа
    /// * `status` - HTTP статус
    /// * `extracts` - Правила извлечения
    ///
    /// # Возвращает
    /// HashMap имен переменных и их значений
    ///
    /// # Пример
    /// ```
    /// use serde_json::json;
    /// use reqwest::header::{HeaderMap, HeaderValue};
    /// use rivet_core::parsers::response::ResponseExtractor;
    /// use rivet_core::parsers::config::models::{Extract, ExtractSource};
    ///
    /// let body = json!({
    ///     "data": {
    ///         "token": "abc123",
    ///         "user_id": 42
    ///     }
    /// });
    ///
    /// let mut headers = HeaderMap::new();
    /// headers.insert("x-request-id", HeaderValue::from_static("req-123"));
    ///
    /// let extracts = vec![
    ///     Extract {
    ///         name: "token".to_string(),
    ///         source: ExtractSource::Body { path: "data.token".to_string() },
    ///     },
    ///     Extract {
    ///         name: "request_id".to_string(),
    ///         source: ExtractSource::Header { name: "x-request-id".to_string() },
    ///     },
    /// ];
    ///
    /// let result = ResponseExtractor::extract(&body, &headers, 200, &extracts).unwrap();
    /// assert_eq!(result.get("token").unwrap(), &json!("abc123"));
    /// assert_eq!(result.get("request_id").unwrap(), &json!("req-123"));
    /// ```
    pub fn extract(
        body: &Value,
        headers: &HeaderMap,
        status: u16,
        extracts: &[Extract],
    ) -> CoreResult<HashMap<String, Value>> {
        let mut results = HashMap::new();

        for extract in extracts {
            let value = Self::extract_one(body, headers, status, extract)?;
            results.insert(extract.name.clone(), value);
        }

        Ok(results)
    }

    /// Извлекает одно значение по правилу
    fn extract_one(
        body: &Value,
        headers: &HeaderMap,
        status: u16,
        extract: &Extract,
    ) -> CoreResult<Value> {
        match &extract.source {
            ExtractSource::Body { path } => {
                Self::extract_from_body(body, path)
            }
            ExtractSource::Header { name } => {
                Self::extract_from_header(headers, name)
            }
            ExtractSource::Status => {
                Ok(Value::Number(status.into()))
            }
            ExtractSource::Cookie { name } => {
                Self::extract_from_cookie(headers, name)
            }
            ExtractSource::Regex { pattern } => {
                Self::extract_from_regex(body, pattern)
            }
        }
    }

    /// Извлекает данные из тела ответа
    fn extract_from_body(body: &Value, path: &str) -> CoreResult<Value> {
        // Проверяем, что это JSON
        if body.is_null() && !body.is_object() && !body.is_array() {
            return Err(CoreError::ParseError(
                "Response body is not valid JSON".to_string()
            ));
        }

        // Ищем значение по пути
        let result = JsonPathParser::extract_first(body, path)?;

        match result {
            Some(value) => Ok(value.clone()),
            None => Err(CoreError::ParseError(
                format!("Path '{}' not found in response body", path)
            )),
        }
    }

    /// Извлекает данные из заголовка
    fn extract_from_header(headers: &HeaderMap, name: &str) -> CoreResult<Value> {
        let header_name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| CoreError::ParseError(
                format!("Invalid header name '{}': {}", name, e)
            ))?;

        if let Some(value) = headers.get(&header_name) {
            if let Ok(value_str) = value.to_str() {
                Ok(Value::String(value_str.to_string()))
            } else {
                Err(CoreError::ParseError(
                    format!("Header '{}' contains non-UTF8 data", name)
                ))
            }
        } else {
            Err(CoreError::ParseError(
                format!("Header '{}' not found in response", name)
            ))
        }
    }

    /// Извлекает данные из cookie
    fn extract_from_cookie(headers: &HeaderMap, name: &str) -> CoreResult<Value> {
        // Ищем заголовок Set-Cookie
        let cookie_header = headers
            .get("set-cookie")
            .ok_or_else(|| CoreError::ParseError(
                "No Set-Cookie header found".to_string()
            ))?;

        let cookie_str = cookie_header.to_str()
            .map_err(|_| CoreError::ParseError(
                "Set-Cookie header contains non-UTF8 data".to_string()
            ))?;

        // Парсим cookies
        let cookies: Vec<&str> = cookie_str.split(';').collect();

        for cookie in cookies {
            let trimmed = cookie.trim();
            if let Some((key, value)) = trimmed.split_once('=') {
                if key.trim() == name {
                    return Ok(Value::String(value.trim().to_string()));
                }
            }
        }

        Err(CoreError::ParseError(
            format!("Cookie '{}' not found", name)
        ))
    }

    /// Извлекает данные по регулярному выражению
    fn extract_from_regex(body: &Value, pattern: &str) -> CoreResult<Value> {
        // Преобразуем тело в строку
        let body_str = match body {
            Value::String(s) => s.clone(),
            Value::Object(_) | Value::Array(_) => {
                serde_json::to_string(body)
                    .map_err(|e| CoreError::ParseError(
                        format!("Failed to serialize JSON: {}", e)
                    ))?
            }
            _ => body.to_string(),
        };

        // Компилируем regex
        let re = regex::Regex::new(pattern)
            .map_err(|e| CoreError::ParseError(
                format!("Invalid regex pattern '{}': {}", pattern, e)
            ))?;

        // Ищем первое совпадение
        if let Some(captures) = re.captures(&body_str) {
            if let Some(matched) = captures.get(0) {
                return Ok(Value::String(matched.as_str().to_string()));
            }
        }

        Err(CoreError::ParseError(
            format!("Pattern '{}' not found in response", pattern)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use reqwest::header::{HeaderMap, HeaderValue};

    fn create_test_extracts() -> Vec<Extract> {
        vec![
            Extract {
                name: "token".to_string(),
                source: ExtractSource::Body { path: "data.token".to_string() },
            },
            Extract {
                name: "user_id".to_string(),
                source: ExtractSource::Body { path: "data.user.id".to_string() },
            },
            Extract {
                name: "request_id".to_string(),
                source: ExtractSource::Header { name: "x-request-id".to_string() },
            },
            Extract {
                name: "status_code".to_string(),
                source: ExtractSource::Status,
            },
        ]
    }

    #[test]
    fn test_extract_from_body() {
        let body = json!({
            "data": {
                "token": "abc123",
                "user": {
                    "id": 42,
                    "name": "John"
                }
            }
        });

        let result = ResponseExtractor::extract_from_body(&body, "data.token").unwrap();
        assert_eq!(result, json!("abc123"));

        let result = ResponseExtractor::extract_from_body(&body, "data.user.id").unwrap();
        assert_eq!(result, json!(42));

        let result = ResponseExtractor::extract_from_body(&body, "data.user.name").unwrap();
        assert_eq!(result, json!("John"));
    }

    #[test]
    fn test_extract_from_body_not_found() {
        let body = json!({ "data": {} });

        let result = ResponseExtractor::extract_from_body(&body, "data.token");
        assert!(result.is_err());
        match result {
            Err(CoreError::ParseError(msg)) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_extract_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("req-123"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let result = ResponseExtractor::extract_from_header(&headers, "x-request-id").unwrap();
        assert_eq!(result, json!("req-123"));

        let result = ResponseExtractor::extract_from_header(&headers, "content-type").unwrap();
        assert_eq!(result, json!("application/json"));
    }

    #[test]
    fn test_extract_from_header_not_found() {
        let headers = HeaderMap::new();

        let result = ResponseExtractor::extract_from_header(&headers, "x-request-id");
        assert!(result.is_err());
        match result {
            Err(CoreError::ParseError(msg)) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_extract_from_status() {
        let result = ResponseExtractor::extract_one(
            &json!({}),
            &HeaderMap::new(),
            200,
            &Extract {
                name: "status".to_string(),
                source: ExtractSource::Status,
            },
        ).unwrap();

        assert_eq!(result, json!(200));
    }

    #[test]
    fn test_extract_full_response() {
        let body = json!({
            "data": {
                "token": "abc123",
                "user": {
                    "id": 42
                }
            }
        });

        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("req-123"));

        let extracts = create_test_extracts();
        let results = ResponseExtractor::extract(&body, &headers, 201, &extracts).unwrap();

        assert_eq!(results.len(), 4);
        assert_eq!(results.get("token").unwrap(), &json!("abc123"));
        assert_eq!(results.get("user_id").unwrap(), &json!(42));
        assert_eq!(results.get("request_id").unwrap(), &json!("req-123"));
        assert_eq!(results.get("status_code").unwrap(), &json!(201));
    }

    #[test]
    fn test_extract_from_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "set-cookie",
            HeaderValue::from_static("session=abc123; Path=/; HttpOnly; session_id=xyz789")
        );

        let result = ResponseExtractor::extract_from_cookie(&headers, "session").unwrap();
        assert_eq!(result, json!("abc123"));

        let result = ResponseExtractor::extract_from_cookie(&headers, "session_id").unwrap();
        assert_eq!(result, json!("xyz789"));
    }

    #[test]
    fn test_extract_from_cookie_not_found() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "set-cookie",
            HeaderValue::from_static("session=abc123")
        );

        let result = ResponseExtractor::extract_from_cookie(&headers, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_from_regex() {
        let body = json!("Response body: error-12345 occurred");

        let result = ResponseExtractor::extract_from_regex(&body, r"error-(\d+)").unwrap();
        assert_eq!(result, json!("error-12345"));
    }

    #[test]
    fn test_extract_from_regex_not_found() {
        let body = json!("No error here");

        let result = ResponseExtractor::extract_from_regex(&body, r"error-(\d+)");
        assert!(result.is_err());
        match result {
            Err(CoreError::ParseError(msg)) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_extract_from_regex_invalid_pattern() {
        let body = json!("test");

        let result = ResponseExtractor::extract_from_regex(&body, r"[");
        assert!(result.is_err());
        match result {
            Err(CoreError::ParseError(msg)) => {
                assert!(msg.contains("Invalid regex pattern"));
            }
            _ => panic!("Expected ParseError"),
        }
    }
}