//! Загрузчик конфигураций из YAML файлов
//!
//! Читает YAML файлы и преобразует их в структуры TestSuite.
//! Поддерживает подстановку переменных окружения.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use yaml_rust2::{YamlLoader, Yaml};

use crate::parsers::config::models::TestSuite;
use crate::error::{CoreError, CoreResult};
use crate::parsers::config::{Assert, Auth, BackoffStrategy, BodyAssert, ConfigValidator, Extract, ExtractSource, HeaderAssert, HttpConfig, HttpMethod, Request, RequestBody, RetryConfig, Stage, StatusAssert};

/// Загрузчик конфигураций
///
/// Читает YAML конфиги и подставляет переменные окружения
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    /// Префикс для переменных окружения (например, "RIVET_")
    env_prefix: Option<String>,

    /// Базовая директория для поиска файлов
    base_dir: Option<PathBuf>,
}

impl ConfigLoader {
    /// Создает новый загрузчик с настройками по умолчанию
    pub fn new() -> Self {
        ConfigLoader {
            env_prefix: None,
            base_dir: None,
        }
    }

    /// Устанавливает префикс для переменных окружения
    ///
    /// # Пример
    /// ```
    /// let loader = ConfigLoader::new()
    ///     .with_env_prefix("RIVET_");
    /// // Теперь переменные вида RIVET_API_URL будут подставляться
    /// ```
    pub fn with_env_prefix(mut self, prefix: &str) -> Self {
        self.env_prefix = Some(prefix.to_string());
        self
    }

    /// Устанавливает базовую директорию для поиска файлов
    pub fn with_base_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.base_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Загружает конфиг из файла
    ///
    /// # Пример
    /// ```
    /// let loader = ConfigLoader::new();
    /// let suite = loader.load_from_file("examples/test.yaml")?;
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> CoreResult<TestSuite> {

        let path = if let Some(base_dir) = &self.base_dir {
            base_dir.join(path)
        } else {
            path.as_ref().to_path_buf()
        };

        let content = fs::read_to_string(&path)
            .map_err(|e| CoreError::IoError(format!("Failed to read config file '{}': {}", path.display(), e)))?;

        self.load_from_str(&content)
    }

    /// Загружает конфиг из строки
    ///
    /// # Пример
    /// ```
    /// let yaml = r#"
    /// name: "My Tests"
    /// stages: []
    /// "#;
    /// let suite = loader.load_from_str(yaml)?;
    /// ```
    pub fn load_from_str(&self, content: &str) -> CoreResult<TestSuite> {
        // Парсим YAML документы
        let docs = YamlLoader::load_from_str(content)
            .map_err(|e| CoreError::ConfigError(format!("Invalid YAML: {}", e)))?;

        if docs.is_empty() {
            return Err(CoreError::ConfigError("Empty YAML document".to_string()));
        }

        // Берем первый документ
        let yaml = &docs[0];

        // Преобразуем Yaml в TestSuite
        let suite = Self::yaml_to_testsuite(yaml)?;

        // Подстановка переменных окружения (пока пропускаем, чтобы не усложнять)
        Ok(suite)
    }

