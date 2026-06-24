//! HTTP клиент для выполнения запросов
//!
//! Отвечает за:
//! - Выполнение HTTP запросов (client)
//! - Построение запросов из конфигурации (request_builder)
//! - Поддержка HTTP/2 и HTTP/3
//! - Retry механизмы
//! - Таймауты

mod client;
mod request_builder;

// Публичный экспорт
pub use client::{HttpClient, HttpRequest, HttpResponse};
pub use request_builder::RequestBuilder;