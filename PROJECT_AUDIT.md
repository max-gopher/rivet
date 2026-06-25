# Rivet Rust Project Audit

Дата аудита: 2026-06-23

## Краткий итог

Проект выглядит как ранний MVP REST API testing framework с тремя основными частями:

- `rivet-core`: загрузка YAML, валидация, HTTP-клиент, выполнение stages, extraction/assertions, шаблоны.
- `rivet-cli`: CLI-обертка над core и системой шаблонов.
- `rivet-gui`: Tauri/Svelte GUI над core.

Текущая сборка уже работоспособна, но качество gate пока не зеленый: workspace build проходит, frontend build проходит, но workspace tests и clippy с `-D warnings` падают. Самые важные проблемы сейчас не инфраструктурные, а поведенческие: GUI не сохраняет загруженный конфиг в состояние, CLI-тесты аварийно завершают test process, часть заявленных request/assertion возможностей описана в моделях, но не выполняется.

## Проверки

Команды, которые были запущены:

| Проверка | Результат |
| --- | --- |
| `cargo build --workspace` | OK, есть warnings в `rivet-cli` |
| `cargo build --bin rivet-gui` | OK |
| `npm run build` в `crates/gui` | OK, есть предупреждение Svelte/Vite |
| `cargo test --workspace` | FAIL |
| `cargo test -p rivet-cli --bin rivet -- --nocapture` | FAIL |
| `cargo clippy --workspace -- -D warnings` | FAIL, 26 errors |

### Детали падений

`cargo test --workspace` падает на unit-тестах `rivet-cli`. Подробный запуск показывает, что оба validate-теста используют YAML:

```yaml
method: GET
```

Но `HttpMethod` десериализуется как lowercase (`get`, `post`, ...). В результате валидный CLI-тест становится невалидным, затем `validate_command` вызывает `std::process::exit(1)`, и весь test binary завершается аварийно.

`cargo clippy --workspace -- -D warnings` падает на 26 замечаниях. Большинство замечаний механические (`collapsible_if`, `manual_map`, `for_kv_map`, `redundant_closure`), но есть и структурные: `HttpClient::default()` конфликтует по смыслу с `Default::default`, `TestContext::new()` и `StageResult::new()` требуют `Default`.

`npm run build` проходит, но выводит:

```text
[vite-plugin-svelte] you are building for production but compilerOptions.dev is true, forcing it to false
```

Причина в `crates/gui/vite.config.js`: `compilerOptions.dev` вычисляется как `!process.env.TAURI_DEBUG`, поэтому обычный production build без `TAURI_DEBUG` получает `dev: true`.

## Приоритетные findings

### 1. GUI не сохраняет загруженный конфиг в `AppState`

Файл: `crates/gui/src-tauri/src/main.rs:39`

`load_config(path)` возвращает `TestSuite`, но не принимает `State<AppState>` и не записывает результат в `state.config`. Затем `run_tests` читает `state.config` и получает `None`, поэтому нормальный сценарий "load config -> run tests" в GUI должен завершаться ошибкой `No config loaded`.

Затронутые строки:

- `load_config` только загружает и возвращает suite: `crates/gui/src-tauri/src/main.rs:39`
- `run_tests` читает `state.config`: `crates/gui/src-tauri/src/main.rs:51`
- ошибка при пустом state: `crates/gui/src-tauri/src/main.rs:56`

Рекомендация:

- Передать `State<'_, AppState>` в `load_config`.
- После успешной загрузки записывать `Some(suite.clone())` в `state.config`.
- Добавить GUI/backend unit или integration smoke на сценарий load/run.

### 2. CLI-команды вызывают `std::process::exit` внутри тестируемой логики

Файл: `crates/cli/src/commands.rs`

Внутренние command-функции напрямую завершают процесс:

- failed test run: `crates/cli/src/commands.rs:204`
- invalid config: `crates/cli/src/commands.rs:246`
- missing template: `crates/cli/src/commands.rs:393`
- failed repository access: `crates/cli/src/commands.rs:443`
- missing directory: `crates/cli/src/commands.rs:454`

Это ломает unit-тесты и делает поведение трудно переиспользуемым. Особенно заметно в тестах `test_validate_command_valid` / `test_validate_command_invalid`: `crates/cli/src/commands.rs:532`.

