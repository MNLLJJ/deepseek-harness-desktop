# DeepSeek Harness 桌面应用（Tauri 2）

用 Tauri 2 把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）的 Web UI 打包成跨平台桌面应用，补齐浏览器版在体验上的短板。

## 方案概览

`dsh` 是一个 Node.js 编写的 agent harness（Cordis 插件架构），`dsh web` 会启动一个本地服务，默认监听 `http://127.0.0.1:3080`，前端与后端之间走 HTTP + WebSocket。

因此本方案采用 **sidecar 模式**：Tauri 桌面壳负责启动 `dsh` 服务进程，等端口就绪后把 WebView 导航到该本地地址。**原有的全部功能（模型配置、推理、会话管理等）与后端通信原样保留**，无需改动 dsh 源码。

```
┌─────────────────────────────────────────────┐
│              Tauri 桌面应用（Rust）            │
│  ┌─────────────────────────────────────────┐ │
│  │  WebView（加载 http://127.0.0.1:<port>） │ │
│  └─────────────────────────────────────────┘ │
│  原生菜单 · 系统托盘 · 快捷键桥 · 窗口状态 · 主题 │
├─────────────────────────────────────────────┤
│  子进程：node launch-dsh.js web --port <port>  │
│  （dsh 服务，HTTP + WebSocket，数据在 DSH_HOME） │
└─────────────────────────────────────────────┘
```

## 已实现的体验优化

- **原生菜单**：发送、取消、重新加载、明/暗/跟随系统主题、退出（macOS 含标准应用菜单）。
- **快捷键**：`Ctrl/Cmd + Enter` 发送、`Esc` 取消（仅当存在「停止/取消」控件时拦截，避免破坏其它 Esc 用法），均经 `src-tauri/src/bridge.rs` 注入脚本实现，`isTrusted` + 捕获阶段保证不重复触发。
- **系统托盘**：左键单击切换显隐，右键菜单「显示/隐藏/退出」；关闭窗口默认**最小化到托盘**而非退出。
- **窗口状态记忆**：`tauri-plugin-window-state` 记住位置/尺寸/最大化状态。
- **DPI 与明暗主题**：Tauri 2 自动适配 HiDPI；主题可跟随系统或手动切换并持久化。
- **单实例**：重复启动时聚焦已有窗口，避免起多个服务进程。
- **外链处理**：Web UI 里的外部链接在系统浏览器打开。

## 目录结构

```
DeepSeek_Harness_Desktop/
├── package.json              # 前端脚本 + Tauri CLI
├── vite.config.ts            # 前端构建（splash 启动页）
├── index.html / src/         # 启动过渡页（dsh 就绪前显示）
├── server/                   # dsh 服务端运行时（随应用打包）
│   ├── package.json          #   依赖 @deepseek-ai/dsh
│   └── launch-dsh.js         #   启动器：定位 dsh CLI 并转发
├── src-tauri/
│   ├── Cargo.toml            # Rust 依赖
│   ├── tauri.conf.json       # 窗口/安全/CSP/打包配置
│   ├── capabilities/         # 前端权限（最小集）
│   ├── icons/                # 应用图标（占位，可替换）
│   └── src/
│       ├── main.rs / lib.rs  # 入口 + 装配
│       ├── dsh.rs            # 服务进程生命周期
│       ├── menu.rs           # 原生菜单
│       ├── tray.rs           # 系统托盘
│       └── bridge.rs         # 快捷键桥 + 导航策略
├── scripts/gen_icons.py      # 占位图标生成（stdlib）
└── docs/                     # 详细文档
```

## 快速开始

```bash
# 1. 安装前端依赖 + Tauri CLI
npm install

# 2. 安装 server 端依赖（dsh 运行时）
npm run setup:server

# 3. 开发模式（自动启动 dsh + 热加载）
npm run tauri dev

# 4. 构建安装包（产物在 src-tauri/target/release/bundle/）
npm run tauri build
```

## 文档索引

- [环境准备（Node/Rust/WebView2/webkit2gtk）](docs/01-environment.md)
- [初始化与开发流程](docs/02-development.md)
- [构建与打包（MSI/DMG/AppImage）](docs/03-build-packaging.md)
- [安全与配置（CSP/权限/环境变量/配置文件）](docs/04-security-config.md)

## 环境变量

| 变量 | 说明 | 默认 |
|---|---|---|
| `DSH_PORT` | 强制指定 dsh 服务端口 | 3080（被占用则自动换空闲端口） |
| `DSH_NODE_BIN` | 指定 Node 可执行文件路径 | 捆绑 node（若打包）→ PATH 中的 `node` |
| `DSH_HOME` | dsh 数据目录（profiles/设置/凭据） | 应用的 app_data_dir |

> 详见 [docs/04-security-config.md](docs/04-security-config.md)。

## 说明

- 生产安装包**默认需要目标机器有 Node.js**（类似 WebView2 / webkit2gtk 的系统依赖）；如需完全免依赖的单文件分发，可把 Node 运行时作为 sidecar 一并打包（`dsh` 会优先使用捆绑的 node），见 [打包文档](docs/03-build-packaging.md) 的「高级：捆绑 Node 运行时」。
- 本项目为改造脚手架 + 完整配置，占位图标可用 `npx tauri icon src-tauri/icons/app-icon.png` 换成正式品牌图标。
