//! Основной движок выполнения тестов
//!
//! Отвечает за:
//! - Выполнение всех этапов тестирования
//! - Управление контекстом (переменные, результаты)
//! - Валидацию конфигурации
//! - Обработку зависимостей между этапами

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::parsers::config::*;
use crate::parsers::config::ConfigValidator;
use crate::parsers::response::ResponseExtractor;
use crate::http::{HttpClient, RequestBuilder};
use crate::context::{TestContext, StageResult, RequestInfo, ResponseInfo};
use crate::error::{CoreError, CoreResult};

/// Основной движок выполнения тестов
pub struct TestEngine {
    client: HttpClient,
    request_builder: RequestBuilder,
    context: Arc<Mutex<TestContext>>,
}

impl TestEngine {
    /// Создает новый движок с дефолтным HTTP клиентом
    pub fn new() -> Self {
        Self::with_http_config(None)
    }

    /// Создает движок с настройками HTTP из конфига
    pub fn with_http_config(http_config: Option<&HttpConfig>) -> Self {
        let client = if let Some(config) = http_config {
            HttpClient::new(config.clone())
        } else {
            HttpClient::default()
        };

        TestEngine {
            client,
            request_builder: RequestBuilder::new(),
            context: Arc::new(Mutex::new(TestContext::new())),
        }
    }

    /// Запускает выполнение тестов и возвращает детальные результаты
    pub async fn run_detailed(&self, suite: &TestSuite) -> CoreResult<Vec<StageResult>> {
        // 1. Валидация конфига
        ConfigValidator::validate(suite)?;

        // 2. Инициализация переменных
        {
            let mut context = self.context.lock().await;
            for (key, value) in &suite.variables {
                context.set_variable(key.clone(), value.clone());
            }
        }

        info!("Starting test suite: {}", suite.name);

        // 3. Топологическая сортировка этапов
        let sorted_stages = self.topological_sort(&suite.stages)?;

        // 4. Выполнение этапов
        for stage in sorted_stages {
            if let Some(true) = stage.skip {
                info!("Skipping stage: {}", stage.name);
                continue;
            }

            if !self.check_dependencies(&stage).await {
                warn!("Stage '{}' dependencies not met", stage.name);
                let result = StageResult::failure(
                    stage.name.clone(),
                    format!("Dependencies not met: {:?}", stage.depends_on)
                );
                let mut context = self.context.lock().await;
                context.stage_completed(stage.name.clone(), result);
                continue;
            }

            let result = self.execute_stage(&stage).await?;
            let mut context = self.context.lock().await;
            context.stage_completed(stage.name.clone(), result);
        }

        // Собираем и возвращаем все результаты
        let context = self.context.lock().await;
        let results: Vec<StageResult> = context.stage_results.values().cloned().collect();

        info!("Test suite complete: {}/{} passed",
            results.iter().filter(|r| r.passed).count(),
            results.len()
        );

        Ok(results)
    }

    /// Запускает выполнение тестов и возвращает только статус (успешно/неуспешно)
    pub async fn run(&self, suite: &TestSuite) -> CoreResult<bool> {
        let results = self.run_detailed(suite).await?;
        Ok(results.iter().all(|r| r.passed))
    }

