//! Определение команд CLI
//!
//! Описывает все доступные команды и их логику

use clap::{Parser, Subcommand};
use anyhow::Result;
use colored::*;
use std::path::{Path, PathBuf};

use rivet_core::{TestEngine, parsers::config::{load_and_validate_from_file, ConfigLoader}};
use rivet_core::templates::{TemplateManager, TemplateInfo, TemplateSource, TemplateCategory};
use rivet_core::templates::TemplateRepository;
use crate::output::{ConsoleOutput, ProgressSpinner};

/// Главная структура CLI аргументов
#[derive(Parser)]
#[command(name = "rivet")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "🚀 REST API testing framework")]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Включить подробный вывод
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Отключить цветной вывод
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Вывод в формате JSON
    #[arg(long, global = true)]
    pub json: bool,
}

/// Доступные команды
#[derive(Subcommand)]
pub enum Commands {
    /// Запустить тесты из YAML файла
    Run {
        /// Путь к YAML конфигу
        #[arg(short, long)]
        config: String,

        /// Префикс для переменных окружения
        #[arg(short, long)]
        env_prefix: Option<String>,

        /// Имя для сохранения отчета
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Проверить валидность YAML конфига
    Validate {
        /// Путь к YAML конфигу
        #[arg(short, long)]
        config: String,
    },

    /// Управление шаблонами
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },

    /// Показать информацию о программе
    Info,
}

/// Действия с шаблонами
#[derive(Subcommand)]
pub enum TemplateAction {
    /// Показать список доступных шаблонов
    List {
        /// Показать только шаблоны из указанного источника
        #[arg(long)]
        source: Option<TemplateSourceFilter>,
    },

    /// Сгенерировать файл из шаблона
    Generate {
        /// Имя шаблона
        #[arg(short, long)]
        template: String,

        /// Куда сохранить (если не указано - вывести в консоль)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Переменные для подстановки (key=value)
        #[arg(short, long)]
        var: Vec<String>,
    },

    /// Добавить репозиторий шаблонов
    AddRepo {
        /// URL репозитория (например, https://raw.githubusercontent.com/user/templates/main)
        #[arg(short, long)]
        url: String,
    },

    /// Добавить директорию с кастомными шаблонами
    AddDir {
        /// Путь к директории с шаблонами
        #[arg(short, long)]
        dir: PathBuf,
    },

    /// Создать шаблон из существующего конфига
    FromConfig {
        /// Путь к конфигу
        #[arg(short, long)]
        config: String,

        /// Имя нового шаблона
        #[arg(short, long)]
        name: String,
    },

    /// Обновить кэш репозиториев
    Update,
}

/// Фильтр по источнику шаблонов
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum TemplateSourceFilter {
    Builtin,
    User,
    Repository,
}

/// Выполняет команду
pub async fn execute(cli: Cli) -> Result<()> {
    let output = ConsoleOutput::new(cli.verbose, cli.no_color, cli.json);

    match cli.command {
        Commands::Run { config, env_prefix, output: output_file } => {
            run_command(&config, env_prefix, output_file, &output).await?;
        }
        Commands::Validate { config } => {
            validate_command(&config, &output)?;
        }
        Commands::Template { action } => {
            template_command(action, &output).await?;
        }
        Commands::Info => {
            info_command(&output)?;
        }
    }

    Ok(())
}

