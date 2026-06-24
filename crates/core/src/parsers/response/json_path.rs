//! Парсер JSONPath для извлечения данных из JSON
//!
//! Поддерживает упрощенный синтаксис:
//! - `user.name` - доступ к полю
//! - `user.posts[0]` - доступ по индексу
//! - `user.posts[*]` - все элементы массива
//! - `$` - корень JSON

use serde_json::Value;
use crate::error::{CoreError, CoreResult};

/// Парсер JSONPath
///
/// Извлекает значения из JSON по указанному пути
#[derive(Debug, Default)]
pub struct JsonPathParser;

impl JsonPathParser {
    /// Создает новый парсер
    pub fn new() -> Self {
        JsonPathParser
    }

    /// Извлекает первое значение по пути
    ///
    /// # Пример
    /// ```
    /// use serde_json::json;
    /// use rivet_core::parsers::response::JsonPathParser;
    ///
    /// let data = json!({
    ///     "user": {
    ///         "name": "John",
    ///         "age": 30
    ///     }
    /// });
    ///
    /// let result = JsonPathParser::extract_first(&data, "user.name").unwrap();
    /// assert_eq!(result, Some(&json!("John")));
    /// ```
    pub fn extract_first<'a>(json: &'a Value, path: &str) -> CoreResult<Option<&'a Value>> {
        let path = Self::normalize_path(path);

        let tokens = Self::tokenize(&path)?;

        let mut current = json;

        for token in tokens {
            match token {
                PathToken::Field(field) => {
                    if let Some(obj) = current.as_object() {
                        if let Some(value) = obj.get(&field) {
                            current = value;
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                }
                PathToken::Index(index) => {
                    if let Some(arr) = current.as_array() {
                        if let Some(value) = arr.get(index) {
                            current = value;
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                }
                PathToken::Wildcard => {
                    if let Some(arr) = current.as_array() {
                        if let Some(first) = arr.first() {
                            current = first;
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                }
                PathToken::Root => {}
            }
        }

        Ok(Some(current))
    }

    /// Извлекает все значения по пути
    ///
    /// # Пример
    /// ```
    /// use serde_json::json;
    /// use rivet_core::parsers::response::JsonPathParser;
    ///
    /// let data = json!({
    ///     "user": {
    ///         "posts": [
    ///             {"title": "First", "id": 1},
    ///             {"title": "Second", "id": 2}
    ///         ]
    ///     }
    /// });
    ///
    /// let results = JsonPathParser::extract_all(&data, "user.posts[*].id").unwrap();
    /// assert_eq!(results, vec![&json!(1), &json!(2)]);
    /// ```
    pub fn extract_all<'a>(json: &'a Value, path: &str) -> CoreResult<Vec<&'a Value>> {
        let path = Self::normalize_path(path);
        let tokens = Self::tokenize(&path)?;

        let mut results = vec![json];

        for token in tokens {
            let mut new_results = Vec::new();

            for current in results {
                match token {
                    PathToken::Field(ref field) => {  // ← Используем ref
                        if let Some(obj) = current.as_object() {
                            if let Some(value) = obj.get(field) {
                                new_results.push(value);
                            }
                        }
                    }
                    PathToken::Index(index) => {
                        if let Some(arr) = current.as_array() {
                            if let Some(value) = arr.get(index) {
                                new_results.push(value);
                            }
                        }
                    }
                    PathToken::Wildcard => {
                        if let Some(arr) = current.as_array() {
                            new_results.extend(arr.iter());
                        } else if let Some(obj) = current.as_object() {
                            new_results.extend(obj.values());
                        }
                    }
                    PathToken::Root => {
                        new_results.push(current);
                    }
                }
            }

            results = new_results;

            if results.is_empty() {
                return Ok(vec![]);
            }
        }

        Ok(results)
    }

    /// Нормализует путь в единый формат
    ///
    /// Преобразует упрощенный синтаксис в стандартный JSONPath
    ///
    /// # Пример
    /// ```
    /// use rivet_core::parsers::response::JsonPathParser;
    ///
    /// assert_eq!(
    ///     JsonPathParser::normalize_path("user.name"),
    ///     "$.user.name"
    /// );
    /// assert_eq!(
    ///     JsonPathParser::normalize_path("$.user.name"),
    ///     "$.user.name"
    /// );
    /// assert_eq!(
    ///     JsonPathParser::normalize_path("posts[0].title"),
    ///     "$.posts[0].title"
    /// );
    /// ```
    pub fn normalize_path(path: &str) -> String {
        let path = path.trim();

        // Если уже начинается с $ - возвращаем как есть
        if path.starts_with('$') {
            return path.to_string();
        }

        // Если начинается с точки - добавляем $
        if path.starts_with('.') {
            return format!("${}", path);
        }

        // Иначе добавляем $.
        format!("$.{}", path)
    }

    /// Разбивает путь на токены
    fn tokenize(path: &str) -> CoreResult<Vec<PathToken>> {
        let mut tokens = Vec::new();
        let mut chars = path.chars().peekable();

        // Пропускаем начальный $.
        if let Some('$') = chars.peek() {
            chars.next();
            if let Some('.') = chars.peek() {
                chars.next();
            }
        }

        while let Some(ch) = chars.peek() {
            match ch {
                '.' => {
                    chars.next();
                    // Собираем имя поля
                    let mut field = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '.' || c == '[' || c == '*' || c == ']' {
                            break;
                        }
                        field.push(c);
                        chars.next();
                    }
                    if !field.is_empty() {
                        tokens.push(PathToken::Field(field));
                    }
                }
                '[' => {
                    chars.next();
                    if let Some(&'*') = chars.peek() {
                        chars.next();
                        if let Some(&']') = chars.peek() {
                            chars.next();
                            tokens.push(PathToken::Wildcard);
                        }
                    } else {
                        let mut index_str = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == ']' {
                                break;
                            }
                            if !c.is_ascii_digit() {
                                return Err(CoreError::ParseError(
                                    format!("Expected digit, got '{}'", c)
                                ));
                            }
                            index_str.push(c);
                            chars.next();
                        }
                        if let Some(&']') = chars.peek() {
                            chars.next();
                        }
                        let index: usize = index_str.parse()
                            .map_err(|e| CoreError::ParseError(
                                format!("Invalid index '{}': {}", index_str, e)
                            ))?;
                        tokens.push(PathToken::Index(index));
                    }
                }
                '*' => {
                    chars.next();
                    tokens.push(PathToken::Wildcard);
                }
                ']' => {
                    return Err(CoreError::ParseError(
                        "Unexpected ']'".to_string()
                    ));
                }
                _ => {
                    // Если встретили букву без точки - это поле
                    let mut field = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '.' || c == '[' || c == ']' {
                            break;
                        }
                        field.push(c);
                        chars.next();
                    }
                    if !field.is_empty() {
                        tokens.push(PathToken::Field(field));
                    }
                }
            }
        }

        Ok(tokens)
    }

    /// Проверяет, существует ли путь в JSON
    pub fn path_exists(json: &Value, path: &str) -> CoreResult<bool> {
        Ok(Self::extract_first(json, path)?.is_some())
    }
}

