# Verba

Verba is a Rust GTK desktop translation utility for Ubuntu and Fedora. It runs as a `systemd --user` daemon, shows a tray icon, stores your API key in Secret Service, and sends translation requests to an OpenAI-compatible `/v1/chat/completions` API.

<p align="center">
    <img src="resources/verba_screenshot.png" alt="Verba translation utility" width="auto" height="375" />
</p>

## Features

- Translation via any OpenAI-compatible API
- `verba toggle` — show/hide the window, ideal for binding a global hotkey
- Presets and a built-in preset editor

## Hotkeys

- `Ctrl+Enter` - translate.
- `Ctrl+Shift+C` - copy result.
- `Ctrl+L` - focus language selector
- `Ctrl+P` - focus preset selector
- `Esc` - hide Verba window

## Installation

Download the latest `.deb` or `.rpm` from the releases section and install.

Run to make Verba start with system:

```bash
systemctl --user enable --now verba.service
```

## API configuration

Open Settings and enter:

- Base URL: the provider API root.
- Model: the provider's exact model ID.
- API key: stored in Secret Service.

Examples:

- OpenRouter base URL: `https://openrouter.ai/api`

Verba appends `/v1/chat/completions` automatically.

## Commands

```bash
verba daemon
verba toggle
verba show
verba hide
verba settings
verba quit
```

`verba daemon` runs the user service. The other commands control the running daemon through the D-Bus session bus.

## Manual installation

Requires Rust toolchain (`cargo`), GTK 4 development libraries, `pkg-config`, and D-Bus development headers.

Ubuntu:

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libdbus-1-dev
```

Fedora:

```bash
sudo dnf install gcc pkg-config gtk4-devel dbus-devel
```

From the repository root:

```bash
packaging/scripts/install.sh
```

The installer builds the release binary, installs the desktop integration files
under `/usr`, reloads the user systemd manager, and starts the service:

```bash
systemctl --user enable --now verba.service
```

Don't run the whole installer with `sudo`. The script uses `sudo`
when it needs to install files, but `systemctl --user` must run as your graphical user. To install into a temporary prefix for testing, pass `PREFIX` and disable `sudo`:

```bash
PREFIX=/tmp/verba-install SUDO= packaging/scripts/install.sh
```
