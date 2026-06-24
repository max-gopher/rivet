# Makefile
.PHONY: help build release test clean lint format cli gui

help:  ## Показать справку
	@echo "Доступные команды:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build:  ## Собрать все проекты в debug режиме
	cargo build --workspace

release:  ## Собрать все проекты в release режиме
	cargo build --release --workspace

cli:  ## Собрать только CLI
	cargo build -p rivet-cli --release

gui:  ## Собрать только GUI
	cargo build -p rivet-gui --release

test:  ## Запустить все тесты
	cargo test --workspace -- --nocapture

test-integration:  ## Запустить интеграционные тесты
	cargo test --test integration -- --nocapture

lint:  ## Проверить код на ошибки
	cargo clippy --workspace -- -D warnings

format:  ## Форматировать код
	cargo fmt --all

clean:  ## Очистить сборку
	cargo clean

run-cli:  ## Запустить CLI
	cargo run -p rivet-cli -- $(ARGS)

run-gui:  ## Запустить GUI
	cargo run -p rivet-gui

validate-example:  ## Валидировать пример
	cargo run -p rivet-cli -- validate -c examples/test.yaml

install:  ## Установить бинарники
	install -D target/release/rivet-cli ~/.local/bin/rivet
	@echo "✅ Установлено в ~/.local/bin/rivet"

watch:  ## Автоматическая пересборка при изменении файлов
	cargo watch -x "run -p rivet-cli -- $(ARGS)"

coverage:  ## Сгенерировать отчет о покрытии тестами
	cargo tarpaulin --workspace --out Html

# Использование:
# make run-cli ARGS="-c examples/test.yaml"
# make watch ARGS="-c examples/test.yaml"