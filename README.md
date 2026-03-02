# avconnect

TUI tool for SSH host selection and SFTP file transfer, with AWS EC2 instance discovery and static host support. Single cross-platform Rust binary.

## Features

- Fuzzy-searchable host list from AWS EC2 + static hosts file
- SSH connect with configurable PEM key selection
- SFTP file browser and transfer between remote hosts
- First-run setup wizard and editable config screen
- Adaptive theme: xterm-256 or 16-color fallback

## Build

```
cargo build --release
```

The release binary is optimized for size (`opt-level = "z"`, `lto`, `strip`).

## Configuration

Config lives at `~/.avconnect/config.toml`. On first launch, a wizard walks you through:

1. AWS profile name (or raw access key / secret)
2. AWS region
3. Default PEM key path

Static hosts can be added via a hosts file (one per line: `name address [username]`).

## Usage

```
./avconnect
```

| Key | Action |
|-----|--------|
| ↑/↓ | Navigate |
| Enter | Select / Confirm |
| Ctrl+Enter | Choose PEM key (on host list) |
| Tab / Shift+Tab | Next / prev config field |
| Esc | Go back |
| q | Quit (home screen) |

## License

MIT
