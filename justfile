# justfile
# Установка: cargo install just

set shell := ["bash", "-uc"]

# Показать все команды
default:
    @just --list

# Сборка
build:
    cargo build --workspace

release:
    cargo build --release --workspace

# CLI
cli *args:
    cargo run -p rivet-cli -- {{args}}

run config:
    cargo run -p rivet-cli -- run -c {{config}}

validate config:
    cargo run -p rivet-cli -- validate -c {{config}}

# GUI
gui:
    cargo run -p rivet-gui

# Тестирование
test:
    cargo test --workspace

test-verbose:
    cargo test --workspace -- --nocapture

test-integration:
    cargo test --test integration -- --nocapture

# Качество кода
lint:
    cargo clippy --workspace -- -D warnings

format:
    cargo fmt --all

# Очистка
clean:
    cargo clean

# Установка
install:
    install -D target/release/rivet-cli ~/.local/bin/rivet

# Помощь
help:
    @just --list --unstable