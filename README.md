# Nexora Printer Manager

[![Rust](https://img.shields.io/badge/rust-1.75%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)

**Nexora Printer Manager** is a high-performance desktop application designed to bridge web-based POS systems with local ESC/POS thermal printers. It provides a robust HTTP API and a user-friendly interface for managing USB, Network, and LPT printer connections.

![Nexora Logo](assets/nexora.png)

## 🚀 Key Features

- **Multi-Interface Support**: Connect to thermal printers via USB, Ethernet (TCP/IP), or Parallel (LPT) ports.
- **RESTful API**: Simple integration for web applications to trigger print jobs.
- **Template System**: Dynamic receipt rendering using JSON-based templates.
- **Auto-Discovery**: Automatic scanning and detection of available printer devices.
- **Cross-Platform Core**: Built with Rust for safety and performance (UI optimized for Windows).
- **Modern UI**: Clean, intuitive interface powered by Slint.

## 🛠️ Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)

### Installation

```bash
# Clone the repository
git clone https://github.com/nexora/printer-manager.git
cd printer-manager

# Fast build (debug)
cargo build

# Run the app
cargo run
```

### Production Build

To create an optimized executable for distribution:

```bash
cargo build --release
```
The executable will be located at `target/release/nexora-printer-manager.exe`.

---

## 📖 Documentation

For detailed information on how to use the application, configure templates, and integrate with the API, please refer to our comprehensive documentation:

👉 **[Detailed Documentation & API Reference](docs/README.md)**

### Quick API Examples:

- **Health Check**: `GET http://localhost:8080/health`
- **Printer Status**: `GET http://localhost:8080/status`
- **Print Receipt**: `POST http://localhost:8080/print-template`

---

## 🔧 Technology Stack

- **[Rust](https://www.rust-lang.org/)**: Core logic and printer communication.
- **[Slint](https://slint.dev/)**: Native UI framework.
- **[Axum](https://github.com/tokio-rs/axum)**: High-performance HTTP server.
- **[ESC/POS](https://crates.io/crates/escpos)**: Native thermal printer command generation.

---

## 🛡️ Patent & Modification Protection

This project is licensed under the **GNU GPLv3**. This was chosen specifically to protect the community:

- **No Patent Traps**: By using or contributing to this software, contributors grant a royalty-free patent license to all users. Anyone attempting to enforce patent litigation against this software forfeits their right to use it.

- **Mandatory Reciprocity**: Any company or individual that modifies this manager and distributes it must share their source code modifications with the public. We believe in keeping printer hardware accessible and open.

---

## 📄 License

This project is licensed under the GPL-3.0-only License - see the [LICENSE](LICENSE) file for details.

© 2026 Nexora POS Team
