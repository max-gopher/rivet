# Rivet — REST API Testing Framework

Rivet — это современный фреймворк для тестирования REST API с поддержкой CLI и GUI интерфейсов.

## Особенности

- Простой YAML синтаксис
- Мощные проверки: статусы, JSONPath, заголовки, AND/OR/NOT
- Извлечение данных из ответов
- Поддержка авторизации: Bearer, Basic, API Key
- CLI интерфейс для CI/CD
- GUI интерфейс на Tauri + Svelte
- Кроссплатформенность: Linux, Windows, macOS
- Готовые шаблоны

## Установка

### Linux (Debian/Ubuntu)

wget https://github.com/max-gopher/rivet/releases/latest/download/rivet_0.1.0_amd64.deb
sudo dpkg -i rivet_0.1.0_amd64.deb

### Linux (AppImage)

chmod +x rivet_0.1.0_amd64.AppImage
./rivet_0.1.0_amd64.AppImage

### Windows

Скачайте .msi или .exe из релизов и запустите установщик.

### macOS

Скачайте .dmg из релизов, откройте и перетащите в Applications.

### Сборка из исходников

git clone https://github.com/max-gopher/rivet.git
cd rivet
cargo build -p rivet-cli --release
cargo build -p rivet-gui --release

## Пример теста

Создайте test.yaml:

name: Simple API Test
variables:
base_url: https://jsonplaceholder.typicode.com
stages:
- name: Get Users
  request:
  method: GET
  url: {{base_url}}/users
  assert:
    - status: 200
    - body:
      path: [0].id
      not_null: true

Запустите:

rivet run -c test.yaml

## CLI Команды

rivet run -c test.yaml
rivet run -c test.yaml --env-prefix RIVET_
rivet run -c test.yaml -o report.json
rivet validate -c test.yaml
rivet template generate --template default -o test.yaml
rivet template list
rivet info

## GUI Интерфейс

Запустите rivet-gui:

1. Загрузите YAML файл
2. Запустите тесты
3. Просмотрите результаты
4. Посмотрите детали запросов и ответов

## Переменные окружения

rivet run -c test.yaml --env-prefix RIVET_

В YAML:

env:
API_KEY: {{env.API_KEY}}
headers:
Authorization: Bearer {{env.API_KEY}}

## Шаблоны

rivet template list
rivet template generate --template default -o test.yaml

Пользовательские шаблоны:
- Linux: ~/.local/share/rivet/templates/
- Windows: %APPDATA%/rivet/templates/

## Разработка

cargo run -p rivet-cli -- run -c examples/demo.yaml
cargo run -p rivet-gui
cargo test --workspace
cargo clippy --workspace -- -D warnings

## Лицензия

MIT OR Apache-2.0

## Благодарности

- Tauri
- Svelte
- Rust

Поставьте звезду на GitHub, если проект полезен!