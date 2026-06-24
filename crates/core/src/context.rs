//! Контекст выполнения тестов
//!
//! Хранит состояние выполнения:
//! - Переменные (извлеченные из ответов)
//! - Результаты завершенных этапов
//! - Время начала выполнения

use std::collections::HashMap;
use serde_json::Value;
use chrono::{DateTime, Utc};

/// Контекст выполнения тестов
///
/// Содержит все состояние, которое накапливается во время выполнения:
/// - Переменные, извлеченные из ответов (токены, ID, и т.д.)
/// - Информацию о завершенных этапах
/// - Временные метки
#[derive(Debug, Clone)]
pub struct TestContext {
    /// Переменные, доступные для подстановки в шаблоны
    /// Например: { "token": "eyJhbGc...", "user_id": 123 }
    pub variables: HashMap<String, Value>,

    /// Список завершенных этапов (по имени)
    /// Нужен для проверки зависимостей
    pub completed_stages: Vec<String>,

    /// Детальные результаты каждого этапа
    /// Хранит, прошел ли этап, сколько длился, ошибки
    pub stage_results: HashMap<String, StageResult>,

    /// Время начала выполнения всего теста
    pub started_at: DateTime<Utc>,
}

impl TestContext {
    /// Создает новый пустой контекст
    pub fn new() -> Self {
        TestContext {
            variables: HashMap::new(),
            completed_stages: Vec::new(),
            stage_results: HashMap::new(),
            started_at: Utc::now(),
        }
    }

    /// Добавляет или обновляет переменную
    ///
    /// # Пример
    /// ```
    /// let mut context = TestContext::new();
    /// context.set_variable("token".to_string(), json!("eyJhbGc..."));
    /// ```
    pub fn set_variable(&mut self, key: String, value: Value) {
        self.variables.insert(key, value);
    }

    /// Получает значение переменной
    ///
    /// # Пример
    /// ```
    /// let token = context.get_variable("token");
    /// if let Some(t) = token {
    ///     println!("Token: {}", t);
    /// }
    /// ```
    pub fn get_variable(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }

    /// Проверяет, завершен ли этап
    pub fn is_stage_completed(&self, stage_name: &str) -> bool {
        self.completed_stages.contains(&stage_name.to_string())
    }

    /// Отмечает этап как завершенный и сохраняет результат
    pub fn stage_completed(&mut self, stage_name: String, result: StageResult) {
        self.completed_stages.push(stage_name.clone());
        self.stage_results.insert(stage_name, result);
    }

    /// Возвращает количество пройденных этапов
    pub fn passed_count(&self) -> usize {
        self.stage_results
            .values()
            .filter(|r| r.passed)
            .count()
    }

    /// Возвращает количество проваленных этапов
    pub fn failed_count(&self) -> usize {
        self.stage_results
            .values()
            .filter(|r| !r.passed)
            .count()
    }

    /// Очищает контекст для нового запуска
    pub fn reset(&mut self) {
        self.variables.clear();
        self.completed_stages.clear();
        self.stage_results.clear();
        self.started_at = Utc::now();
    }
}

/// Результат выполнения одного этапа (stage)
#[derive(Debug, Clone)]
pub struct StageResult {
    pub name: String,

    /// Прошел ли этап успешно
    pub passed: bool,

    /// Сколько времени выполнялся этап
    pub duration: std::time::Duration,

    /// HTTP статус ответа (если был)
    pub status: Option<u16>,

    /// Текст ошибки (если этап провалился)
    pub error: Option<String>,

    /// Значения, извлеченные из ответа
    pub extracted: HashMap<String, Value>,

    pub request: RequestInfo,

    pub response: ResponseInfo,
}

impl StageResult {
    /// Создает новый результат
    pub fn new(name: String) -> Self {
        StageResult {
            name,
            passed: false,
            duration: std::time::Duration::from_secs(0),
            status: None,
            error: None,
            extracted: HashMap::new(),
            request: RequestInfo {
                method: String::new(),
                url: String::new(),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
            },
            response: ResponseInfo {
                status: 0,
                headers: HashMap::new(),
                body: String::new(),
            },
        }
    }

    /// Создает успешный результат
    pub fn success(name: String) -> Self {
        StageResult {
            name,
            passed: true,
            duration: std::time::Duration::from_secs(0),
            status: None,
            error: None,
            extracted: HashMap::new(),
            request: RequestInfo {
                method: String::new(),
                url: String::new(),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
            },
            response: ResponseInfo {
                status: 0,
                headers: HashMap::new(),
                body: String::new(),
            },
        }
    }

    /// Создает результат с ошибкой
    pub fn failure(name: String, error: String) -> Self {
        StageResult {
            name,
            passed: false,
            duration: std::time::Duration::from_secs(0),
            status: None,
            error: Some(error),
            extracted: HashMap::new(),
            request: RequestInfo {
                method: String::new(),
                url: String::new(),
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
            },
            response: ResponseInfo {
                status: 0,
                headers: HashMap::new(),
                body: String::new(),
            },
        }
    }
}

/// Информация о запросе
#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

/// Информация об ответе
#[derive(Debug, Clone)]
pub struct ResponseInfo {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_new() {
        let context = TestContext::new();
        assert!(context.variables.is_empty());
        assert!(context.completed_stages.is_empty());
        assert!(context.stage_results.is_empty());
    }

    #[test]
    fn test_set_and_get_variable() {
        let mut context = TestContext::new();
        context.set_variable("token".to_string(), json!("abc123"));

        assert_eq!(context.get_variable("token"), Some(&json!("abc123")));
        assert_eq!(context.get_variable("nonexistent"), None);
    }

    #[test]
    fn test_stage_completed() {
        let mut context = TestContext::new();
        let result = StageResult::success("login".to_string());

        context.stage_completed("login".to_string(), result);

        assert!(context.is_stage_completed("login"));
        assert!(!context.is_stage_completed("logout"));
        assert_eq!(context.completed_stages.len(), 1);
    }

    #[test]
    fn test_passed_failed_count() {
        let mut context = TestContext::new();

        context.stage_completed("stage1".to_string(), StageResult::success("stage1".to_string()));
        context.stage_completed("stage2".to_string(), StageResult::failure("stage2".to_string(), "error".to_string()));
        context.stage_completed("stage3".to_string(), StageResult::success("stage3".to_string()));

        assert_eq!(context.passed_count(), 2);
        assert_eq!(context.failed_count(), 1);
    }

    #[test]
    fn test_context_reset() {
        let mut context = TestContext::new();
        context.set_variable("token".to_string(), json!("abc123"));
        context.stage_completed("login".to_string(), StageResult::success("login".to_string()));

        context.reset();

        assert!(context.variables.is_empty());
        assert!(context.completed_stages.is_empty());
        assert!(context.stage_results.is_empty());
    }
}