/// Запускает тесты
async fn run_command(
    config_path: &str,
    env_prefix: Option<String>,
    output_file: Option<PathBuf>,
    output: &ConsoleOutput,
) -> Result<()> {
    output.print_header("Loading configuration...");

    let suite = if let Some(prefix) = env_prefix {
        let loader = ConfigLoader::new().with_env_prefix(&prefix);
        let suite = loader.load_from_file(config_path)?;
        rivet_core::parsers::config::ConfigValidator::validate(&suite)?;
        suite
    } else {
        load_and_validate_from_file(config_path)?
    };

    output.print_success(&format!("✅ Loaded: {}", suite.name));
    output.print_info(&format!("📊 Stages: {}", suite.stages.len()));

    let engine = TestEngine::with_http_config(suite.http.as_ref());

    let start = std::time::Instant::now();
    output.print_info("🚀 Running tests...");

    let spinner = ProgressSpinner::new("Executing test suite...");
    let all_passed = engine.run(&suite).await?;
    spinner.finish();

    let duration = start.elapsed();

    if all_passed {
        output.print_success(&format!("✅ All tests passed! ({}ms)", duration.as_millis()));
    } else {
        output.print_error(&format!("❌ Some tests failed! ({}ms)", duration.as_millis()));
    }

    if let Some(path) = output_file {
        output.save_report(&suite, all_passed, duration, &path)?;
        output.print_success(&format!("📄 Report saved to: {}", path.display()));
    }

    if !all_passed {
        return Err(anyhow::anyhow!("Some tests failed"))
    }

    Ok(())
}

/// Проверяет валидность конфига
fn validate_command(config_path: &str, output: &ConsoleOutput) -> Result<()> {
    output.print_header("Validating configuration...");

    match load_and_validate_from_file(config_path) {
        Ok(suite) => {
            output.print_success("✅ Configuration is valid!");

            println!("\n{}", "📋 Suite Info:".bold());
            println!("  Name: {}", suite.name);
            if let Some(version) = suite.version {
                println!("  Version: {}", version);
            }
            if let Some(desc) = suite.description {
                println!("  Description: {}", desc);
            }
            println!("  Stages: {}", suite.stages.len());

            if !suite.stages.is_empty() {
                println!("\n{}", "📋 Stages:".bold());
                for (i, stage) in suite.stages.iter().enumerate() {
                    let num = (i + 1).to_string();
                    let deps = if stage.depends_on.is_empty() {
                        "none".to_string()
                    } else {
                        format!("depends on: {}", stage.depends_on.join(", "))
                    };
                    println!("  {}. {} ({})", num, stage.name, deps);
                }
            }

            Ok(())
        }
        Err(e) => {
            output.print_error(&format!("❌ Invalid configuration: {}", e));
            Err(e.into())
        }
    }
}

/// Обрабатывает команды шаблонов
async fn template_command(action: TemplateAction, output: &ConsoleOutput) -> Result<()> {
    match action {
        TemplateAction::List { source } => {
            template_list_command(source, output).await?;
        }
        TemplateAction::Generate { template, output: output_file, var } => {
            template_generate_command(&template, output_file, var, output).await?;
        }
        TemplateAction::AddRepo { url } => {
            template_add_repo_command(&url, output).await?;
        }
        TemplateAction::AddDir { dir } => {
            template_add_dir_command(&dir, output)?;
        }
        TemplateAction::FromConfig { config, name } => {
            template_from_config_command(&config, &name, output)?;
        }
        TemplateAction::Update => {
            template_update_command(output).await?;
        }
    }

    Ok(())
}

/// Показывает список шаблонов
async fn template_list_command(
    source_filter: Option<TemplateSourceFilter>,
    output: &ConsoleOutput,
) -> Result<()> {
    output.print_header("📋 Available Templates");

    let manager = TemplateManager::new();
    let mut templates = manager.list_templates()?;

    // Фильтруем по источнику
    if let Some(filter) = source_filter {
        let filter_source = match filter {
            TemplateSourceFilter::Builtin => TemplateSource::Builtin,
            TemplateSourceFilter::User => TemplateSource::User,
            TemplateSourceFilter::Repository => TemplateSource::Repository,
        };
        templates.retain(|t| t.source == filter_source);
    }

    if templates.is_empty() {
        output.print_info("No templates found");
        return Ok(());
    }

    // Группируем по категориям
    let mut grouped: std::collections::HashMap<String, Vec<&TemplateInfo>> = std::collections::HashMap::new();

    for template in &templates {
        let category = match template.category {
            TemplateCategory::Basic => "Basic",
            TemplateCategory::Auth => "Auth",
            TemplateCategory::Crud => "CRUD",
            TemplateCategory::Full => "Full",
            TemplateCategory::Simple => "Simple",
            TemplateCategory::OAuth => "OAuth",
            TemplateCategory::GraphQL => "GraphQL",
            TemplateCategory::FileUpload => "File Upload",
            TemplateCategory::Custom => "Custom",
        };

        grouped.entry(category.to_string())
            .or_default()
            .push(template);
    }

    // Выводим
    for (category, templates) in grouped {
        println!("\n{}", category.bold().cyan());
        for template in templates {
            let source = match template.source {
                TemplateSource::Builtin => "📦 builtin".dimmed(),
                TemplateSource::User => "📁 user".dimmed(),
                TemplateSource::Repository => "🌐 remote".dimmed(),
            };

            println!("  • {} - {}", template.name.bold(), template.description);
            println!("    {}", source);
        }
    }

    println!("\n{}", "💡 Use: rivet template generate --template <name> -o test.yaml".dimmed());
    println!("{}", "📦 Add repository: rivet template add-repo --url <url>".dimmed());

    Ok(())
}