    /// Выполняет один этап
    async fn execute_stage(&self, stage: &Stage) -> CoreResult<StageResult> {
        let start = std::time::Instant::now();

        // Строим HTTP запрос (контекст освобождается после блока)
        let http_request = {
            let context = self.context.lock().await;
            self.request_builder.build(stage, &context.variables)?
        };

        // --- Сохраняем информацию о запросе ---
        let request_info = RequestInfo {
            method: http_request.method.clone(),
            url: http_request.url.clone(),
            headers: http_request.headers.clone(),
            params: http_request.params.clone(),
            body: http_request.body.clone(),
        };

        let response = self.client.execute_with_retry(http_request).await?;

        // --- Сохраняем информацию об ответе ---
        let response_info = ResponseInfo {
            status: response.status,
            headers: response.headers.clone(),
            body: response.body.clone(),
        };

        let status = response.status;
        let headers = response.headers;
        let headers_map = response.headers_map;
        let body_json = response.json_body;
        let _body_text = response.body;

        let mut result = StageResult::new(stage.name.clone());
        result.status = Some(status);
        result.request = request_info;
        result.response = response_info;

        let mut errors = Vec::new();
        for assert in &stage.assert {
            if let Err(e) = self.validate_assertion(assert, &body_json, status, &headers) {
                errors.push(e.to_string());
            }
        }

        if errors.is_empty() {
            result.passed = true;
        } else {
            result.passed = false;
            result.error = Some(errors.join("; "));
        }

        if !stage.extract.is_empty() {
            let extracted = ResponseExtractor::extract(
                &body_json,
                &headers_map,
                status,
                &stage.extract,
            )?;

            // Сохраняем переменные в контекст (отдельный блок)
            {
                let mut context = self.context.lock().await;
                for (key, value) in &extracted {
                    context.set_variable(key.clone(), value.clone());
                }
            }

            result.extracted = extracted;
        }

        result.duration = start.elapsed();
        Ok(result)
    }

