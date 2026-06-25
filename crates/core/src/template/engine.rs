//! Движок шаблонизации
//!
//! Использует Handlebars для подстановки переменных

use std::collections::HashMap;
use serde_json::Value;
use handlebars::Handlebars;

use crate::error::{CoreError, CoreResult};

/// Движок шаблонизации
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    /// Создает новый движок
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        // Отключаем экранирование для поддержки JSON
        handlebars.register_escape_fn(handlebars::no_escape);
        TemplateEngine { handlebars }
    }

    /// Рендерит шаблон с переменными
    ///
    /// # Пример
    /// ```
    /// use serde_json::json;
    /// use rivet_core::template::TemplateEngine;
    ///
    /// let engine = TemplateEngine::new();
    /// let mut vars = HashMap::new();
    /// vars.insert("name".to_string(), json!("John"));
    ///
    /// let result = engine.render("Hello {{name}}!", &vars)?;
    /// assert_eq!(result, "Hello John!");
    /// ```
    pub fn render(
        &self,
        template: &str,
        variables: &HashMap<String, Value>,
    ) -> CoreResult<String> {
        let context = serde_json::to_value(variables)
            .map_err(|e| CoreError::TemplateError(e.to_string()))?;

        self.handlebars
            .render_template(template, &context)
            .map_err(|e| CoreError::TemplateError(e.to_string()))
    }

    /// Рендерит JSON значение с подстановкой переменных
    pub fn render_json(
        &self,
        value: &Value,
        variables: &HashMap<String, Value>,
    ) -> CoreResult<Value> {
        let json_str = serde_json::to_string(value)
            .map_err(CoreError::JsonError)?;

        let rendered = self.render(&json_str, variables)?;

        serde_json::from_str(&rendered)
            .map_err(CoreError::JsonError)
    }

    /// Рендерит URL с подстановкой переменных
    pub fn render_url(
        &self,
        url: &str,
        variables: &HashMap<String, Value>,
    ) -> CoreResult<String> {
        self.render(url, variables)
    }

    /// Рендерит заголовки с подстановкой переменных
    pub fn render_headers(
        &self,
        headers: &HashMap<String, String>,
        variables: &HashMap<String, Value>,
    ) -> CoreResult<HashMap<String, String>> {
        let mut result = HashMap::new();
        for (key, value) in headers {
            let rendered = self.render(value, variables)?;
            result.insert(key.clone(), rendered);
        }
        Ok(result)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_simple() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), json!("John"));

        let result = engine.render("Hello {{name}}!", &vars).unwrap();
        assert_eq!(result, "Hello John!");
    }

    #[test]
    fn test_render_json() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("user_id".to_string(), json!(123));
        vars.insert("name".to_string(), json!("John"));

        let template = json!({
            "id": "{{user_id}}",
            "name": "{{name}}"
        });

        let result = engine.render_json(&template, &vars).unwrap();
        assert_eq!(result["id"], json!("123"));
        assert_eq!(result["name"], json!("John"));
    }

    #[test]
    fn test_render_missing_variable() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        // Если переменной нет - оставляем как есть
        let result = engine.render("Hello {{name}}!", &vars).unwrap();
        assert_eq!(result, "Hello {{name}}!");
    }

    #[test]
    fn test_render_nested_variables() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), json!({
            "name": "John",
            "age": 30
        }));

        let result = engine.render("User: {{user.name}}, age: {{user.age}}", &vars).unwrap();
        assert_eq!(result, "User: John, age: 30");
    }
}