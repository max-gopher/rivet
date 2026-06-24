//! Типы ошибок для библиотеки Rivet Core
//!
//! Определяет все возможные ошибки, которые могут возникнуть:
//! - Ошибки конфигурации
//! - Ошибки HTTP
//! - Ошибки парсинга
//! - Ошибки валидации

use thiserror::Error;

/// Главный тип ошибки для всей библиотеки
#[derive(Error, Debug)]
pub enum CoreError {
    /// Ошибка конфигурации (неправильный YAML, отсутствуют поля)
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Ошибка HTTP (проблемы с сетью, сервер не отвечает)
    #[error("HTTP error: {0}")]
    HttpError(String),

    /// Ошибка парсинга (не удалось разобрать JSON или YAML)
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Ошибка шаблонизации (не удалось подставить переменные)
    #[error("Template error: {0}")]
    TemplateError(String),

    /// Ошибка валидации (неправильные данные, циклические зависимости)
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Ошибка выполнения теста (что-то пошло не так во время теста)
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Ошибка зависимостей (этап зависит от другого, но тот не выполнен)
    #[error("Dependency error: {0}")]
    DependencyError(String),

    /// Ошибка ввода-вывода (не удалось прочитать файл)
    #[error("IO error: {0}")]
    IoError(String),

    /// Ошибка YAML (неправильный формат YAML)
    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yml::Error),

    /// Ошибка JSON (неправильный формат JSON)
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Внутренняя ошибка (непредвиденная ситуация)
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Удобный тип для возврата результатов
/// Используется вместо `Result<T, anyhow::Error>`
pub type CoreResult<T> = Result<T, CoreError>;