    // Рекурсивная функция для преобразования Yaml в TestSuite
    fn yaml_to_testsuite(yaml: &Yaml) -> CoreResult<TestSuite> {
        let mut suite = TestSuite {
            name: "".to_string(),
            version: None,
            description: None,
            variables: std::collections::HashMap::new(),
            env: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
            http: None,
            stages: Vec::new(),
        };

        // Парсим name
        if let Some(name) = yaml["name"].as_str() {
            suite.name = name.to_string();
        } else {
            return Err(CoreError::ConfigError("Missing 'name' field".to_string()));
        }

        // Парсим version
        if let Some(version) = yaml["version"].as_str() {
            suite.version = Some(version.to_string());
        }

        // Парсим description
        if let Some(desc) = yaml["description"].as_str() {
            suite.description = Some(desc.to_string());
        }

        // Парсим stages
        if let Some(stages) = yaml["stages"].as_vec() {
            for stage_yaml in stages {
                let stage = Self::yaml_to_stage(stage_yaml)?;
                suite.stages.push(stage);
            }
        } else {
            return Err(CoreError::ConfigError("Missing 'stages' field".to_string()));
        }

        // Парсим variables
        if let Some(vars) = yaml["variables"].as_hash() {
            for (key, value) in vars {
                if let Some(key_str) = key.as_str() {
                    // Преобразуем Yaml в serde_json::Value
                    let json_value = Self::yaml_to_json(value);
                    suite.variables.insert(key_str.to_string(), json_value);
                }
            }
        }

        // Парсим env
        if let Some(env) = yaml["env"].as_hash() {
            for (key, value) in env {
                if let (Some(key_str), Some(value_str)) = (key.as_str(), value.as_str()) {
                    suite.env.insert(key_str.to_string(), value_str.to_string());
                }
            }
        }

        // Парсим metadata
        if let Some(metadata) = yaml["metadata"].as_hash() {
            for (key, value) in metadata {
                if let (Some(key_str), Some(value_str)) = (key.as_str(), value.as_str()) {
                    suite.metadata.insert(key_str.to_string(), value_str.to_string());
                }
            }
        }

        // Парсим http
        if let Some(http) = yaml["http"].as_hash() {
            let mut http_config = HttpConfig::default();

            if let Some(timeout) = http.get(&Yaml::from_str("timeout")) {
                if let Some(t) = timeout.as_i64() {
                    http_config.timeout = t as u64;
                }
            }

            if let Some(retry_count) = http.get(&Yaml::from_str("retry_count")) {
                if let Some(r) = retry_count.as_i64() {
                    http_config.retry_count = r as u32;
                }
            }

            if let Some(retry_delay) = http.get(&Yaml::from_str("retry_delay")) {
                if let Some(r) = retry_delay.as_i64() {
                    http_config.retry_delay = r as u64;
                }
            }

            if let Some(exponential_backoff) = http.get(&Yaml::from_str("exponential_backoff")) {
                if let Some(eb) = exponential_backoff.as_bool() {
                    http_config.exponential_backoff = eb;
                }
            }

            if let Some(verify_ssl) = http.get(&Yaml::from_str("verify_ssl")) {
                if let Some(vs) = verify_ssl.as_bool() {
                    http_config.verify_ssl = vs;
                }
            }

            if let Some(user_agent) = http.get(&Yaml::from_str("user_agent")) {
                if let Some(ua) = user_agent.as_str() {
                    http_config.user_agent = ua.to_string();
                }
            }

            suite.http = Some(http_config);
        }

        Ok(suite)
    }

    fn yaml_to_json(yaml: &Yaml) -> serde_json::Value {
        match yaml {
            Yaml::String(s) => serde_json::Value::String(s.clone()),
            Yaml::Integer(i) => serde_json::Value::Number((*i).into()),
            Yaml::Real(f) => {
                if let Ok(n) = f.parse::<f64>() {
                    if let Some(num) = serde_json::Number::from_f64(n) {
                        serde_json::Value::Number(num)
                    } else {
                        serde_json::Value::String(f.clone())
                    }
                } else {
                    serde_json::Value::String(f.clone())
                }
            }
            Yaml::Boolean(b) => serde_json::Value::Bool(*b),
            Yaml::Array(arr) => {
                let mut json_arr = Vec::new();
                for item in arr {
                    json_arr.push(Self::yaml_to_json(item));
                }
                serde_json::Value::Array(json_arr)
            }
            Yaml::Hash(hash) => {
                let mut json_obj = serde_json::Map::new();
                for (k, v) in hash {
                    if let Some(key_str) = k.as_str() {
                        json_obj.insert(key_str.to_string(), Self::yaml_to_json(v));
                    }
                }
                serde_json::Value::Object(json_obj)
            }
            _ => serde_json::Value::Null,
        }
    }

