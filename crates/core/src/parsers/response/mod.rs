//! Парсеры ответов HTTP
//!
//! Отвечают за:
//! - Извлечение данных из JSON по JSONPath (json_path)
//! - Извлечение данных из ответа (extractor)
//! - Поддержка разных источников: тело, заголовки, статус, cookies

mod json_path;
mod extractor;

// Публичный экспорт
pub use json_path::JsonPathParser;
pub use extractor::ResponseExtractor;

// Переэкспорт для удобства
use crate::error::CoreResult;
use serde_json::Value;
use reqwest::header::HeaderMap;

/// Извлекает данные из ответа по указанным правилам
///
/// Удобная обертка над ResponseExtractor
///
/// # Пример
/// ```
/// use rivet_core::parsers::response::extract_from_response;
///
/// let extracted = extract_from_response(
///     &body_json,
///     &headers,
///     200,
///     &extract_rules
/// )?;
/// ```
pub fn extract_from_response(
    body: &Value,
    headers: &HeaderMap,
    status: u16,
    extract_rules: &[crate::parsers::config::Extract],
) -> CoreResult<std::collections::HashMap<String, Value>> {
    ResponseExtractor::extract(body, headers, status, extract_rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use reqwest::header::{HeaderMap, HeaderValue};
    use crate::parsers::config::{Extract, ExtractSource};

    #[test]
    fn test_extract_from_response_body() {
        let body = json!({
            "data": {
                "token": "abc123",
                "user": {
                    "id": 1,
                    "name": "John"
                }
            }
        });
        let headers = HeaderMap::new();
        let extract_rules = vec![
            Extract {
                name: "token".to_string(),
                source: ExtractSource::Body { path: "data.token".to_string() },
            },
            Extract {
                name: "user_id".to_string(),
                source: ExtractSource::Body { path: "data.user.id".to_string() },
            },
        ];

        let result = extract_from_response(&body, &headers, 200, &extract_rules).unwrap();

        assert_eq!(result.get("token").unwrap(), &json!("abc123"));
        assert_eq!(result.get("user_id").unwrap(), &json!(1));
    }

    #[test]
    fn test_extract_from_response_header() {
        let body = json!({});
        let mut headers = HeaderMap::new();
        headers.insert("x-auth-token", HeaderValue::from_static("secret123"));

        let extract_rules = vec![
            Extract {
                name: "auth_token".to_string(),
                source: ExtractSource::Header { name: "x-auth-token".to_string() },
            },
        ];

        let result = extract_from_response(&body, &headers, 200, &extract_rules).unwrap();

        assert_eq!(result.get("auth_token").unwrap(), &json!("secret123"));
    }

    #[test]
    fn test_extract_from_response_status() {
        let body = json!({});
        let headers = HeaderMap::new();

        let extract_rules = vec![
            Extract {
                name: "status_code".to_string(),
                source: ExtractSource::Status,
            },
        ];

        let result = extract_from_response(&body, &headers, 201, &extract_rules).unwrap();

        assert_eq!(result.get("status_code").unwrap(), &json!(201));
    }
}