Рекомендация:

- В command-функциях возвращать `Result<CommandOutcome>` или custom error.
- Решение о process exit code оставить только в `main`.
- В тестах проверять возвращаемый error/outcome, а не ловить завершение процесса.

### 3. YAML examples и тесты используют неверный case для HTTP method

`HttpMethod` объявлен с `#[serde(rename_all = "lowercase")]`, значит валидные значения: `get`, `post`, `put`, etc. Но CLI-тесты используют `GET`: `crates/cli/src/commands.rs:540` и `crates/cli/src/commands.rs:557`.

Рекомендация:

- Либо привести все examples/tests к lowercase.
- Либо сделать десериализацию методов case-insensitive, что удобнее для пользователей API testing tool.

### 4. Модели конфигурации обещают больше, чем реально выполняет engine

Файл: `crates/core/src/engine.rs:193`

В моделях есть `RequestBody::Raw`, `RequestBody::Multipart`, `Auth`, `params`, per-stage `retry`, per-stage `timeout`, SSL override. В engine часть этих полей не используется:

- `Raw` и `Multipart` игнорируются: `crates/core/src/engine.rs:215`
- `Text` и `Form` преобразуются в `serde_json::Value`, а потом HTTP client всегда отправляет `.json(body)`, то есть form/text фактически уходят как JSON.
- `request.params`, `request.auth`, `request.validate_ssl`, `stage.retry`, `stage.timeout` сейчас не применяются в `build_http_request` / `HttpClient`.
- `Custom` assertions явно не реализованы: `crates/core/src/engine.rs:346`

Риск: пользователь может написать валидный YAML, который выглядит поддержанным, но исполняется иначе или молча игнорируется.

Рекомендация:

- Разделить request body на transport-level enum в `HttpRequest`: JSON/text/form/raw/multipart.
- Добавить query params и auth в request builder.
- Либо временно запретить неподдержанные поля в validator, чтобы fail fast.

### 5. GUI результат выполнения не содержит stage-level данных

Файл: `crates/gui/src-tauri/src/main.rs:69`

`TestRunResult` формируется агрегированно:

- `passed_count` равен `suite.stages.len()` только если все прошло.
- `failed_count` равен `1` при любом провале.
- `stages` всегда `vec![]`: `crates/gui/src-tauri/src/main.rs:75`

Риск: UI не сможет показать пользователю реальные failed stages, статусы, ошибки и длительности.

Рекомендация:

- Изменить `TestEngine::run` так, чтобы он возвращал структурированный отчет, а не только `bool`.
- Либо добавить отдельный API получения сохраненных `StageResult` из `TestContext`.

### 6. Тест HTTP-клиента зависит от внешней сети

Файл: `crates/core/src/http/client.rs:233`

`test_http_request_building` ходит в `https://httpbin.org/get`: `crates/core/src/http/client.rs:237`.

Риск: flaky tests в CI/offline окружении. Сейчас тест мягко проходит при ошибке (`if let Ok(resp)`), поэтому он также может давать ложное чувство покрытия.

Рекомендация:

- Заменить на локальный mock server.
- Если нужен smoke against internet, вынести в ignored/integration test.

### 7. Build scripts и examples указывают на несуществующий файл

Файлы:

- `Makefile:41`
- `scripts/quick-test.sh:10`

Обе команды используют `examples/test.yaml`, но такого файла в репозитории нет. Есть `examples/auth.yaml`, `examples/request.yaml`, `examples/test_with_http_config.yaml`.

Рекомендация:

- Обновить команды на существующий пример.
- Добавить `make validate-example` в CI после исправления examples.

### 8. `.gitignore` слишком узкий

Файл: `.gitignore:1`

Сейчас игнорируется только `/target`. В рабочем дереве видны `crates/gui/node_modules/` и generated schemas `crates/gui/gen/`. `node_modules` почти никогда не должен попадать в git.

Рекомендация:

- Добавить минимум:

```gitignore
node_modules/
crates/gui/node_modules/
crates/gui/dist/
crates/gui/gen/
.env*
!.env.example
```

С `.env*` осторожно: сейчас `.env.dev` и `.env.prod` уже добавлены в index; нужно проверить, нет ли там секретов.