/// Токен пути в JSONPath
#[derive(Debug, Clone, PartialEq)]
enum PathToken {
    /// Поле объекта (например, "name")
    Field(String),
    /// Индекс массива (например, 0)
    Index(usize),
    /// Все элементы (wildcard)
    Wildcard,
    /// Корень JSON
    #[allow(dead_code)]
    Root,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn get_test_data() -> Value {
        json!({
            "user": {
                "name": "John Doe",
                "age": 30,
                "email": "john@example.com",
                "address": {
                    "city": "New York",
                    "country": "USA"
                },
                "posts": [
                    {
                        "id": 1,
                        "title": "First Post",
                        "tags": ["rust", "programming"]
                    },
                    {
                        "id": 2,
                        "title": "Second Post",
                        "tags": ["python", "data"]
                    }
                ]
            },
            "status": "active"
        })
    }

    #[test]
    fn test_extract_first_simple() {
        let data = get_test_data();

        let result = JsonPathParser::extract_first(&data, "user.name").unwrap();
        assert_eq!(result, Some(&json!("John Doe")));

        let result = JsonPathParser::extract_first(&data, "user.age").unwrap();
        assert_eq!(result, Some(&json!(30)));

        let result = JsonPathParser::extract_first(&data, "status").unwrap();
        assert_eq!(result, Some(&json!("active")));
    }