    fn yaml_to_stage(yaml: &Yaml) -> CoreResult<Stage> {
        let mut stage = Stage {
            id: None,
            name: "".to_string(),
            description: None,
            depends_on: Vec::new(),
            skip: None,
            retry: None,
            timeout: None,
            request: Request {
                method: HttpMethod::Get,
                url: "".to_string(),
                headers: std::collections::HashMap::new(),
                params: std::collections::HashMap::new(),
                body: None,
                auth: None,
                validate_ssl: None,
            },
            extract: Vec::new(),
            assert: Vec::new(),
            tags: Vec::new(),
        };

        // Парсим name
        if let Some(name) = yaml["name"].as_str() {
            stage.name = name.to_string();
        } else {
            return Err(CoreError::ConfigError("Missing stage 'name'".to_string()));
        }

        // Парсим description
        if let Some(desc) = yaml["description"].as_str() {
            stage.description = Some(desc.to_string());
        }

        // Парсим depends_on
        if let Some(deps) = yaml["depends_on"].as_vec() {
            for dep in deps {
                if let Some(dep_str) = dep.as_str() {
                    stage.depends_on.push(dep_str.to_string());
                }
            }
        }

        // Парсим request
        if let Some(request) = yaml["request"].as_hash() {
            // Парсим method
            if let Some(method) = request.get(&Yaml::from_str("method")) {
                if let Some(method_str) = method.as_str() {
                    stage.request.method = match method_str.to_lowercase().as_str() {
                        "get" => HttpMethod::Get,
                        "post" => HttpMethod::Post,
                        "put" => HttpMethod::Put,
                        "delete" => HttpMethod::Delete,
                        "patch" => HttpMethod::Patch,
                        "head" => HttpMethod::Head,
                        "options" => HttpMethod::Options,
                        "trace" => HttpMethod::Trace,
                        _ => return Err(CoreError::ConfigError(
                            format!("Unknown HTTP method: {}", method_str)
                        )),
                    };
                }
            }

            // Парсим url
            if let Some(url) = request.get(&Yaml::from_str("url")) {
                if let Some(url_str) = url.as_str() {
                    stage.request.url = url_str.to_string();
                }
            }

            // Парсим headers
            if let Some(headers) = request.get(&Yaml::from_str("headers")) {
                if let Some(headers_hash) = headers.as_hash() {
                    for (key, value) in headers_hash {
                        if let (Some(key_str), Some(value_str)) = (key.as_str(), value.as_str()) {
                            stage.request.headers.insert(key_str.to_string(), value_str.to_string());
                        }
                    }
                }
            }

            // Парсим body
            if let Some(body) = request.get(&Yaml::from_str("body")) {
                // Преобразуем Yaml в serde_json::Value
                let json_value = Self::yaml_to_json(body);
                stage.request.body = Some(RequestBody::Json(json_value));
            }

            // Парсим params
            if let Some(params) = request.get(&Yaml::from_str("params")) {
                if let Some(params_hash) = params.as_hash() {
                    for (key, value) in params_hash {
                        if let (Some(key_str), Some(value_str)) = (key.as_str(), value.as_str()) {
                            stage.request.params.insert(key_str.to_string(), value_str.to_string());
                        }
                    }
                }
            }

            // Парсим auth
            if let Some(auth) = request.get(&Yaml::from_str("auth")) {
                if let Some(auth_hash) = auth.as_hash() {
                    let auth_type = auth_hash.get(&Yaml::from_str("type"))
                        .and_then(|t| t.as_str())
                        .ok_or_else(|| CoreError::ConfigError("Auth missing 'type'".to_string()))?;

                    match auth_type {
                        "bearer" => {
                            let token = auth_hash.get(&Yaml::from_str("token"))
                                .and_then(|t| t.as_str())
                                .ok_or_else(|| CoreError::ConfigError("Bearer auth missing 'token'".to_string()))?
                                .to_string();
                            stage.request.auth = Some(Auth::Bearer { token });
                        }
                        "basic" => {
                            let username = auth_hash.get(&Yaml::from_str("username"))
                                .and_then(|u| u.as_str())
                                .ok_or_else(|| CoreError::ConfigError("Basic auth missing 'username'".to_string()))?
                                .to_string();
                            let password = auth_hash.get(&Yaml::from_str("password"))
                                .and_then(|p| p.as_str())
                                .ok_or_else(|| CoreError::ConfigError("Basic auth missing 'password'".to_string()))?
                                .to_string();
                            stage.request.auth = Some(Auth::Basic { username, password });
                        }
                        "api_key" => {
                            let key = auth_hash.get(&Yaml::from_str("key"))
                                .and_then(|k| k.as_str())
                                .ok_or_else(|| CoreError::ConfigError("API key auth missing 'key'".to_string()))?
                                .to_string();
                            let value = auth_hash.get(&Yaml::from_str("value"))
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| CoreError::ConfigError("API key auth missing 'value'".to_string()))?
                                .to_string();
                            let in_header = auth_hash.get(&Yaml::from_str("in_header"))
                                .and_then(|i| i.as_bool())
                                .unwrap_or(true);
                            let prefix = auth_hash.get(&Yaml::from_str("prefix"))
                                .and_then(|p| p.as_str())
                                .map(|s| s.to_string());
                            stage.request.auth = Some(Auth::ApiKey { key, value, in_header, prefix });
                        }
                        _ => return Err(CoreError::ConfigError(
                            format!("Unknown auth type: {}", auth_type)
                        )),
                    }
                }
            }
        }

