# avconnect — Architecture

## Overview

avconnect is a terminal UI (TUI) application built with **ratatui** and **crossterm**. It follows an **Elm-like architecture**: a single `Screen` state machine driven by `AppMessage` events through a pure `reduce()` function. Side effects (AWS calls, SSH exec, SFTP sessions) are handled outside the reducer in `App`.

## Module Structure

```
src/
├── main.rs                   # Terminal init, panic hook, app bootstrap
├── app.rs                    # Event loop, side-effect coordination, dispatch
├── message.rs                # AppMessage enum — all possible state transitions
├── reducer.rs                # Pure fn: (Screen, AppMessage) → Screen
│
├── state/
│   ├── mod.rs
│   ├── screen.rs             # Screen enum (finite state machine)
│   └── models.rs             # Host, PemKey, FileNode, Config, etc.
│
├── services/
│   ├── mod.rs
│   ├── aws.rs                # EC2 describe-instances → Vec<Host>
│   ├── config.rs             # TOML load/save for ~/.avconnect/config.toml
│   ├── host_registry.rs      # Merge AWS + static hosts, dedupe, sort
│   ├── ssh.rs                # exec() on Unix / spawn+wait on Windows
│   └── sftp.rs               # russh/russh-sftp worker thread
│
└── ui/
    ├── mod.rs                # Screen → render dispatch, status hints
    ├── theme.rs              # xterm-256 / 16-color palette detection
    ├── home.rs               # Home menu (SSH, SFTP, Config, Quit)
    ├── host_select.rs        # Fuzzy-searchable host list (shared)
    ├── key_select.rs         # PEM key picker modal
    ├── sftp_browser.rs       # Remote file tree browser
    ├── config_view.rs        # Config edit form
    ├── first_run.rs          # Setup wizard
    └── components/
        ├── error_popup.rs    # Centered error overlay
        └── status_bar.rs     # Bottom keybinding hint bar
```

## State Machine

```
              ┌─────────┐
       ┌─────▶│  Home   │◀──────────────────┐
       │      └────┬────┘                    │
       │           │ SSH / SFTP              │
       │      ┌────▼──────────┐              │
       │      │ LoadingHosts  │              │
       │      └────┬──────────┘              │
       │           │                         │
       │      ┌────▼──────────┐              │
       │      │  SshSelect /  │              │
       │      │SftpSourceSelect│             │
       │      └────┬──────────┘              │
       │           │ Enter / Ctrl+Enter      │
       │      ┌────▼──────────┐              │
       │      │  KeySelect    │              │
       │      │  (optional)   │              │
       │      └────┬──────────┘              │
       │           │                         │
       │    SSH: exec → exit                 │
       │    SFTP: ┌▼──────────────┐          │
       │         │ SftpBrowser    │          │
       │         └─┬──────────────┘          │
       │           │ confirm                 │
       │         ┌─▼──────────────┐          │
       │         │ SftpDestSelect │          │
       │         └─┬──────────────┘          │
       │           │                         │
       │         ┌─▼──────────────┐          │
       │         │ SftpProgress   │──────────┘
       │         └────────────────┘
       │
       └──── Config ◀──── Home
```

Every screen variant lives in `state::Screen`. Transitions are handled exclusively in `reducer::reduce()`.

## Data Flow

```
┌──────────┐    crossterm     ┌──────────┐
│ Terminal  │ ──── events ───▶│  App     │
│          │                  │          │
│          │◀── ratatui ──────│ .draw()  │
└──────────┘    render        └────┬─────┘
                                   │
                        ┌──────────▼──────────┐
                        │  handle_event()     │
                        │  maps KeyEvent →    │
                        │  Vec<AppMessage>    │
                        └──────────┬──────────┘
                                   │
               ┌───────────────────▼───────────────────┐
               │           dispatch(msg)               │
               │  1. reduce(screen, msg) → new screen  │
               │  2. maybe_spawn_load_hosts()          │
               │  3. maybe_spawn_sftp_connect()        │
               │  4. maybe_load_config()               │
               │  5. maybe_save_config()               │
               │  6. draw()                            │
               └───────────────────────────────────────┘
```

Async results (host loading, SFTP responses) arrive via `mpsc::channel` and are processed at the top of the event loop before polling for keyboard input.

## Key Design Decisions

### Elm Architecture

The reducer is a pure function — no I/O, no side effects. This makes state transitions predictable and testable. Side effects are triggered by `App` methods that inspect the screen after each reduce call.

### SFTP Worker Thread

`services::sftp` spawns a dedicated thread with its own tokio runtime. The thread owns the `russh` SSH session and `SftpSession`, processes commands (`ListDir`, `Disconnect`) received over a channel, and sends results back as `AppMessage` variants. This keeps the main thread (which drives the TUI) completely synchronous.

### SSH Execution

On Unix, `ssh` replaces the process via `exec()` — the TUI exits, cleanup happens beforehand, and the user is dropped directly into the SSH session. On Windows, `ssh.exe` is spawned and waited on. In both cases, the app terminates after SSH exits.

### AWS Credential Loading

Primary path: `aws-config` with named profile. Fallback: raw `access_key_id` + `secret_access_key` from config. The first-run wizard detects which approach the user prefers.

### Theme Detection

`crossterm::style::available_color_count()` is checked once at startup. If >= 256, the Rich palette (xterm-256 indexed colors) is used; otherwise, the Basic 16-color palette. All render functions receive `&Theme` — no hardcoded colors.

### Config Schema

```toml
[aws]
profile = "default"
region = "ap-south-1"
# access_key_id = "..."       # optional fallback
# secret_access_key = "..."   # optional fallback

[ssh]
default_key = "~/.ssh/av.pem"
# hosts_file = "~/.avconnect/hosts"  # optional

[[keys]]
name = "prod"
path = "~/.ssh/prod.pem"
```

`ConfigDraft` mirrors `Config` with all `String` fields for in-place editing. Conversion helpers (`config_to_draft`, `draft_to_config`) handle the mapping.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI widgets, layout, rendering |
| `crossterm` | Terminal backend (raw mode, events, Windows support) |
| `tokio` | Async runtime for AWS SDK and russh |
| `aws-config` + `aws-sdk-ec2` | EC2 instance discovery |
| `russh` + `russh-sftp` | Pure-Rust SSH/SFTP (no C deps) |
| `ssh-key` | SSH key type used by russh Handler trait |
| `serde` + `toml` | Config serialization |
| `fuzzy-matcher` | Skim-based fuzzy host filtering |
| `dirs` | Cross-platform `~/.avconnect/` resolution |
| `shellexpand` | Expand `~` in PEM key paths |
| `anyhow` | Ergonomic error handling |
