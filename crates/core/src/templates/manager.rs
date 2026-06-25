//! Управление шаблонами

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use crate::error::{CoreError, CoreResult};
use crate::templates::models::{TemplateInfo, TemplateCategory, TemplateSource, TemplateGenerationResult};

#[derive(Debug, Clone)]
pub struct TemplateManager {
    template_dirs: Vec<PathBuf>,
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut manager = TemplateManager {
            template_dirs: Vec::new(),
        };
        manager.add_builtin_dir();
        manager.add_user_dir();
        manager
    }

    fn add_builtin_dir(&mut self) {
        // Встроенные шаблоны загружаются через include_str!
    }

    fn add_user_dir(&mut self) {
        if let Some(user_dir) = get_user_template_dir() && user_dir.exists() {
            self.template_dirs.push(user_dir);
        }
    }

    pub fn add_template_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.template_dirs.push(dir.as_ref().to_path_buf());
    }

    pub fn list_templates(&self) -> CoreResult<Vec<TemplateInfo>> {
        let mut templates = Vec::new();
        templates.extend(self.load_builtin_templates()?);
        templates.extend(self.load_user_templates()?);
        Ok(templates)
    }

    fn load_builtin_templates(&self) -> CoreResult<Vec<TemplateInfo>> {
        let mut templates = Vec::new();

        let builtin_templates = [
            ("default", "Default Template", "Basic template with common features", TemplateCategory::Basic),
            ("auth", "Auth Template", "Template with JWT authentication", TemplateCategory::Auth),
            ("crud", "CRUD Template", "Complete CRUD operations", TemplateCategory::Crud),
            ("simple", "Simple Template", "Minimal example with one endpoint", TemplateCategory::Simple),
        ];

        for (name, display, desc, category) in builtin_templates {
            // Проверяем, что шаблон существует
            let _content = match name {
                "default" => include_str!("../templates/builtin/default.yaml"),
                "auth" => include_str!("../templates/builtin/auth.yaml"),
                "crud" => include_str!("../templates/builtin/crud.yaml"),
                "simple" => include_str!("../templates/builtin/simple.yaml"),
                _ => continue,
            };

            templates.push(TemplateInfo {
                name: name.to_string(),
                display_name: display.to_string(),
                description: desc.to_string(),
                category,
                source: TemplateSource::Builtin,
                path: PathBuf::from(format!("builtin://{}", name)),
                version: "1.0.0".to_string(),
            });
        }

        Ok(templates)
    }

    fn load_user_templates(&self) -> CoreResult<Vec<TemplateInfo>> {
        let mut templates = Vec::new();

        for dir in &self.template_dirs {
            if !dir.exists() {
                continue;
            }

            let entries = fs::read_dir(dir)
                .map_err(|e| CoreError::IoError(e.to_string()))?;

            for entry in entries {
                let entry = entry.map_err(|e| CoreError::IoError(
                    format!("Failed to read directory entry: {}", e)
                ))?;
                let path = entry.path();

                if path.extension().and_then(|e| e.to_str()) == Some("yaml") && let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    let info = self.parse_template_info(name, &path)?;
                    templates.push(info);
                }
            }
        }

        Ok(templates)
    }

    fn parse_template_info(&self, name: &str, path: &Path) -> CoreResult<TemplateInfo> {
        Ok(TemplateInfo {
            name: name.to_string(),
            display_name: name.to_string(),
            description: format!("Custom template: {}", name),
            category: TemplateCategory::Custom,
            source: TemplateSource::User,
            path: path.to_path_buf(),
            version: "1.0.0".to_string(),
        })
    }

    pub fn generate_template(
        &self,
        template_name: &str,
        output_path: Option<&Path>,
        variables: &HashMap<String, String>,
    ) -> CoreResult<TemplateGenerationResult> {
        let template = self.find_template(template_name)?;

        let content = match template.source {
            TemplateSource::Builtin => {
                self.get_builtin_template(template_name)?
            }
            TemplateSource::User | TemplateSource::Repository => {
                fs::read_to_string(&template.path)
                    .map_err(|e| CoreError::IoError(
                        format!("Failed to read template '{}': {}", template_name, e)
                    ))?
            }
        };

        let content = self.substitute_variables(&content, variables);

        let saved_path = if let Some(path) = output_path {
            fs::write(path, &content)
                .map_err(|e| CoreError::IoError(
                    format!("Failed to save template to '{}': {}", path.display(), e)
                ))?;
            Some(path.to_path_buf())
        } else {
            None
        };

        Ok(TemplateGenerationResult {
            content,
            saved_path,
            variables: variables.clone(),
        })
    }

    fn find_template(&self, name: &str) -> CoreResult<TemplateInfo> {
        let templates = self.list_templates()?;
        templates.into_iter()
            .find(|t| t.name == name)
            .ok_or_else(|| CoreError::ConfigError(
                format!("Template '{}' not found", name)
            ))
    }

    fn get_builtin_template(&self, name: &str) -> CoreResult<String> {
        let content = match name {
            "default" => include_str!("../templates/builtin/default.yaml"),
            "auth" => include_str!("../templates/builtin/auth.yaml"),
            "crud" => include_str!("../templates/builtin/crud.yaml"),
            "simple" => include_str!("../templates/builtin/simple.yaml"),
            _ => return Err(CoreError::ConfigError(
                format!("Builtin template '{}' not found", name)
            )),
        };
        Ok(content.to_string())
    }

    fn substitute_variables(&self, content: &str, variables: &HashMap<String, String>) -> String {
        let mut result = content.to_string();
        for (key, value) in variables {
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }
        result
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_user_template_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("RIVET_TEMPLATE_DIR") {
        return Some(PathBuf::from(dir));
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(data_dir) = std::env::var("XDG_DATA_HOME") {
            return Some(PathBuf::from(data_dir).join("rivet").join("templates"));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join(".local").join("share").join("rivet").join("templates"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join("Library").join("Application Support").join("rivet").join("templates"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return Some(PathBuf::from(app_data).join("rivet").join("templates"));
        }
    }

    None
}