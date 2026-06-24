#!/bin/bash
# Быстрый запуск тестов с примером

set -e

echo "🚀 Запуск тестов..."
cargo build -p rivet-cli

echo "📝 Валидация конфига..."
cargo run -p rivet-cli -- validate -c examples/test.yaml

echo "🧪 Запуск тестов..."
cargo run -p rivet-cli -- run -c examples/test.yaml -v

echo "✅ Готово!"