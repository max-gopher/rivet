//! Вывод в консоль с цветами и форматированием

use colored::*;
use std::path::Path;
use std::time::Duration;
use anyhow::Result;
use serde_json::json;

use rivet_core::TestSuite;
use rivet_core::context::TestContext;
use crate::output::formatters::format_duration;

/// Основной класс для вывода в консоль
pub struct ConsoleOutput {
    pub verbose: bool,
    pub no_color: bool,
    pub json: bool,
}

impl ConsoleOutput {
    /// Создает новый вывод
    pub fn new(verbose: bool, no_color: bool, json: bool) -> Self {
        if no_color {
            colored::control::set_override(false);
        }

        ConsoleOutput { verbose, no_color, json }
    }

    // ============ ЗАГОЛОВКИ ============

    /// Печатает заголовок
    pub fn print_header(&self, message: &str) {
        if self.json {
            return;
        }
        println!("\n{}", "═".repeat(60).bright_blue());
        println!("{} {}", "▶".bright_blue(), message.bright_white());
        println!("{}", "═".repeat(60).bright_blue());
    }

    /// Печатает информацию
    pub fn print_info(&self, message: &str) {
        if self.json {
            return;
        }
        println!("{}", message);
    }

    /// Печатает успешное сообщение
    pub fn print_success(&self, message: &str) {
        if self.json {
            return;
        }
        println!("{}", message.green());
    }

    /// Печатает предупреждение
    pub fn print_warning(&self, message: &str) {
        if self.json {
            return;
        }
        println!("{}", message.yellow());
    }

    /// Печатает ошибку
    pub fn print_error(&self, message: &str) {
        if self.json {
            return;
        }
        eprintln!("{}", message.red());
    }

    // ============ ВЫВОД РЕЗУЛЬТАТОВ ============

    /// Печатает заголовок тестового набора
    #[allow(dead_code)]
    pub fn print_suite_header(&self, suite: &TestSuite) {
        if self.json {
            return;
        }

        println!("\n{}", "════════════════════════════════════════════════════════════════════".bright_blue());
        println!("{} {}", "🚀".bright_green(), suite.name.bright_white());
        if let Some(version) = &suite.version {
            println!("  {} {}", "Version:".dimmed(), version);
        }
        if let Some(desc) = &suite.description {
            println!("  {} {}", "Description:".dimmed(), desc);
        }
        println!("{}", "════════════════════════════════════════════════════════════════════".bright_blue());
    }

    /// Печатает этап теста
    #[allow(dead_code)]
    pub fn print_stage(&self, name: &str, is_skipped: bool) {
        if self.json {
            return;
        }

        if is_skipped {
            println!("  {} {} {}", "⏭️".yellow(), name.bright_white(), "(skipped)".dimmed());
        } else {
            print!("  {} {} ... ", "▶️".bright_yellow(), name.bright_white());
        }
    }

    /// Печатает результат этапа
    #[allow(dead_code)]
    pub fn print_stage_result(&self, _name: &str, passed: bool, duration: Duration, error: Option<&str>) {
        if self.json {
            return;
        }

        if passed {
            println!("{}", "✅".bright_green());
            if self.verbose {
                println!("     {}", format_duration(duration).dimmed());
            }
        } else {
            println!("{}", "❌".red());
            if let Some(err) = error {
                println!("     {}", err.red());
            }
            if self.verbose && !error.is_some() {
                println!("     {}", format_duration(duration).dimmed());
            }
        }
    }

    /// Печатает итоговую сводку
    #[allow(dead_code)]
    pub fn print_summary(&self, context: &TestContext, all_passed: bool, duration: Duration) {
        if self.json {
            return;
        }

        let total = context.stage_results.len();
        let passed = context.passed_count();
        let failed = context.failed_count();

        println!("\n{}", "════════════════════════════════════════════════════════════════════".bright_blue());
        println!("{}", "📊 Results:".bright_cyan());
        println!("  {} {}", "Total:".dimmed(), total);
        println!("  {} {}", "✅ Passed:".green(), passed);
        println!("  {} {}", "❌ Failed:".red(), failed);
        println!("  {} {}", "⏱️ Duration:".dimmed(), format_duration(duration));

        if all_passed {
            println!("\n{}", "🎉 All tests passed!".bright_green());
        } else {
            println!("\n{}", "❌ Some tests failed!".red());

            // Показываем проваленные этапы
            println!("\n{}", "📋 Failed stages:".bold());
            for (name, result) in &context.stage_results {
                if !result.passed {
                    println!("  ❌ {}", name.red());
                    if let Some(error) = &result.error {
                        println!("     {}", error.dimmed());
                    }
                }
            }
        }
        println!("{}", "════════════════════════════════════════════════════════════════════".bright_blue());
    }

    // ============ JSON ВЫВОД ============

    /// Выводит результат в JSON формате
    #[allow(dead_code)]
    pub fn print_json_result(&self, context: &TestContext, all_passed: bool, duration: Duration) {
        if !self.json {
            return;
        }

        let total = context.stage_results.len();
        let passed = context.passed_count();
        let failed = context.failed_count();

        let mut stages = Vec::new();
        for (name, result) in &context.stage_results {
            stages.push(json!({
                "name": name,
                "passed": result.passed,
                "duration_ms": result.duration.as_millis(),
                "status": result.status,
                "error": result.error,
                "extracted": result.extracted,
            }));
        }

        let output = json!({
            "success": all_passed,
            "summary": {
                "total": total,
                "passed": passed,
                "failed": failed,
                "duration_ms": duration.as_millis(),
            },
            "stages": stages,
        });

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }

    // ============ ОТЧЕТЫ ============

    /// Сохраняет отчет в файл
    pub fn save_report(
        &self,
        suite: &TestSuite,
        all_passed: bool,
        duration: Duration,
        path: &Path,
    ) -> Result<()> {
        let report = json!({
            "suite": {
                "name": suite.name,
                "version": suite.version,
                "description": suite.description,
            },
            "success": all_passed,
            "duration_ms": duration.as_millis(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let content = serde_json::to_string_pretty(&report)?;
        std::fs::write(path, content)?;

        Ok(())
    }
}