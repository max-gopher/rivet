//! Модуль вывода в консоль
//!
//! Отвечает за:
//! - Цветной вывод
//! - Прогресс-бары и спиннеры
//! - Форматирование результатов
//! - Сохранение отчетов

mod console;
mod progress;
mod formatters;

pub use console::ConsoleOutput;
pub use progress::ProgressSpinner;