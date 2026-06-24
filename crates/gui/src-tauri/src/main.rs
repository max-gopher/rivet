//! Rivet GUI - Tauri приложение

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

use rivet_core::{TestEngine, TestSuite};
use rivet_core::parsers::config::load_and_validate_from_file;

/// Состояние приложения
#[derive(Default)]
struct AppState {
    config: Arc<Mutex<Option<TestSuite>>>,
    results: Arc<Mutex<Option<serde_json::Value>>>,
}

/// Результат выполнения теста
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    pub passed: bool,
    pub total: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub duration_ms: u128,
    pub stages: Vec<StageResult>,
}

/// Результат одного этапа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u128,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub request: RequestInfo,
    pub response: ResponseInfo
}

/// Информация о запросе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

/// Информация об ответе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseInfo {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// Загружает конфиг из файла
#[tauri::command]
async fn load_config(path: String, state: State<'_, AppState>) -> Result<TestSuite, String> {
    let suite = load_and_validate_from_file(&path)
        .map_err(|e| e.to_string())?;

    let mut config_guard = state.config.lock().await;
    *config_guard = Some(suite.clone());

    Ok(suite)
}

/// Запускает тесты
#[tauri::command]
async fn run_tests(
    state: State<'_, AppState>,
) -> Result<TestRunResult, String> {
    eprintln!("🔍 run_tests: start");

    let config = {
        let guard = state.config.lock().await;
        guard.clone()
    };

    let suite = config.ok_or_else(|| "No config loaded".to_string())?;
    eprintln!("🔍 run_tests: config loaded");

    let engine = TestEngine::with_http_config(suite.http.as_ref());

    let start = std::time::Instant::now();
    println!("🔍 run_tests: starting engine.run_detailed");
    let stage_results = engine.run_detailed(&suite).await
        .map_err(|e| e.to_string())?;
    eprintln!("🔍 run_tests: engine.run_detailed finished");

    let total = stage_results.len();
    let passed_count = stage_results.iter().filter(|r| r.passed).count();
    let failed_count = total - passed_count;
    let all_passed = failed_count == 0;

    let duration = start.elapsed();

    let result = TestRunResult {
        passed: all_passed,
        total,
        passed_count,
        failed_count,
        duration_ms: duration.as_millis(),
        stages: stage_results.into_iter().map(|r| StageResult {
            name: r.name,
            passed: r.passed,
            duration_ms: r.duration.as_millis(),
            status: r.status,
            error: r.error,
            request: RequestInfo {
                method: r.request.method,
                url: r.request.url,
                headers: r.request.headers,
                params: r.request.params,
                body: r.request.body,
            },
            response: ResponseInfo {
                status: r.response.status,
                headers: r.response.headers,
                body: r.response.body,
            }
        }).collect(),
    };

    {
        let mut guard = state.results.lock().await;
        *guard = Some(serde_json::to_value(&result).unwrap());
    }

    Ok(result)
}

/// Получает информацию о программе
#[tauri::command]
fn get_info() -> serde_json::Value {
    serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
    })
}

fn main() {
    // Tauri сам найдет tauri.conf.json в корне пакета
    let context = tauri::generate_context!();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            load_config,
            run_tests,
            get_info,
        ])
        .run(context)
        .expect("error while running tauri application");
}