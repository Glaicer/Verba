# Verba

Rust GTK4 desktop translation utility for Ubuntu. Runs as a `systemd --user` daemon with a tray icon, sends requests to any OpenAI-compatible `/v1/chat/completions` API.

## Features

- **Translation**: Paste text, pick a target language and preset, press Translate. The result appears in the output pane with a Copy button.
- **System tray**: Left-click toggles the main window. Right-click opens a menu with Open/Minimize, Settings, and Exit.
- **CLI control**: `verba toggle|show|hide|settings|quit` talks to the running daemon over D-Bus. `verba toggle` auto-starts the systemd service if the daemon is not running.
- **Presets**: Named translation styles (Precise, Natural, Formal by default) each with a freeform instruction. Fully editable in the built-in preset editor (Settings → Configure Presets).
- **Settings dialog**: Edit provider base URL, model name, and API key. API key is stored in Secret Service (libsecret), never written to `config.toml`. The password field shows "Configured" when a key exists.
- **Desktop notifications**: Translation errors trigger `notify-send` with urgency based on HTTP status (critical for 403/404, normal for 429/5xx/network).
- **Keyboard shortcuts**: `Ctrl+Return` translate, `Escape` close/hide, `Ctrl+L` focus language, `Ctrl+P` focus preset, `Ctrl+Shift+C` copy result.
- **Config**: TOML file at `~/.config/verba/config.toml`. Atomic writes via temp file + rename. File permissions set to 0600. Validated on load and save (URL scheme, temperature range, preset uniqueness).
- **State machine**: `AppState` enum (`Hidden → VisibleIdle → VisibleTranslating → … → Exiting`) behind `Arc<Mutex<RuntimeState>>`. Polled every 100ms by both GTK and tray to sync UI to runtime state.

## Project Structure

```
src/
  main.rs              — binary entrypoint: parses CLI, dispatches to daemon or IPC client
  lib.rs               — re-exports all public modules
  cli.rs               — clap CLI definition (Daemon|Toggle|Show|Hide|Settings|Quit), exit codes
  daemon.rs            — wires ConfigStore → AppRuntime → IPC server (tokio task) + tray thread + GTK main loop
  app_runtime.rs       — AppRuntime (Arc<Mutex<RuntimeState>>), AppState state machine, translate_text(), TranslationOutcome
  error.rs             — VerbaError enum (thiserror), Result<T> alias
  logging.rs           — tracing init with RUST_LOG env (defaults to info)
  config/
    mod.rs             — re-exports
    schema.rs          — AppConfig, ProviderConfig, UiConfig, Preset structs (serde)
    store.rs           — ConfigStore: load_or_create/save with atomic writes, default path ($XDG_CONFIG_HOME/verba/config.toml)
    validation.rs      — validate(): URL scheme, temperature range, preset id/name/uniqueness, rejects /v1/chat/completions in base_url
  gui/
    mod.rs             — GTK Application setup, 100ms exit-poll, present-on-startup flag
    actions.rs         — GuiAction enum, keyboard accelerator mappings
    main_window.rs     — MainWindowController: builds the main window layout, wires all buttons/fields, translation async bridge (mpsc channel + glib poll)
    settings_dialog.rs — SettingsDialog: provider URL + model + API key fields, applies SettingsDraft via ConfigStore + SecretStore
    preset_editor.rs   — PresetEditor + PresetEditorModel: add/delete/edit presets, slugified IDs, validation
  ipc/
    mod.rs             — re-exports
    constants.rs       — D-Bus service name, object path, interface name (dev.aronov.Verba)
    server.rs          — VerbaIpc zbus interface: toggle/show/hide/settings/quit/reload_config + signals + properties
    client.rs          — IPC client: auto-starts systemd service on first failure, retries for 2s
  llm/
    mod.rs             — re-exports
    client.rs          — LlmClient: reqwest POST to {base_url}/v1/chat/completions, bearer auth, status→LlmError mapping
    errors.rs          — LlmErrorKind enum (Unauthorized, NotFound, RateLimited, …), LlmError struct
    prompt.rs          — build_system_prompt(): language + preset instruction + hard rules
    schema.rs          — ChatCompletionRequest/Response serde structs
  notify/
    mod.rs             — Notifier trait + Urgency enum
    notify_send.rs     — NotifySend: spawns `notify-send` with --app-name and --urgency
  secrets/
    mod.rs             — SecretStore trait (async: get/set/clear API key)
    secret_service.rs  — SecretServiceStore: libsecret via secret-service crate, looks up by application=verba + kind=api-key
  tray/
    mod.rs             — re-exports
    indicator.rs       — TrayIndicator (ksni): VerbaTray struct, left-click=toggle, menu with Open/Minimize+Settings+Exit, 100ms state sync thread, 32x32 procedurally-generated pixmap icon
tests/
  cli_config.rs        — CLI parsing, config defaults and validation
  gui_shell.rs         — GUI helper functions (default languages, accelerators, preset selection)
  ipc_runtime.rs       — AppRuntime state machine transitions, IpcCommand mapping, exit codes
  llm_notify.rs        — Prompt format, notify-send args, LlmClient against local TCP mock server, error classification by HTTP status
  translation_flow.rs  — End-to-end translate_text() with SecretStub + RecordingNotifier, validation, success, and error paths
  preset_editor.rs     — PresetEditorModel: add, delete, validate, slugify, uniqueness
  settings_dialog.rs   — SettingsDraft validation, apply_settings with stub secrets
  tray_indicator.rs    — Tray menu labels, left-click behavior, state sync polling
  packaging_layout.rs  — Verifies installed file layout matches packaging scripts
packaging/
  scripts/
    install.sh         — cargo build --release + install binary, .desktop, icon, metainfo, systemd service
    uninstall.sh       — stops/disables service, removes all installed files
    build-deb.sh       — builds .deb package
  systemd/
    verba.service      — systemd --user unit, PartOf=graphical-session.target, RUST_LOG=info
  linux/
    verba.desktop      — desktop entry
  icons/
    hicolor/scalable/apps/verba.svg
  metainfo/
    dev.aronov.Verba.metainfo.xml
```

## Build & Run

```bash
cargo build --release
```

Full system install (builds release binary, installs desktop files, enables systemd user service):

```bash
packaging/scripts/install.sh
```

System deps: GTK 4 dev libraries, `pkg-config`, Rust toolchain.

## Tests

```bash
cargo test
```

All tests are pure unit tests in `tests/`. No external services, databases, or GUI session required. Tests use hand-written trait stubs (e.g. `SecretStub`, `RecordingNotifier`) — no mock framework.

## Key Patterns

- Error handling: two error enums — `VerbaError` (app-level) and `LlmError`/`LlmErrorKind` (LLM-specific, mapped to user messages).
- Secrets: API key stored in Secret Service, never written to `config.toml`.
- Logging: `tracing` crate, controlled via `RUST_LOG` env var (defaults to `info`).
- IPC: CLI commands talk to the running daemon over D-Bus session bus. `verba toggle` auto-starts `verba.service` via systemd if the daemon is not running.
- Trait-based abstraction at module boundaries (`SecretStore`, `Notifier`) enables unit testing without real system services.
- Async bridge: GUI translates via `tokio::spawn` → `mpsc::channel` → `glib::timeout_add_local` poll (50ms). No async on the GTK thread.
- Config writes are atomic (temp file + rename) with 0600 permissions.

## Verification after Changes

```bash
cargo test && cargo build --release
```
