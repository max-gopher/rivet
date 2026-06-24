//! Модуль конфигурации тестов
//!
//! Отвечает за:
//! - Загрузку YAML файлов (loader)
//! - Валидацию конфигурации (validator)
//! - Модели данных (models)

mod models;
mod loader;
mod validator;

// Публичный экспорт основных типов
pub use models::*;
pub use loader::ConfigLoader;
pub use validator::ConfigValidator;

// Импорты для удобных функций
use crate::error::CoreResult;

/// Загружает и валидирует конфигурацию из файла
///
/// Это основная функция для работы с конфигами.
/// Объединяет загрузку и валидацию в один шаг.
///
/// # Пример
/// ```
/// use rivet_core::parsers::config::load_and_validate;
///
/// let suite = load_and_validate("tests/my_test.yaml")?;
/// println!("Loaded suite: {}", suite.name);
/// ```
pub fn load_and_validate_from_file<P: AsRef<std::path::Path>>(path: P) -> CoreResult<TestSuite> {
    let loader = ConfigLoader::new();
    loader.load_and_validate_from_file(path)
}

/// Загружает и валидирует конфигурацию из строки
///
/// Полезно для тестов или когда конфиг приходит откуда-то еще.
///
/// # Пример
/// ```
/// let yaml = r#"
/// name: "My Test"
/// stages: []
/// "#;
/// let suite = load_and_validate_from_str(yaml)?;
/// ```
pub fn load_and_validate_from_str(content: &str) -> CoreResult<TestSuite> {
    let loader = ConfigLoader::new();
    loader.load_and_validate_from_str(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_validate_function() {
        let yaml = r#"
name: "Test Suite"
stages:
  - name: "Test"
    request:
      method: GET
      url: "https://api.example.com"
"#;

        let result = load_and_validate_from_str(yaml);
        assert!(result.is_ok());

        let suite = result.unwrap();
        assert_eq!(suite.name, "Test Suite");
        assert_eq!(suite.stages.len(), 1);
    }

    #[test]
    fn test_load_and_validate_invalid() {
        let yaml = r#"
name: "Test Suite"
stages:
  - name: "Test"
  - name: "Test"
"#;

        let result = load_and_validate_from_str(yaml);
        assert!(result.is_err());
    }
}