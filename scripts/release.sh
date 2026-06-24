#!/bin/bash
# Создание релиза

set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "❌ Укажите версию: ./scripts/release.sh 0.1.0"
    exit 1
fi

echo "🚀 Создание релиза v$VERSION..."

# Обновление версии во всех Cargo.toml
sed -i "s/version = \".*\"/version = \"$VERSION\"/g" Cargo.toml
find crates -name "Cargo.toml" -exec sed -i "s/version = \".*\"/version = \"$VERSION\"/g" {} \;

# Сборка релиза
cargo build --release --workspace

# Создание архива
mkdir -p dist
cp target/release/rivet-cli dist/
cp target/release/rivet-gui dist/

tar -czf rivet-v$VERSION.tar.gz -C dist .

echo "✅ Релиз готов: rivet-v$VERSION.tar.gz"