    #[test]
    fn test_extract_first_nested() {
        let data = get_test_data();

        let result = JsonPathParser::extract_first(&data, "user.address.city").unwrap();
        assert_eq!(result, Some(&json!("New York")));

        let result = JsonPathParser::extract_first(&data, "user.posts[0].title").unwrap();
        assert_eq!(result, Some(&json!("First Post")));
    }

    #[test]
    fn test_extract_first_not_found() {
        let data = get_test_data();

        let result = JsonPathParser::extract_first(&data, "user.nonexistent").unwrap();
        assert_eq!(result, None);

        let result = JsonPathParser::extract_first(&data, "user.posts[99].title").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_all_wildcard() {
        let data = get_test_data();

        let results = JsonPathParser::extract_all(&data, "user.posts[*].title").unwrap();
        assert_eq!(results, vec![
            &json!("First Post"),
            &json!("Second Post")
        ]);

        let results = JsonPathParser::extract_all(&data, "user.posts[*].id").unwrap();
        assert_eq!(results, vec![
            &json!(1),
            &json!(2)
        ]);
    }

    #[test]
    fn test_extract_all_nested_wildcard() {
        let data = get_test_data();

        let results = JsonPathParser::extract_all(&data, "user.posts[*].tags[*]").unwrap();
        assert_eq!(results, vec![
            &json!("rust"),
            &json!("programming"),
            &json!("python"),
            &json!("data")
        ]);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            JsonPathParser::normalize_path("user.name"),
            "$.user.name"
        );
        assert_eq!(
            JsonPathParser::normalize_path("$.user.name"),
            "$.user.name"
        );
        assert_eq!(
            JsonPathParser::normalize_path("user.posts[0].title"),
            "$.user.posts[0].title"
        );
        assert_eq!(
            JsonPathParser::normalize_path("posts[*].id"),
            "$.posts[*].id"
        );
    }

    #[test]
    fn test_path_exists() {
        let data = get_test_data();

        assert!(JsonPathParser::path_exists(&data, "user.name").unwrap());
        assert!(JsonPathParser::path_exists(&data, "user.address.city").unwrap());
        assert!(!JsonPathParser::path_exists(&data, "user.nonexistent").unwrap());
        assert!(!JsonPathParser::path_exists(&data, "user.posts[99]").unwrap());
    }

    #[test]
    fn test_extract_first_with_root() {
        let data = get_test_data();

        let result = JsonPathParser::extract_first(&data, "$.user.name").unwrap();
        assert_eq!(result, Some(&json!("John Doe")));

        let result = JsonPathParser::extract_first(&data, "$.status").unwrap();
        assert_eq!(result, Some(&json!("active")));
    }

    #[test]
    fn test_extract_first_with_dot_prefix() {
        let data = get_test_data();

        let result = JsonPathParser::extract_first(&data, ".user.name").unwrap();
        assert_eq!(result, Some(&json!("John Doe")));
    }

    #[test]
    fn test_complex_path() {
        let data = json!({
            "store": {
                "books": [
                    {
                        "category": "fiction",
                        "title": "The Hobbit",
                        "price": 10
                    },
                    {
                        "category": "nonfiction",
                        "title": "The Rust Book",
                        "price": 30
                    }
                ]
            }
        });

        let result = JsonPathParser::extract_first(&data, "store.books[1].title").unwrap();
        assert_eq!(result, Some(&json!("The Rust Book")));

        let results = JsonPathParser::extract_all(&data, "store.books[*].price").unwrap();
        assert_eq!(results, vec![&json!(10), &json!(30)]);
    }
}