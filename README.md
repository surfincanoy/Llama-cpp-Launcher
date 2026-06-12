<div align="center">

# 🦙 llama.cpp Launcher

> A GUI launcher for llama.cpp server based on egui

![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=flat&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=flat&logo=mit)
![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Windows-blue?style=flat&logo=linux)
![egui](https://img.shields.io/badge/GUI-egui-purple?style=flat)

![screenshot](screenshot.png)

</div>

[🇺🇸 English](README.md) | [🇨🇳 中文](README.zh-CN.md)

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🖥️ **GUI Configuration** | Visual configuration of llama-server without memorizing command-line parameters |
| 💾 **Multi-Profile Management** | Save/load named configurations, switch between environments and models |
| 🌐 **Auto Language Detection** | Automatically detects system language (Chinese/English) |
| 🧠 **Advanced Parameters** | GPU layers, context size, Flash Attention, MTP Token Prediction |
| 👁️ **Vision Model** | Support for mmproj vision model loading |
| ⌨ **Command Line Preview** | Real-time command line display with manual extra-args editing |
| 🚀 **One-Click Start/Stop** | Start/stop server with automatic health check and port conflict detection |

---

## 🚀 Quick Start

1. **Select Executable** — Browse and select llama-server
2. **Select Model Directory** — Choose the folder containing your models
3. **Select Model File** — Pick a model from the dropdown list
4. **Configure Parameters** — Port, GPU layers, context size, etc.
5. **Start Server** — Click the「▶ Start」button

> 💡 Configurations can be saved as named profiles for quick switching via「📂 Load」.

---

## 📦 Build

### Linux

```bash
cargo build --release
```

Output: `target/release/llamacpp-launcher`

### Windows Cross-Compilation

Requires MinGW-w64:

```bash
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
./build.sh x86_64-pc-windows-gnu
```

Output: `target/x86_64-pc-windows-gnu/release/llamacpp-launcher-win-x86_64.exe`

### Release Optimizations

`Cargo.toml` is configured with:
- LTO (Link-Time Optimization)
- Strip (debug symbols removed)
- opt-level = "z" (minimal binary size)
- codegen-units = 1

---

## ⚙️ Configuration

The config file `llamacpp_config.json` is saved alongside the executable with multi-profile support:

```json
{
  "__meta__": { "last_profile": "Local Dev" },
  "Local Dev": {
    "executable": "/path/to/llama-server",
    "model_dir": "/path/to/models",
    "model_name": "model-name-q4_k_m",
    "mmproj": "",
    "host": "127.0.0.1",
    "port": 11433,
    "n_gpu_layers": 30,
    "ctx_size": 4096,
    "flash_attn": "auto",
    "mtp_enabled": false,
    "spec_draft_n_max": 2,
    "command_text": ""
  }
}
```

> ⚠️ Switching profiles does not auto-save current changes. Click「💾 Save」to persist.

---

## 🧩 Dependencies

### Rust Crates

| Crate | Purpose |
|-------|---------|
| [eframe](https://github.com/emilk/egui) | GUI framework |
| [rfd](https://github.com/PolyMeilex/rfd) | Native file dialog |
| [serde](https://github.com/serde-rs/serde) | Serialization |
| [serde_json](https://github.com/serde-rs/json) | JSON config persistence |
| [ureq](https://github.com/algesten/ureq) | HTTP health check |
| [sys-locale](https://github.com/rust-utils/sys-locale) | System locale detection |

### System Dependencies

- **Linux**: `psmisc` (provides `fuser` for port conflict detection)
  ```bash
  sudo apt install psmisc
  ```
- **Windows Cross-Compilation**: `mingw-w64`

---

## 📄 License

MIT License
