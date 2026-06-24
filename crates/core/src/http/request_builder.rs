//! Построитель HTTP запросов из конфигурации этапа

use std::collections::HashMap;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::parsers::config::*;
use crate::http::HttpRequest;
use crate::error::{CoreError, CoreResult};
use crate::template::TemplateEngine;

pub struct RequestBuilder {
    template_engine: TemplateEngine,
}

impl RequestBuilder {
    pub fn new() -> Self {
        RequestBuilder {
            template_engine: TemplateEngine::new(),
        }
    }

    pub fn build(
        &self,
        stage: &Stage,
        variables: &HashMap<String, serde_json::Value>,
    ) -> CoreResult<HttpRequest> {
        // 1. URL
        let url = self.template_engine.render(&stage.request.url, variables)?;

        // 2. Заголовки
        let mut headers = HashMap::new();
        for (key, value) in &stage.request.headers {
            let rendered = self.template_engine.render(value, variables)?;
            headers.insert(key.clone(), rendered);
        }

        // 3. Query параметры
        let mut params = HashMap::new();
        for (key, value) in &stage.request.params {
            let rendered = self.template_engine.render(value, variables)?;
            params.insert(key.clone(), rendered);
        }

        // 4. Auth
        if let Some(auth) = &stage.request.auth {
            match auth {
                Auth::Bearer { token } => {
                    let rendered_token = self.template_engine.render(token, variables)?;
                    headers.insert("Authorization".to_string(), format!("Bearer {}", rendered_token));
                }
                Auth::Basic { username, password } => {
                    let rendered_username = self.template_engine.render(username, variables)?;
                    let rendered_password = self.template_engine.render(password, variables)?;
                    let credentials = format!("{}:{}", rendered_username, rendered_password);
                    let encoded = STANDARD.encode(credentials);
                    headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
                }
                Auth::ApiKey { key, value, in_header, prefix } => {
                    let rendered_value = self.template_engine.render(value, variables)?;
                    let header_value = if let Some(prefix) = prefix {
                        format!("{} {}", prefix, rendered_value)
                    } else {
                        rendered_value
                    };
                    if *in_header {
                        headers.insert(key.clone(), header_value);
                    } else {
                        params.insert(key.clone(), header_value);
                    }
                }
                _ => {}
            }
        }

        // 5. Тело запроса
        let (body, form_data) = if let Some(body) = &stage.request.body {
            match body {
                RequestBody::Json(json) => {
                    let json_str = serde_json::to_string(json)
                        .map_err(|e| CoreError::TemplateError(e.to_string()))?;
                    let rendered = self.template_engine.render(&json_str, variables)?;
                    let parsed = serde_json::from_str(&rendered)
                        .map_err(|e| CoreError::JsonError(e))?;
                    (Some(parsed), None)
                }
                RequestBody::Text(text) => {
                    let rendered = self.template_engine.render(text, variables)?;
                    (Some(serde_json::Value::String(rendered)), None)
                }
                RequestBody::Form(form) => {
                    let mut form_data = HashMap::new();
                    for (key, value) in form {
                        let rendered = self.template_engine.render(value, variables)?;
                        form_data.insert(key.clone(), rendered);
                    }
                    (None, Some(form_data))
                }
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        Ok(HttpRequest {
            method: stage.request.method.to_string(),
            url,
            headers,
            params,
            body,
            form_data,
        })
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}