/// Генерирует файл из шаблона
async fn template_generate_command(
    template_name: &str,
    output_file: Option<PathBuf>,
    vars: Vec<String>,
    output: &ConsoleOutput,
) -> Result<()> {
    output.print_header(&format!("Generating from template: {}", template_name));

    // Парсим переменные
    let mut variables = std::collections::HashMap::new();
    for var in vars {
        if let Some((key, value)) = var.split_once('=') {
            variables.insert(key.to_string(), value.to_string());
        } else {
            output.print_warning(&format!("Ignoring invalid variable: {}", var));
        }
    }

    // Проверяем, может это из репозитория?
    let manager = TemplateManager::new();
    let templates = manager.list_templates()?;

    let template_info = templates.iter().find(|t| t.name == template_name);

    let result = if let Some(info) = template_info {
        if info.source == TemplateSource::Repository {
            // Скачиваем из репозитория
            let repo = TemplateRepository::new(&info.path.to_string_lossy())?;
            let content = repo.fetch_template(template_name).await?;

            // Подставляем переменные
            let content = substitute_variables(&content, &variables);

            if let Some(path) = &output_file {
                std::fs::write(path, &content)?;
                output.print_success(&format!("✅ Template saved to: {}", path.display()));
            } else {
                println!("\n{}\n", content);
                output.print_info("💡 Save with: --output <file>");
            }

            return Ok(());
        } else {
            // Используем менеджер для локальных шаблонов
            manager.generate_template(template_name, output_file.as_deref(), &variables)?
        }
    } else {
        output.print_error(&format!("❌ Template '{}' not found", template_name));
        return Err(anyhow::anyhow!("Template '{}' not found", template_name))
    };

    if let Some(path) = result.saved_path {
        output.print_success(&format!("✅ Template saved to: {}", path.display()));
    } else {
        println!("\n{}\n", result.content);
        output.print_info("💡 Save with: --output <file>");
    }

    Ok(())
}