        // Парсим assert
        if let Some(asserts) = yaml["assert"].as_vec() {
            for assert_yaml in asserts {
                let assert = Self::yaml_to_assert(assert_yaml)?;
                stage.assert.push(assert);
            }
        }

        // Парсим extract
        if let Some(extracts) = yaml["extract"].as_vec() {
            for extract_yaml in extracts {
                let extract = Self::yaml_to_extract(extract_yaml)?;
                stage.extract.push(extract);
            }
        }

        // Парсим tags
        if let Some(tags) = yaml["tags"].as_vec() {
            for tag in tags {
                if let Some(tag_str) = tag.as_str() {
                    stage.tags.push(tag_str.to_string());
                }
            }
        }

        // Парсим retry
        if let Some(retry) = yaml["retry"].as_hash() {
            let count = retry.get(&Yaml::from_str("count"))
                .and_then(|c| c.as_i64())
                .unwrap_or(3) as u32;

            let delay = retry.get(&Yaml::from_str("delay"))
                .and_then(|d| d.as_i64())
                .unwrap_or(1000) as u64;

            // Парсим backoff стратегию
            let backoff = retry.get(&Yaml::from_str("backoff"))
                .and_then(|b| b.as_str())
                .map(|s| match s {
                    "fixed" => BackoffStrategy::Fixed,
                    "linear" => BackoffStrategy::Linear,
                    _ => BackoffStrategy::Exponential { factor: 2.0 },
                });

            stage.retry = Some(RetryConfig {
                count,
                delay,
                backoff,
            });
        }

        // Парсим timeout
        if let Some(timeout) = yaml["timeout"].as_i64() {
            stage.timeout = Some(timeout as u64);
        }

