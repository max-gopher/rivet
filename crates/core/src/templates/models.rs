//! Модели для работы с шаблонами

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Информация о шаблоне
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Имя шаблона (уникальный идентификатор)
    pub name: String,

    /// Отображаемое имя
    pub display_name: String,

    /// Описание шаблона
    pub description: String,

    /// Категория шаблона
    pub category: TemplateCategory,

    /// Источник шаблона (встроенный/пользовательский)
    pub source: TemplateSource,

    /// Путь к файлу шаблона
    pub path: PathBuf,

    /// Версия шаблона
    pub version: String,
}

/// Категория шаблона
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemplateCategory {
    /// Базовый шаблон
    Basic,
    /// Авторизация
    Auth,
    /// CRUD операции
    Crud,
    /// Полноценный пример
    Full,
    /// Минимальный пример
    Simple,
    /// OAuth2 поток
    OAuth,
    /// GraphQL запросы
    GraphQL,
    /// Загрузка файлов
    FileUpload,
    /// Пользовательский шаблон
    Custom,
}

/// Источник шаблона
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemplateSource {
    /// Встроенный в программу
    Builtin,
    /// Пользовательский
    User,
    /// Из репозитория
    Repository,
}

/// Результат генерации шаблона
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateGenerationResult {
    /// Содержимое шаблона
    pub content: String,

    /// Путь, куда сохранен (если сохранен)
    pub saved_path: Option<PathBuf>,

    /// Использованные переменные
    pub variables: std::collections::HashMap<String, String>,
}