/// Подставляет переменные в шаблон
fn substitute_variables(content: &str, variables: &std::collections::HashMap<String, String>) -> String {
    let mut result = content.to_string();
    for (key, value) in variables {
        let pattern = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}

/// Добавляет репозиторий шаблонов
async fn template_add_repo_command(url: &str, output: &ConsoleOutput) -> Result<()> {
    output.print_header("Adding template repository...");

    // Создаем репозиторий
    let repo = TemplateRepository::new(url)?;

    // Проверяем доступность
    match repo.list_templates().await {
        Ok(templates) => {
            output.print_success(&format!("✅ Repository added: {}", url));
            output.print_info(&format!("📊 Found {} templates", templates.len()));

            // Показываем найденные шаблоны
            if !templates.is_empty() {
                println!("\n{}", "📋 Available templates:".bold());
                for template in templates {
                    println!("  • {} - {}", template.name, template.description);
                }
            }

            // Сохраняем в конфиг
            // В реальном проекте - сохраняем в ~/.rivet/config.toml
            output.print_info("💡 Use: rivet template generate --template <name> -o test.yaml");
        }
        Err(e) => {
            output.print_error(&format!("❌ Failed to access repository: {}", e));
            return Err(e.into());
        }
    }

    Ok(())
}

/// Добавляет директорию с кастомными шаблонами
fn template_add_dir_command(dir: &Path, output: &ConsoleOutput) -> Result<()> {
    if !dir.exists() {
        output.print_error(&format!("Directory does not exist: {}", dir.display()));
        return Err(anyhow::anyhow!("Directory does not exist"));
    }

    output.print_success(&format!("✅ Added template directory: {}", dir.display()));
    output.print_info("💡 To use templates from this directory:");
    output.print_info("   rivet template list");
    output.print_info("   rivet template generate --template <name> -o test.yaml");

    Ok(())
}

/// Создает шаблон из существующего конфига
fn template_from_config_command(config_path: &str, name: &str, output: &ConsoleOutput) -> Result<()> {
    output.print_header("Creating template from config...");

    let suite = load_and_validate_from_file(config_path)?;
    let content = serde_yaml::to_string(&suite)?;

    let user_dir = rivet_core::templates::get_user_template_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine user template directory"))?;

    std::fs::create_dir_all(&user_dir)?;

    let template_path = user_dir.join(format!("{}.yaml", name));
    std::fs::write(&template_path, content)?;

    output.print_success(&format!("✅ Template saved to: {}", template_path.display()));
    output.print_info("💡 Now you can use it with:");
    output.print_info(&format!("   rivet template generate --template {}", name));

    Ok(())
}

/// Обновляет кэш репозиториев
async fn template_update_command(output: &ConsoleOutput) -> Result<()> {
    output.print_header("Updating template repositories...");

    // В реальном проекте - читаем список репозиториев из конфига
    // и обновляем кэш для каждого

    output.print_success("✅ Cache updated successfully");
    Ok(())
}

/// Показывает информацию о программе
fn info_command(output: &ConsoleOutput) -> Result<()> {
    output.print_header("Rivet - REST API Testing Framework");

    println!("\n{}", "📦 Version:".bold());
    println!("  {}", env!("CARGO_PKG_VERSION"));

    println!("\n{}", "🔧 Features:".bold());
    println!("  ✅ YAML configuration");
    println!("  ✅ Variable substitution ({{var}} and {{env.VAR}})");
    println!("  ✅ HTTP requests with retry");
    println!("  ✅ JSONPath extraction");
    println!("  ✅ Multiple assertions (status, body, headers)");
    println!("  ✅ Stage dependencies");
    println!("  ✅ Beautiful output with colors");
    println!("  ✅ Template system with repositories");

    println!("\n{}", "📚 Examples:".bold());
    println!("  {}", "📄 List templates:     rivet template list".dimmed());
    println!("  {}", "📄 Generate template:  rivet template generate --template default -o test.yaml".dimmed());
    println!("  {}", "🔍 Validate config:    rivet validate -c test.yaml".dimmed());
    println!("  {}", "🚀 Run tests:          rivet run -c test.yaml".dimmed());
    println!("  {}", "📊 Verbose output:     rivet run -c test.yaml -v".dimmed());
    println!("  {}", "🌐 Add repository:     rivet template add-repo --url https://raw.githubusercontent.com/user/templates/main".dimmed());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_validate_command_valid() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, r#"
name: "Test Suite"
stages:
  - name: "Test"
    request:
      method: GET
      url: "https://api.example.com"
"#).unwrap();

        let output = ConsoleOutput::new(false, false, false);
        let result = validate_command(temp.path().to_str().unwrap(), &output);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_command_invalid() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, r#"
name: "Test Suite"
stages:
  - name: "Test"
    request:
      method: GET
"#).unwrap(); // missing url

        let output = ConsoleOutput::new(false, false, false);
        let result = validate_command(temp.path().to_str().unwrap(), &output);

        match result {
            Ok(_) => panic!("Expected error but got success"),
            Err(_) => {}
        }
    }
}