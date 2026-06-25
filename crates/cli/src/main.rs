#![allow(dead_code)]

mod commands;
mod output;

use clap::Parser;
use commands::Cli;
use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Настройка логирования
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("rivet=info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // 2. Парсинг аргументов
    let cli = Cli::parse();

    // 3. Выполнение команды
    commands::execute(cli).await?;

    Ok(())
}