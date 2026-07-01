//! Rivet GUI - Tauri приложение

#![cfg_attr(
    all(target_os = "windows", not(feature = "dev-console")),
    windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::{Manager, State};
use tokio::sync::Mutex;

use rivet_core::parsers::config::load_and_validate_from_file;
use rivet_core::{TestEngine, TestSuite};

static LOG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();

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
    pub response: ResponseInfo,
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
    log_to_file(&format!("📂 load_config: {}", path));
    let suite = match load_and_validate_from_file(&path) {
        Ok(suite) => suite,
        Err(err) => {
            log_to_file(&format!("❌ load_config failed: {}", err));
            return Err(err.to_string());
        }
    };

    let mut config_guard = state.config.lock().await;
    *config_guard = Some(suite.clone());

    log_to_file("✅ Config loaded and saved");
    Ok(suite)
}

/// Запускает тесты
#[tauri::command]
async fn run_tests(state: State<'_, AppState>) -> Result<TestRunResult, String> {
    log_to_file("🔍 run_tests: start");

    let config = {
        let guard = state.config.lock().await;
        guard.clone()
    };

    let suite = config.ok_or_else(|| "No config loaded".to_string())?;
    log_to_file(&format!("📂 Config loaded: {}", suite.name));

    let engine = TestEngine::with_http_config(suite.http.as_ref());

    let start = std::time::Instant::now();
    log_to_file("🚀 Starting engine.run_detailed");
    let stage_results = match engine.run_detailed(&suite).await {
        Ok(stage_results) => stage_results,
        Err(err) => {
            log_to_file(&format!("❌ engine.run_detailed failed: {}", err));
            return Err(err.to_string());
        }
    };
    log_to_file("✅ engine.run_detailed finished");

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
        stages: stage_results
            .into_iter()
            .map(|r| StageResult {
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
                },
            })
            .collect(),
    };

    {
        let mut guard = state.results.lock().await;
        match serde_json::to_value(&result) {
            Ok(value) => {
                *guard = Some(value);
            }
            Err(err) => {
                log_to_file(&format!("❌ failed to serialize test result: {}", err));
                return Err(err.to_string());
            }
        }
    }

    log_to_file(&format!(
        "✅ run_tests finished: total={}, passed={}, failed={}",
        result.total, result.passed_count, result.failed_count
    ));

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
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = format!("💥 Panic: {:?}", panic_info);
        log_to_file(&msg);

        // Дополнительная информация
        if let Some(location) = panic_info.location() {
            log_to_file(&format!("   at {}:{}", location.file(), location.line()));
        }

        log_to_file(&format!("Backtrace:\n{}", Backtrace::force_capture()));
        wait_for_enter("panic");
    }));

    let context = tauri::generate_context!();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            init_log_file(app);
            log_to_file("Rivet GUI started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_config, run_tests, get_info,])
        .run(context);

    if let Err(err) = result {
        log_to_file(&format!(
            "❌ error while running tauri application: {}",
            err
        ));
        wait_for_enter("Tauri runtime error");
    }
}

fn init_log_file(app: &tauri::App) {
    let log_dir = diagnostic_log_dir().unwrap_or_else(|| {
        app.path()
            .app_log_dir()
            .or_else(|_| app.path().app_data_dir())
            .unwrap_or_else(|_| std::env::temp_dir().join("rivet"))
    });

    if let Err(err) = std::fs::create_dir_all(&log_dir) {
        eprintln!(
            "Failed to create log directory {}: {}",
            log_dir.display(),
            err
        );
        return;
    }

    let _ = LOG_FILE_PATH.set(log_dir.join("rivet.log"));
}

#[cfg(all(target_os = "windows", feature = "dev-console"))]
fn diagnostic_log_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
}

#[cfg(not(all(target_os = "windows", feature = "dev-console")))]
fn diagnostic_log_dir() -> Option<PathBuf> {
    None
}

fn log_to_file(msg: &str) {
    eprintln!("{}", msg);

    let path = LOG_FILE_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("rivet_error.log"));

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{}", msg);
    }
}

#[cfg(all(target_os = "windows", feature = "dev-console"))]
fn wait_for_enter(reason: &str) {
    eprintln!();
    eprintln!("Rivet diagnostic console stopped after {}.", reason);
    eprintln!("Press Enter to close this window...");

    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
}

#[cfg(not(all(target_os = "windows", feature = "dev-console")))]
fn wait_for_enter(_reason: &str) {}
