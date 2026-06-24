#!/bin/bash
# Среда разработки

set -e

echo "🔧 Настройка разработки..."

# Проверка установленных инструментов
command -v mold >/dev/null 2>&1 || echo "⚠️  mold не установлен (рекомендуется для быстрой сборки)"
command -v just >/dev/null 2>&1 || echo "⚠️  just не установлен (рекомендуется для управления задачами)"

# Установка необходимых компонентов
rustup component add rustfmt clippy rust-analyzer

# Сборка в debug режиме
echo "📦 Сборка..."
cargo build --workspace

echo "✅ Готово! Используй:"
echo "  cargo cl run -c examples/test.yaml"
echo "  cargo gui"