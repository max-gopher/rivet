//! Валидатор конфигураций
//!
//! Проверяет корректность конфигурации перед выполнением:
//! - Уникальные имена этапов
//! - Существование зависимостей
//! - Отсутствие циклических зависимостей
//! - Корректность настроек

use std::collections::{HashMap, HashSet};
use crate::parsers::config::models::{TestSuite};
use crate::error::{CoreError, CoreResult};
use crate::parsers::config::models::BackoffStrategy;

/// Валидатор конфигураций
///
/// Проверяет все аспекты конфигурации тестов
#[derive(Debug, Default)]
pub struct ConfigValidator;

impl ConfigValidator {
    /// Создает новый валидатор
    pub fn new() -> Self {
        Self
    }

    /// Проверяет всю конфигурацию
    ///
    /// # Пример
    /// ```
    /// let suite = load_config("test.yaml")?;
    /// ConfigValidator::validate(&suite)?;
    /// ```
    pub fn validate(suite: &TestSuite) -> CoreResult<()> {
        // 0. Проверка на пустые этапы
        Self::check_not_empty(suite)?;

        // 1. Проверяем уникальность имен этапов
        Self::check_unique_stage_names(suite)?;

        // 2. Проверяем, что все зависимости существуют
        Self::check_dependencies_exist(suite)?;

        // 3. Проверяем отсутствие циклических зависимостей
        Self::check_cycles(suite)?;

        // 4. Проверяем корректность настроек retry
        Self::check_retry_configs(suite)?;

        // 5. Проверяем корректность timeout
        Self::check_timeouts(suite)?;

        // 6. Проверяем корректность URL
        Self::check_urls(suite)?;

        Ok(())
    }

    // ============ ПРОВЕРКА 0: А ЕСТЬ ЛИ ШАГИ ============
    fn check_not_empty(suite: &TestSuite) -> CoreResult<()> {
        if suite.stages.is_empty() {
            return Err(CoreError::ValidationError(
                "No stages defined in config".to_string()
            ));
        }
        Ok(())
    }

    // ============ ПРОВЕРКА 1: УНИКАЛЬНЫЕ ИМЕНА ============

    /// Проверяет, что у всех этапов уникальные имена
    fn check_unique_stage_names(suite: &TestSuite) -> CoreResult<()> {
        let mut names = HashSet::new();

        for stage in &suite.stages {
            if !names.insert(&stage.name) {
                return Err(CoreError::ValidationError(
                    format!("Duplicate stage name: '{}'", stage.name)
                ));
            }
        }

        Ok(())
    }

    // ============ ПРОВЕРКА 2: СУЩЕСТВОВАНИЕ ЗАВИСИМОСТЕЙ ============

    /// Проверяет, что все указанные зависимости существуют
    fn check_dependencies_exist(suite: &TestSuite) -> CoreResult<()> {
        // Собираем все имена этапов
        let stage_names: HashSet<String> = suite.stages
            .iter()
            .map(|stage| stage.name.clone())
            .collect();

        // Проверяем каждую зависимость
        for stage in &suite.stages {
            for dep in &stage.depends_on {
                if !stage_names.contains(dep) {
                    return Err(CoreError::ValidationError(
                        format!(
                            "Stage '{}' depends on '{}' which doesn't exist",
                            stage.name, dep
                        )
                    ));
                }
            }
        }

        Ok(())
    }

    // ============ ПРОВЕРКА 3: ЦИКЛИЧЕСКИЕ ЗАВИСИМОСТИ ============

    /// Проверяет, что нет циклических зависимостей
    /// Например: A → B → C → A (плохо!)
    fn check_cycles(suite: &TestSuite) -> CoreResult<()> {
        // Строим граф зависимостей
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for stage in &suite.stages {
            graph.insert(stage.name.clone(), stage.depends_on.clone());
        }

        // Проверяем каждый узел
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for stage in &suite.stages {
            if !visited.contains(&stage.name) {
                Self::dfs(
                    &stage.name,
                    &graph,
                    &mut visited,
                    &mut recursion_stack,
                )?;
            }
        }

        Ok(())
    }

