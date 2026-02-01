# nm-wifi

A fast Terminal User Interface (TUI) for managing Wi-Fi connections on Linux using NetworkManager.

## Features

- **Network Scanning** - Automatically scans and lists available Wi-Fi networks
- **Connection Management** - Connect to and disconnect from networks directly from the terminal
- **Signal Strength Visualization** - Visual signal strength bars with percentage display
- **Security Indicators** - Shows which networks are secured (WPA/WPA2)
- **Frequency Band Display** - Identifies 2.4 GHz and 5 GHz networks
- **Password Input** - Secure password entry with visibility toggle
- **Vim-style Navigation** - Use `j`/`k` or arrow keys to navigate
- **Catppuccin Theme** - Modern, visually appealing color scheme

## Requirements

- **Linux** with NetworkManager running
- **D-Bus** system bus access
- **nmcli** command-line tool (part of NetworkManager)

## Installation

### Using Cargo

```bash
cargo install --path .
```

### Using Nix

```bash
# Run directly
nix run github:cfcosta/nm-wifi

# Or build
nix build github:cfcosta/nm-wifi
```

### From Source

```bash
git clone https://github.com/cfcosta/nm-wifi
cd nm-wifi
cargo build --release
./target/release/nm-wifi
```

## Usage

Launch the application:

```bash
nm-wifi
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `Enter` / `c` | Connect to selected network |
| `d` | Disconnect from current network |
| `r` | Rescan for networks |
| `i` | Show network details |
| `h` | Show help |
| `Tab` | Toggle password visibility (in password input) |
| `Esc` / `q` | Quit / Cancel |

### Interface

The interface displays:
- Network name (SSID)
- Connection status (connected networks shown with link icon)
- Security status (lock icon for secured networks)
- Frequency band (2.4G or 5G)
- Signal strength (percentage and visual bar)

Connected networks appear at the top of the list, followed by other networks sorted by signal strength.

## Development

### Prerequisites

This project uses Nix for reproducible development environments:

```bash
# Enter development shell
nix develop

# Or with direnv
direnv allow
```

### Build and Run

```bash
# Build
cargo build

# Run
cargo run

# Run tests
cargo test

# Format code
treefmt
```

### Toolchain

The project uses Rust nightly (configured in `rust-toolchain.toml`) with:
- clippy
- rustfmt
- rust-analyzer

## Technical Details

- Built with [ratatui](https://ratatui.rs) for the TUI
- Uses [crossterm](https://crates.io/crates/crossterm) as the terminal backend
- Communicates with NetworkManager via D-Bus
- Uses `nmcli` for connection operations
- Async runtime powered by [tokio](https://tokio.rs)

## License

This project is dual-licensed under MIT OR Apache-2.0. See [LICENSE](LICENSE) and [LICENSE-APACHE](LICENSE-APACHE) for details.
