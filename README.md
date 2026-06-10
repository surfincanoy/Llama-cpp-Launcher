# llama.cpp Launcher

`llama-server` 图形化启动器 —— 基于 **egui** 构建的跨平台桌面应用。

<p>
  <img src="https://img.shields.io/badge/Rust-1.85+-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/egui-0.31-blue" alt="egui">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
  <img src="https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20windows-lightgrey" alt="Platform">
</p>

---

## 功能一览

| | 特性 | 说明 |
|---|------|------|
| | 配置管理 | 图形化配置可执行文件、模型路径、网络、GPU、上下文等参数 |
| | 模型扫描 | 自动扫描模型目录下的 `.gguf` 文件，下拉选择 |
| | 服务控制 | 一键启动 / 停止 `llama-server` |
| | 状态监控 | 实时显示运行状态及滚动日志 |
| | 持久化 | 配置自动保存为 `llamacpp_config.json` |
| | 自动打开浏览器 | 服务就绪后自动访问 `http://host:port` |
| | 多语言 | 根据系统语言自动切换中文 / 英文 |
| | 应用图标 | 支持 GNOME 活动概览及任务栏显示 |

---

## 快速开始

### 1. 构建

```bash
git clone <repo>
cd llamacpp-laucher
cargo build --release
```

产物位于 `target/release/llamacpp-laucher`。

### 2. 使用

| 步骤 | 操作 |
|------|------|
| 1 | 选择 `llama-server` 可执行文件路径 |
| 2 | 选择模型目录（自动扫描 `.gguf` 文件） |
| 3 | 选择模型文件 |
| 4 | 配置监听地址、端口、GPU 层数、上下文大小 |
| 5 | 点击 **保存配置** |
| 6 | 点击 **启动服务** |
| 7 | 如需停止，点击 **停止服务** |

服务启动后浏览器自动打开，无需手动输入地址。

---

## 配置

配置文件 `llamacpp_config.json` 保存在可执行文件同级目录。

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `executable` | `llama-server` 路径 | — |
| `model_dir` | 模型文件所在目录 | — |
| `model_name` | 模型文件名 | — |
| `host` | 监听地址 | `127.0.0.1` |
| `port` | 监听端口 | `8080` |
| `n_gpu_layers` | GPU 卸载层数（`0`=CPU） | `0` |
| `ctx_size` | 上下文大小 | `4096` |

---

## 多语言

系统区域设置为 `zh` 开头时显示中文，其余显示英文。

检测逻辑：

```rust
if locale.starts_with("zh") { Lang::Zh } else { Lang::En }
```

---

## 项目结构

```
src/
  main.rs     入口，窗口创建与图标加载
  app.rs      主界面布局与交互逻辑
  config.rs   配置数据结构、JSON 读写、模型扫描
  server.rs   进程管理（启动/停止/健康检查）
  i18n.rs     多语言翻译
  icon/       应用图标
```

---

## 技术栈

| 依赖 | 版本 | 用途 |
|------|------|------|
| [eframe](https://github.com/emilk/egui) / [egui](https://github.com/emilk/egui) | 0.31 | GUI 框架 |
| [serde](https://github.com/serde-rs/serde) / serde_json | 1.x | 配置序列化 |
| [rfd](https://github.com/PolyMeilex/rfd) | 0.15 | 原生文件选择对话框 |
| [ureq](https://github.com/algesten/ureq) | 3.x | HTTP 健康检查 |
| [webbrowser](https://github.com/amodm/webbrowser-rs) | 1.x | 自动打开浏览器 |
| [sys-locale](https://github.com/1Password/sys-locale) | 0.3 | 系统语言检测 |
| [image](https://github.com/image-rs/image) | 0.25 | 应用图标解码 |
| libc (Unix) | 0.2 | 进程信号管理 |

---

## 许可证

[MIT](LICENSE)
