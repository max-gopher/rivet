//! Шаблонизатор для подстановки переменных
//!
//! Поддерживает синтаксис {{variable_name}} для подстановки значений

mod engine;

pub use engine::TemplateEngine;