//! Модуль для работы с шаблонами конфигов

mod models;
mod manager;
mod repository;

pub use models::*;
pub use manager::TemplateManager;
pub use manager::get_user_template_dir;
pub use repository::TemplateRepository;