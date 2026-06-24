//! Форматирование данных для вывода

use std::time::Duration;
use colored::*;

/// Форматирует длительность в человекочитаемый вид
pub fn format_duration(duration: Duration) -> String {
    let ms = duration.as_millis();

    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        let secs = ms as f64 / 1000.0;
        format!("{:.1}s", secs)
    } else {
        let mins = ms as f64 / 60000.0;
        format!("{:.1}m", mins)
    }
}

/// Форматирует статус HTTP с цветом
#[allow(dead_code)]
pub fn format_status(status: u16) -> String {
    let colored = match status {
        200..=299 => status.to_string().green(),
        300..=399 => status.to_string().yellow(),
        400..=499 => status.to_string().red(),
        500..=599 => status.to_string().purple(),
        _ => status.to_string().white(),
    };
    colored.to_string()
}

/// Форматирует размер в человекочитаемый вид
#[allow(dead_code)]
pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.1}GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

/// Форматирует JSON значение для вывода
#[allow(dead_code)]
pub fn format_json_value(value: &serde_json::Value, max_length: usize) -> String {
    let str_value = value.to_string();
    if str_value.len() > max_length {
        format!("{}...", &str_value[..max_length])
    } else {
        str_value
    }
}

/// Создает строку с отступами
#[allow(dead_code)]
pub fn indent(text: &str, level: usize) -> String {
    let indent = "  ".repeat(level);
    text.lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.5s");
        assert_eq!(format_duration(Duration::from_millis(65000)), "1.1m");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500B");
        assert_eq!(format_size(1500), "1.5KB");
        assert_eq!(format_size(1_500_000), "1.4MB");
        assert_eq!(format_size(1_500_000_000), "1.4GB");
    }
}