# 环境准备

Tauri 2 应用构建需要 Rust 工具链 + 前端 Node 环境，并按目标平台安装系统 WebView 依赖。

## 1. Rust 工具链

```bash
# 官方安装脚本（macOS / Linux）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Windows：安装 rustup 后同样得到 cargo/rustc
rustc --version   # 需 >= 1.77.2
cargo --version
```

> 国内网络可先配置 rustup 镜像：`export RUSTUP_DIST_SERVER=https://rsproxy.cn`、`export RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup`，并把 crates.io 源指向 `https://rsproxy.cn`（见 `~/.cargo/config.toml`）。

## 2. Node.js

`dsh` 服务本身是 Node 程序，桌面壳需要通过 Node 启动它。

- 安装 Node.js **>= 20**（推荐 LTS）。macOS/Linux 可用 nvm，Windows 用官网安装包。
- 验证：`node --version`、`npm --version`。

```bash
# 安装前端依赖与 Tauri CLI
npm install
# 安装 dsh 服务端运行时（写入 server/node_modules）
npm run setup:server
```

## 3. 系统 WebView 依赖

Tauri 2 使用系统自带的 WebView：

| 平台 | 依赖 | 说明 |
|---|---|---|
| Windows | **WebView2 Runtime** | Win10/11 大多已预装；缺失时安装程序需先装 [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)。构建还需 MSVC（Visual Studio Build Tools，勾选「使用 C++ 的桌面开发」）。 |
| macOS | 无 | 系统自带 WKWebView。构建需 Xcode Command Line Tools（`xcode-select --install`）。 |
| Linux | **webkit2gtk 4.1** 等 | 见下方命令。 |

### Linux（Debian/Ubuntu）

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl wget file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### Linux（Fedora / Arch）

```bash
# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libappindicator-gtk3-devel librsvg2-devel
# Arch
sudo pacman -S webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg
```

## 4. Tauri CLI

推荐作为 npm devDependency（本项目 `package.json` 已包含 `@tauri-apps/cli`），这样 `npm run tauri` 即用，无需全局安装。也可全局装：

```bash
npm install -g @tauri-apps/cli   # 可选
```

验证：

```bash
npm run tauri -- --version
# 或
npx tauri --version
```

## 5. 校验清单

| 命令 | 预期 |
|---|---|
| `rustc --version` | >= 1.77.2 |
| `node --version` | >= 20 |
| `npm run tauri -- --version` | 输出 tauri 版本 |
| `npm run setup:server` | `server/node_modules/@deepseek-ai/dsh` 存在 |

全部通过后即可进入[开发流程](02-development.md)。