### 9. Tauri frontend build workflow остается ручным

Файл: `crates/gui/tauri.conf.json:6`

`beforeBuildCommand` пустой, а `frontendDist` указывает на `src-tauri/dist`: `crates/gui/tauri.conf.json:9` и `crates/gui/tauri.conf.json:10`.

При этом `npm run build` генерирует стандартный `crates/gui/dist`, не `src-tauri/dist`. Сейчас Rust build проходит только потому, что `src-tauri/dist/index.html` уже есть.

Рекомендация:

- Выбрать один путь:
  - либо `frontendDist: "dist"` и `beforeBuildCommand: "npm run build"`;
  - либо настроить Vite `build.outDir = "src-tauri/dist"`.
- Для Tauri v2 обычно проще держать `frontendDist: "dist"` рядом с `tauri.conf.json`.

### 10. Workspace metadata содержит placeholders

Файл: `Cargo.toml:11`

Поля `authors` и `repository` пока шаблонные:

- `authors = ["Your Name <email@example.com>"]`: `Cargo.toml:14`
- `repository = "https://github.com/yourname/rivet_rust"`: `Cargo.toml:16`

Рекомендация:

- Заполнить реальными значениями до публикации/релиза.

## Security notes

- В `tauri.conf.json` установлен `csp: null`: `crates/gui/tauri.conf.json:24`. Для dev это допустимо, для production лучше задать CSP.
- В core есть возможность отключать SSL verification через `verify_ssl = false`, что применяется через `danger_accept_invalid_certs(true)`: `crates/core/src/http/client.rs:35`. Это нужно явно документировать как небезопасный режим.
- В examples есть токены/пароли-заглушки и env placeholders. Реальных секретов при поверхностном поиске не найдено, но `.env.dev` и `.env.prod` находятся под git status как добавленные/измененные; их стоит проверить вручную перед коммитом.

## Что уже хорошо

- Workspace разнесен по понятным crate boundaries: `core`, `cli`, `gui`.
- Core имеет собственные типы ошибок и отдельные модули для config/response/template/http.
- Есть unit-тесты для parser/validator/jsonpath/extractor/template.
- Tauri GUI теперь собирается, и иконки сгенерированы через `tauri icon`.
- CLI имеет зачатки нормального UX: colored output, progress spinner, report output, templates.

## Рекомендуемый порядок исправлений

1. [x] Починить `load_config` в GUI, чтобы он сохранял suite в `AppState`.
2. [x] Убрать `std::process::exit` из command-функций CLI и починить CLI-тесты.
3. [x] Привести HTTP method parsing к user-friendly case-insensitive режиму.
4. [x] Согласовать Tauri/Vite build output (`dist` vs `src-tauri/dist`) и добавить `beforeBuildCommand`.
5. [x] Расширить `.gitignore`, убрать `node_modules` и generated artifacts из git.
6. [x] Сделать clippy green с `cargo clippy --workspace -- -D warnings`.
7. [x] Заменить внешний `httpbin.org` тест локальным mock server. - Оставляем как есть. Лучше проверять на реальном сервисе.
8. [ ] Либо реализовать `Raw/Form/Text/Multipart/Auth/params/timeout/retry`, либо запретить неподдержанное в validator.
   1. [ ] Raw
   2. [ ] Multipart
   3. [ ] validate_ssl
   4. [ ] Custom assertions
9. [ ] Добавить README с install/run/examples/troubleshooting.
10. [ ] Добавить CI: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `npm ci`, `npm run build`.

## Быстрый статус готовности

| Область | Статус | Комментарий |
| --- | --- | --- |
| Rust build | Зеленый | `cargo build --workspace` проходит |
| GUI build | Зеленый | `cargo build --bin rivet-gui` и `npm run build` проходят |
| Tests | Красный | CLI tests падают из-за YAML method case + `process::exit` |
| Clippy strict | Красный | 26 errors с `-D warnings` |
| Docs | Красный | `README.md` пустой |
| Packaging | Желтый | Иконки есть, но frontendDist workflow надо согласовать |
| Security hardening | Желтый | CSP отключен, SSL bypass возможен, env files нужно проверить |

