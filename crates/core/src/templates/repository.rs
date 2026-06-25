//! Работа с удаленными репозиториями шаблонов

use std::path::{PathBuf};
use std::fs;
use reqwest;

use crate::error::{CoreError, CoreResult};
use crate::templates::models::{TemplateInfo, TemplateSource};

/// Менеджер репозиториев шаблонов
#[derive(Debug, Clone)]
pub struct TemplateRepository {
    /// URL репозитория
    pub url: String,

    /// Локальный кэш
    pub cache_dir: PathBuf,
}

impl TemplateRepository {
    /// Создает новый репозиторий
    pub fn new(url: &str) -> CoreResult<Self> {
        let cache_dir = get_cache_dir()?.join("repositories").join(url.replace('/', "_"));

        Ok(TemplateRepository {
            url: url.to_string(),
            cache_dir,
        })
    }

    /// Скачивает шаблон из репозитория
    pub async fn fetch_template(&self, template_name: &str) -> CoreResult<String> {
        if self.cache_dir.exists() {
            let cached_path = self.cache_dir.join(format!("{}.yaml", template_name));
            if cached_path.exists() {
                return fs::read_to_string(cached_path)
                    .map_err(|e| CoreError::IoError(
                        format!("Failed to read cached template '{}': {}", template_name, e)
                    ));
            }
        }

        let url = format!("{}/{}.yaml", self.url, template_name);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CoreError::ConfigError(
                format!("Template '{}' not found in repository: {}", template_name, self.url)
            ));
        }

        let content = response.text()
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?;

        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)
                .map_err(|e| CoreError::IoError(
                    format!("Failed to create cache directory: {}", e)
                ))?;
        }

        fs::write(self.cache_dir.join(format!("{}.yaml", template_name)), &content)
            .map_err(|e| CoreError::IoError(
                format!("Failed to cache template '{}': {}", template_name, e)
            ))?;

        Ok(content)
    }

    /// Получает список доступных шаблонов в репозитории
    pub async fn list_templates(&self) -> CoreResult<Vec<TemplateInfo>> {
        let url = format!("{}/index.json", self.url);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let response_text = response.text()
            .await
            .map_err(|e| CoreError::HttpError(e.to_string()))?;

        let index: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(CoreError::JsonError)?;

        let mut templates = Vec::new();

        if let Some(templates_array) = index.as_array() {
            for item in templates_array {
                let name = item["name"].as_str().unwrap_or("unknown").to_string();
                let display_name = item["display_name"].as_str().unwrap_or(&name).to_string();
                let description = item["description"].as_str().unwrap_or("").to_string();

                templates.push(TemplateInfo {
                    name,
                    display_name,
                    description,
                    category: crate::templates::models::TemplateCategory::Custom,
                    source: TemplateSource::Repository,
                    path: PathBuf::from(&self.url),
                    version: item["version"].as_str().unwrap_or("1.0.0").to_string(),
                });
            }
        }

        Ok(templates)
    }
}

/// Получает директорию для кэша
fn get_cache_dir() -> CoreResult<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(cache_dir) = std::env::var("XDG_CACHE_HOME") {
            return Ok(PathBuf::from(cache_dir).join("rivet"));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Ok(PathBuf::from(home).join(".cache").join("rivet"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Ok(PathBuf::from(home).join("Library").join("Caches").join("rivet"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
            return Ok(PathBuf::from(app_data).join("rivet").join("cache"));
        }
    }

    Ok(PathBuf::from(".rivet").join("cache"))
}