    /// Валидирует assertion
    fn validate_assertion(
        &self,
        assert: &Assert,
        body: &serde_json::Value,
        status: u16,
        headers: &HashMap<String, String>,
    ) -> CoreResult<()> {
        match assert {
            Assert::Status(status_assert) => {

                if status != status_assert.status {
                    return Err(CoreError::ValidationError(
                        format!("Expected status {}, got {}", status_assert.status, status)
                    ));
                }
            }
            Assert::Body(body_assert) => {
                // Всегда используем JsonPathParser
                let value = crate::parsers::response::JsonPathParser::extract_first(body, &body_assert.path)?;

                if let Some(value) = value {
                    if let Some(expected) = &body_assert.equals {
                        if value != expected {
                            return Err(CoreError::ValidationError(
                                format!("Expected {} to equal {:?}, got {:?}",
                                        body_assert.path, expected, value)
                            ));
                        }
                    }

                    if let Some(not_null) = body_assert.not_null {
                        if not_null && value.is_null() {
                            return Err(CoreError::ValidationError(
                                format!("Expected {} to not be null", body_assert.path)
                            ));
                        }
                    }

                    if let Some(regex) = &body_assert.regex {
                        if let Some(str_value) = value.as_str() {
                            let re = regex::Regex::new(regex)
                                .map_err(|e| CoreError::ValidationError(e.to_string()))?;
                            if !re.is_match(str_value) {
                                return Err(CoreError::ValidationError(
                                    format!("Value '{}' doesn't match regex '{}'", str_value, regex)
                                ));
                            }
                        }
                    }
                } else {
                    return Err(CoreError::ValidationError(
                        format!("Path '{}' not found in response", body_assert.path)
                    ));
                }
            }
            Assert::Header(header_assert) => {
                if let Some(value) = headers.get(&header_assert.header) {
                    if let Some(expected) = &header_assert.equals {
                        if value != expected {
                            return Err(CoreError::ValidationError(
                                format!("Expected header {} to equal {}, got {}",
                                        header_assert.header, expected, value)
                            ));
                        }
                    }
                } else if let Some(true) = header_assert.exists {
                    return Err(CoreError::ValidationError(
                        format!("Header '{}' not found", header_assert.header)
                    ));
                }
            }
            Assert::And(asserts) => {
                for assert in asserts {
                    self.validate_assertion(assert, body, status, headers)?;
                }
            }
            Assert::Or(asserts) => {
                let mut any_passed = false;
                let mut errors = Vec::new();
                for assert in asserts {
                    match self.validate_assertion(assert, body, status, headers) {
                        Ok(()) => { any_passed = true; break; }
                        Err(e) => errors.push(e.to_string()),
                    }
                }
                if !any_passed {
                    return Err(CoreError::ValidationError(
                        format!("All OR assertions failed: {}", errors.join("; "))
                    ));
                }
            }
            Assert::Not(assert) => {
                match self.validate_assertion(assert, body, status, headers) {
                    Ok(()) => {
                        return Err(CoreError::ValidationError(
                            "Assertion should have failed but passed".to_string()
                        ));
                    }
                    Err(_) => {} // Ожидаемая ошибка - все хорошо
                }
            }
            Assert::Custom(_) => {
                return Err(CoreError::ValidationError(
                    "Custom assertions not implemented yet".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Проверяет зависимости этапа
    async fn check_dependencies(&self, stage: &Stage) -> bool {
        if stage.depends_on.is_empty() {
            return true;
        }

        let context = self.context.lock().await;
        for dep in &stage.depends_on {
            if !context.is_stage_completed(dep) {
                return false;
            }
        }
        true
    }

    /// Топологическая сортировка этапов
    fn topological_sort(&self, stages: &[Stage]) -> CoreResult<Vec<Stage>> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut temp = HashSet::new();

        for stage in stages {
            if !visited.contains(&stage.name.as_str()) {
                self.visit(stage, stages, &mut visited, &mut temp, &mut sorted)?;
            }
        }

        Ok(sorted)
    }

    /// Рекурсивный обход для топологической сортировки
    fn visit<'a>(
        &self,
        stage: &'a Stage,
        stages: &'a [Stage],
        visited: &mut HashSet<&'a str>,
        temp: &mut HashSet<&'a str>,
        sorted: &mut Vec<Stage>,
    ) -> CoreResult<()> {
        if temp.contains(stage.name.as_str()) {
            return Err(CoreError::DependencyError(
                format!("Circular dependency detected at stage: {}", stage.name)
            ));
        }

        if visited.contains(stage.name.as_str()) {
            return Ok(());
        }

        temp.insert(&stage.name);

        for dep in &stage.depends_on {
            if let Some(dep_stage) = stages.iter().find(|s| &s.name == dep) {
                self.visit(dep_stage, stages, visited, temp, sorted)?;
            } else {
                return Err(CoreError::DependencyError(
                    format!("Stage '{}' depends on unknown stage '{}'", stage.name, dep)
                ));
            }
        }

        temp.remove(&stage.name.as_str());
        visited.insert(&stage.name);
        sorted.push(stage.clone());

        Ok(())
    }
}

impl Default for TestEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_request() -> Request {
        Request {
            method: HttpMethod::Get,
            url: "https://api.example.com".to_string(),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: None,
            auth: None,
            validate_ssl: None,
        }
    }

    fn create_test_stage(name: &str, depends_on: Vec<&str>) -> Stage {
        Stage {
            id: None,
            name: name.to_string(),
            description: None,
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            skip: None,
            retry: None,
            timeout: None,
            request: create_test_request(),
            extract: vec![],
            assert: vec![],
            tags: vec![],
        }
    }

    #[test]
    fn test_topological_sort() {
        let stages = vec![
            create_test_stage("A", vec![]),
            create_test_stage("B", vec!["A"]),
            create_test_stage("C", vec!["A", "B"]),
        ];

        let engine = TestEngine::new();
        let sorted = engine.topological_sort(&stages).unwrap();

        // Проверяем порядок: сначала A, потом B, потом C
        assert_eq!(sorted[0].name, "A");
        assert_eq!(sorted[1].name, "B");
        assert_eq!(sorted[2].name, "C");
    }

    #[test]
    fn test_circular_dependency_detection() {
        let stages = vec![
            create_test_stage("A", vec!["B"]),
            create_test_stage("B", vec!["C"]),
            create_test_stage("C", vec!["A"]),
        ];

        let engine = TestEngine::new();
        let result = engine.topological_sort(&stages);

        assert!(result.is_err());
        match result {
            Err(CoreError::DependencyError(msg)) => {
                assert!(msg.contains("Circular dependency"));
            }
            _ => panic!("Expected DependencyError"),
        }
    }

    #[test]
    fn test_unknown_dependency() {
        let stages = vec![
            create_test_stage("A", vec![]),
            create_test_stage("B", vec!["A", "C"]),
        ];

        let engine = TestEngine::new();
        let result = engine.topological_sort(&stages);

        assert!(result.is_err());
        match result {
            Err(CoreError::DependencyError(msg)) => {
                assert!(msg.contains("depends on unknown stage"));
            }
            _ => panic!("Expected DependencyError"),
        }
    }
}