    /// Depth-First Search для обнаружения циклов
    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> CoreResult<()> {
        visited.insert(node.to_string());
        recursion_stack.insert(node.to_string());

        // Проверяем все зависимости
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    Self::dfs(neighbor, graph, visited, recursion_stack)?;
                } else if recursion_stack.contains(neighbor) {
                    return Err(CoreError::ValidationError(
                        format!("Circular dependency detected: {} → {}", node, neighbor)
                    ));
                }
            }
        }

        recursion_stack.remove(node);
        Ok(())
    }

    // ============ ПРОВЕРКА 4: НАСТРОЙКИ RETRY ============

    /// Проверяет корректность настроек повторных попыток
    fn check_retry_configs(suite: &TestSuite) -> CoreResult<()> {
        for stage in &suite.stages {
            if let Some(retry) = &stage.retry {
                if retry.count == 0 {
                    return Err(CoreError::ValidationError(
                        format!("Stage '{}' has retry count 0", stage.name)
                    ));
                }

                if retry.delay == 0 {
                    return Err(CoreError::ValidationError(
                        format!("Stage '{}' has retry delay 0", stage.name)
                    ));
                }

                // Проверяем бэкофф стратегию
                if let Some(backoff) = &retry.backoff {
                    match backoff {
                        BackoffStrategy::Exponential { factor } => {
                            if *factor <= 1.0 {
                                return Err(CoreError::ValidationError(
                                    format!(
                                        "Stage '{}' has invalid exponential factor: {} (must be > 1.0)",
                                        stage.name, factor
                                    )
                                ));
                            }
                        }
                        _ => {} // Fixed и Linear всегда корректны
                    }
                }
            }
        }

        Ok(())
    }

    // ============ ПРОВЕРКА 5: TIMEOUT ============

    /// Проверяет корректность таймаутов
    fn check_timeouts(suite: &TestSuite) -> CoreResult<()> {
        for stage in &suite.stages {
            if let Some(timeout) = stage.timeout {
                if timeout == 0 {
                    return Err(CoreError::ValidationError(
                        format!("Stage '{}' has timeout 0", stage.name)
                    ));
                }

                if timeout > 3600 { // 1 час
                    return Err(CoreError::ValidationError(
                        format!("Stage '{}' has timeout too large: {}s (max 3600s)",
                                stage.name, timeout)
                    ));
                }
            }
        }

        Ok(())
    }

    // ============ ПРОВЕРКА 6: URL ============

    /// Проверяет корректность URL
    fn check_urls(suite: &TestSuite) -> CoreResult<()> {
        for stage in &suite.stages {
            let url = &stage.request.url;

            // Проверяем, что URL не пустой
            if url.is_empty() {
                return Err(CoreError::ValidationError(
                    format!("Stage '{}' has empty URL", stage.name)
                ));
            }

            // Простая проверка URL (можно сделать более строгой)
            if !url.starts_with("http://") &&
                !url.starts_with("https://") &&
                !url.starts_with("{{") { // Шаблоны разрешены
                return Err(CoreError::ValidationError(
                    format!("Stage '{}' has invalid URL: '{}' (must start with http:// or https://)",
                            stage.name, url)
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::config::models::*;

    fn create_test_suite(stages: Vec<Stage>) -> TestSuite {
        TestSuite {
            name: "Test Suite".to_string(),
            version: None,
            description: None,
            variables: std::collections::HashMap::new(),
            env: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
            http: None,
            stages,
        }
    }

    fn create_stage(name: &str, depends_on: Vec<&str>) -> Stage {
        Stage {
            id: None,
            name: name.to_string(),
            description: None,
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            skip: None,
            retry: None,
            timeout: None,
            request: Request {
                method: HttpMethod::Get,
                url: "https://api.example.com".to_string(),
                headers: std::collections::HashMap::new(),
                params: std::collections::HashMap::new(),
                body: None,
                auth: None,
                validate_ssl: None,
            },
            extract: vec![],
            assert: vec![],
            tags: vec![],
        }
    }

    #[test]
    fn test_valid_config() {
        let stages = vec![
            create_stage("Login", vec![]),
            create_stage("GetProfile", vec!["Login"]),
            create_stage("Logout", vec!["Login"]),
        ];
        let suite = create_test_suite(stages);

        let result = ConfigValidator::validate(&suite);
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_names() {
        let stages = vec![
            create_stage("Login", vec![]),
            create_stage("Login", vec![]), // Дубликат!
        ];
        let suite = create_test_suite(stages);

        let result = ConfigValidator::validate(&suite);
        assert!(result.is_err());

        match result {
            Err(CoreError::ValidationError(msg)) => {
                assert!(msg.contains("Duplicate stage name"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_nonexistent_dependency() {
        let stages = vec![
            create_stage("Login", vec![]),
            create_stage("GetProfile", vec!["Login", "Auth"]), // Auth не существует!
        ];
        let suite = create_test_suite(stages);

        let result = ConfigValidator::validate(&suite);
        assert!(result.is_err());

        match result {
            Err(CoreError::ValidationError(msg)) => {
                assert!(msg.contains("depends on 'Auth'"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_circular_dependency() {
        let stages = vec![
            create_stage("A", vec!["B"]),
            create_stage("B", vec!["C"]),
            create_stage("C", vec!["A"]), // Цикл: A → B → C → A
        ];
        let suite = create_test_suite(stages);

        let result = ConfigValidator::validate(&suite);
        assert!(result.is_err());

        match result {
            Err(CoreError::ValidationError(msg)) => {
                assert!(msg.contains("Circular dependency"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_invalid_retry_config() {
        let mut stage = create_stage("Login", vec![]);
        stage.retry = Some(RetryConfig {
            count: 0, // Невалидно!
            delay: 1000,
            backoff: None,
        });

        let suite = create_test_suite(vec![stage]);
        let result = ConfigValidator::validate(&suite);

        assert!(result.is_err());
        match result {
            Err(CoreError::ValidationError(msg)) => {
                assert!(msg.contains("retry count 0"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_invalid_url() {
        let mut stage = create_stage("Login", vec![]);
        stage.request.url = "not_a_url".to_string();

        let suite = create_test_suite(vec![stage]);
        let result = ConfigValidator::validate(&suite);

        assert!(result.is_err());
        match result {
            Err(CoreError::ValidationError(msg)) => {
                assert!(msg.contains("invalid URL"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }
}