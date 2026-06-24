//! Rivet Core - библиотека для тестирования REST API
//!
//! Это ядро проекта, которое содержит:
//! - Парсеры YAML конфигураций
//! - Парсеры JSON ответов
//! - HTTP клиент
//! - Шаблонизатор
//! - Движок выполнения тестов

// Запрещаем использование unsafe кода (для безопасности)
#![deny(unsafe_code)]

// Объявляем модули
pub mod parsers;
pub mod http;
pub mod template;
pub mod templates;
pub mod engine;
pub mod context;
pub mod error;

// Re-export основных типов для удобства использования
// Теперь можно писать: use rivet_core::TestEngine;
// Вместо: use rivet_core::engine::TestEngine;
pub use engine::TestEngine;
pub use context::TestContext;
pub use error::CoreError;
pub use parsers::config::TestSuite;