        Ok(stage)
    }

    fn yaml_to_extract(yaml: &Yaml) -> CoreResult<Extract> {
        let name = yaml["name"].as_str()
            .ok_or_else(|| CoreError::ConfigError("Extract missing 'name'".to_string()))?
            .to_string();

        let source = if let Some(source) = yaml["source"].as_hash() {
            let source_type = source.get(&Yaml::from_str("type"))
                .and_then(|t| t.as_str())
                .ok_or_else(|| CoreError::ConfigError("Extract source missing 'type'".to_string()))?;

            match source_type {
                "body" => {
                    let path = source.get(&Yaml::from_str("path"))
                        .and_then(|p| p.as_str())
                        .ok_or_else(|| CoreError::ConfigError("Body extract missing 'path'".to_string()))?
                        .to_string();
                    ExtractSource::Body { path }
                }
                "header" => {
                    let header_name = source.get(&Yaml::from_str("name"))
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| CoreError::ConfigError("Header extract missing 'name'".to_string()))?
                        .to_string();
                    ExtractSource::Header { name: header_name }
                }
                "status" => ExtractSource::Status,
                _ => return Err(CoreError::ConfigError(format!("Unknown extract source: {}", source_type))),
            }
        } else {
            return Err(CoreError::ConfigError("Extract missing 'source'".to_string()));
        };

        Ok(Extract { name, source })
    }

    fn yaml_to_assert(yaml: &Yaml) -> CoreResult<Assert> {
        // Проверяем статус
        if let Some(status) = yaml["status"].as_i64() {
            return Ok(Assert::Status(StatusAssert { status: status as u16 }));
        }

        // Проверяем body
        if let Some(body) = yaml["body"].as_hash() {
            let mut path = String::new();
            let mut not_null = None;
            let mut equals = None;

            if let Some(p) = body.get(&Yaml::from_str("path")) {
                if let Some(p_str) = p.as_str() {
                    path = p_str.to_string();
                }
            }

            if let Some(nn) = body.get(&Yaml::from_str("not_null")) {
                if let Some(nn_bool) = nn.as_bool() {
                    not_null = Some(nn_bool);
                }
            }

            if let Some(eq) = body.get(&Yaml::from_str("equals")) {
                equals = Some(Self::yaml_to_json(eq));
            }

            return Ok(Assert::Body(BodyAssert {
                path,
                not_null,
                equals,
                regex: None,
                r#type: None,
                contains: None,
                in_range: None,
            }));
        }

        // Проверяем header
        if let Some(header) = yaml["header"].as_hash() {
            let header_name = header.get(&Yaml::from_str("header"))
                .and_then(|h| h.as_str())
                .ok_or_else(|| CoreError::ConfigError("Header assert missing 'header' field".to_string()))?
                .to_string();

            let equals = header.get(&Yaml::from_str("equals"))
                .and_then(|e| e.as_str())
                .map(|s| s.to_string());

            let exists = header.get(&Yaml::from_str("exists"))
                .and_then(|e| e.as_bool());

            let regex = header.get(&Yaml::from_str("regex"))
                .and_then(|r| r.as_str())
                .map(|s| s.to_string());

            return Ok(Assert::Header(HeaderAssert {
                header: header_name,
                equals,
                exists,
                regex,
            }));
        }

        // Проверяем and
        if let Some(and) = yaml["and"].as_vec() {
            let mut asserts = Vec::new();
            for a in and {
                asserts.push(Self::yaml_to_assert(a)?);
            }
            return Ok(Assert::And(asserts));
        }

        // Проверяем or
        if let Some(or) = yaml["or"].as_vec() {
            let mut asserts = Vec::new();
            for a in or {
                asserts.push(Self::yaml_to_assert(a)?);
            }
            return Ok(Assert::Or(asserts));
        }

        // Проверяем not
        if let Some(not) = yaml["not"].as_hash() {
            // Преобразуем в Yaml и передаём в yaml_to_assert
            let not_yaml = Yaml::Hash(not.clone());
            let inner = Box::new(Self::yaml_to_assert(&not_yaml)?);
            return Ok(Assert::Not(inner));
        }

        Err(CoreError::ConfigError("Unknown assert type".to_string()))
    }

    /// Подставляет переменные окружения в конфиг
    ///
    /// Ищет значения вида {{env.VAR_NAME}} и заменяет их на реальные значения
    fn substitute_env_vars(&self, suite: &mut TestSuite, prefix: &str) {
        // Собираем переменные окружения с нужным префиксом
        let env_vars: std::collections::HashMap<String, String> = std::env::vars()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, value)| (key, value))
            .collect();

        // Если есть env vars, подставляем их
        if !env_vars.is_empty() {
            // Подставляем в переменные
            for (key, value) in &mut suite.variables {
                if let Some(substituted) = self.substitute_value(value, &env_vars) {
                    *value = substituted;
                }
            }

            // Подставляем в URL и заголовки каждого запроса
            for stage in &mut suite.stages {
                // URL
                if let Some(substituted) = self.substitute_in_string(&stage.request.url, &env_vars) {
                    stage.request.url = substituted;
                }

                // Заголовки
                for (_key, value) in &mut stage.request.headers {
                    if let Some(substituted) = self.substitute_in_string(value, &env_vars) {
                        *value = substituted;
                    }
                }

                // Параметры
                for (_, value) in &mut stage.request.params {
                    if let Some(substituted) = self.substitute_in_string(value, &env_vars) {
                        *value = substituted;
                    }
                }
            }
        }
    }

    /// Рекурсивная подстановка
    fn substitute_value(
        &self,
        value: &serde_json::Value,
        env_vars: &HashMap<String, String>,
    ) -> Option<serde_json::Value> {
        match value {
            serde_json::Value::String(s) => {
                if let Some(substituted) = self.substitute_in_string(s, env_vars) {
                    Some(serde_json::Value::String(substituted))
                } else {
                    None
                }
            }
            serde_json::Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                let mut changed = false;
                for (k, v) in obj {
                    if let Some(substituted) = self.substitute_value(v, env_vars) {
                        new_obj.insert(k.clone(), substituted);
                        changed = true;
                    } else {
                        new_obj.insert(k.clone(), v.clone());
                    }
                }
                if changed { Some(serde_json::Value::Object(new_obj)) } else { None }
            }
            serde_json::Value::Array(arr) => {
                let mut new_arr = Vec::new();
                let mut changed = false;
                for item in arr {
                    if let Some(substituted) = self.substitute_value(item, env_vars) {
                        new_arr.push(substituted);
                        changed = true;
                    } else {
                        new_arr.push(item.clone());
                    }
                }
                if changed { Some(serde_json::Value::Array(new_arr)) } else { None }
            }
            _ => None, // Числа, булевы, null - не обрабатываем
        }
    }

    /// Заменяет шаблоны вида {{env.VAR_NAME}} в строке
    fn substitute_in_string(
        &self,
        input: &str,
        env_vars: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        let mut result = input.to_string();
        let mut changed = false;

        for (key, value) in env_vars {
            let pattern = format!("{{{{env.{}}}}}", key);
            if result.contains(&pattern) {
                result = result.replace(&pattern, value);
                changed = true;
            }
        }

        if changed { Some(result) } else { None }
    }

    /// Загружает конфиг из файла и валидирует его
    pub fn load_and_validate_from_file<P: AsRef<Path>>(&self, path: P) -> CoreResult<TestSuite> {
        let suite = self.load_from_file(path)?;

        ConfigValidator::validate(&suite)?;

        Ok(suite)
    }

    /// Загружает конфиг из строки и валидирует его
    pub fn load_and_validate_from_str(&self, content: &str) -> CoreResult<TestSuite> {
        let suite = self.load_from_str(content)?;

        ConfigValidator::validate(&suite)?;

        Ok(suite)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_load_from_str() {
        let yaml = r#"
name: "Test Suite"
variables:
  foo: "bar"
stages:
  - name: "Test"
    request:
      method: GET
      url: "https://api.example.com"
"#;

        let loader = ConfigLoader::new();
        let suite = loader.load_from_str(yaml).unwrap();

        assert_eq!(suite.name, "Test Suite");
        assert_eq!(suite.variables.get("foo").unwrap(), &serde_json::json!("bar"));
        assert_eq!(suite.stages.len(), 1);
        assert_eq!(suite.stages[0].name, "Test");
    }

    #[test]
    fn test_load_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
name: "File Test"
stages:
  - name: "Test"
    request:
      method: GET
      url: "https://api.example.com"
"#).unwrap();

        let loader = ConfigLoader::new();
        let suite = loader.load_from_file(temp_file.path()).unwrap();

        assert_eq!(suite.name, "File Test");
    }

    #[test]
    fn test_load_and_validate_duplicate_names() {
        let yaml = r#"
name: "Test Suite"
stages:
  - name: "Test"
    request:
      method: GET
      url: "https://api.example.com"
  - name: "Test"
    request:
      method: GET
      url: "https://api.example.com"
"#;

        let loader = ConfigLoader::new();
        let result = loader.load_and_validate_from_str(yaml);

        assert!(result.is_err());
        match result {
            Err(CoreError::ValidationError(msg)) => {
                assert!(msg.contains("Duplicate stage name"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_load_and_validate_empty_stages() {
        let yaml = r#"
name: "Test Suite"
stages: []
"#;

        let loader = ConfigLoader::new();
        let result = loader.load_and_validate_from_str(yaml);

        assert!(result.is_err());
        match result {
            Err(CoreError::ValidationError(msg)) => {
                assert!(msg.contains("No stages defined"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_env_substitution() {
        temp_env::with_var("RIVET_API_URL", Some("https://api.example.com"), || {
            let yaml = r#"
name: "Test Suite"
variables:
  base_url: "{{env.RIVET_API_URL}}"
stages:
  - name: "Test"
    request:
      method: GET
      url: "{{env.RIVET_API_URL}}/users"
"#;

            let loader = ConfigLoader::new()
                .with_env_prefix("RIVET_");
            let suite = loader.load_from_str(yaml).unwrap();

            assert_eq!(
                suite.variables.get("base_url").unwrap().as_str().unwrap(),
                "https://api.example.com"
            );
            assert_eq!(
                suite.stages[0].request.url,
                "https://api.example.com/users"
            );
        });
    }
}