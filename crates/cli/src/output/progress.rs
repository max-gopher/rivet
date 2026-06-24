//! Прогресс-бары и спиннеры

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Спиннер для индикации выполнения
pub struct ProgressSpinner {
    pb: ProgressBar,
}

impl ProgressSpinner {
    /// Создает новый спиннер
    pub fn new(message: &str) -> Self {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));

        ProgressSpinner { pb }
    }

    /// Обновляет сообщение спиннера
    pub fn set_message(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }

    /// Завершает спиннер с успехом
    pub fn finish(&self) {
        self.pb.finish_with_message("✅ Done".to_string());
    }

    /// Завершает спиннер с ошибкой
    pub fn finish_with_error(&self, message: &str) {
        self.pb.finish_with_message(format!("❌ {}", message));
    }
}

/// Прогресс-бар для отображения прогресса
pub struct ProgressBarWrapper {
    pb: ProgressBar,
}

impl ProgressBarWrapper {
    /// Создает новый прогресс-бар
    pub fn new(total: u64, message: &str) -> Self {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}"
            )
                .unwrap()
                .progress_chars("#>-")
        );
        pb.set_message(message.to_string());

        ProgressBarWrapper { pb }
    }

    /// Увеличивает прогресс на 1
    pub fn inc(&self) {
        self.pb.inc(1);
    }

    /// Устанавливает сообщение
    pub fn set_message(&self, message: &str) {
        self.pb.set_message(message.to_string());
    }

    /// Завершает прогресс-бар
    pub fn finish(&self) {
        self.pb.finish_with_message("✅ Complete".to_string());
    }
}