# 安全与配置

## 1. 安全模型总览

本方案 WebView 加载的是 `http://127.0.0.1:<port>` 这一**本地受信任服务**，而非任意公网地址。安全策略围绕两点：**只放行本地来源** + **前端最小权限**。

| 层 | 机制 | 说明 |
|---|---|---|
| 内容来源 | `on_navigation` | 仅放行 `127.0.0.1`/`localhost`/`ipc.localhost`，外链交系统浏览器 |
| 脚本注入 | `initialization_script`（`bridge.rs`） | 快捷键桥，仅操作 DOM，不暴露 Tauri IPC |
| 前端权限 | `capabilities/default.json` | 最小集：`core:default` + `opener` + `window-state` |
| 资源隔离 | `bundle.resources` | dsh 运行时以资源形式内置，路径只读 |
| 数据隔离 | `DSH_HOME` | 指向应用专属数据目录，与全局 `~/.dsh` 隔离 |

## 2. CSP（内容安全策略）

配置在 `src-tauri/tauri.conf.json → app.security.csp`：

```json
"csp": "default-src 'self' http://127.0.0.1:* http://localhost:*; script-src 'self' http://127.0.0.1:* http://localhost:*; style-src 'self' 'unsafe-inline' http://127.0.0.1:* http://localhost:*; img-src 'self' data: blob: http://127.0.0.1:* http://localhost:* https:; font-src 'self' data: http://127.0.0.1:* http://localhost:*; connect-src 'self' http://127.0.0.1:* http://localhost:* ws://127.0.0.1:* ws://localhost:* ipc: http://ipc.localhost; worker-src 'self' blob: http://127.0.0.1:* http://localhost:*"
```

要点：

- **动态端口**：用 `http://127.0.0.1:*` 的端口通配，端口变化无需改 CSP；
- **WebSocket**：`connect-src` 显式放行 `ws://127.0.0.1:*`；
- **Tauri IPC**：保留 `ipc: http://ipc.localhost`（Tauri 内部注入脚本需要）；
- `style-src 'unsafe-inline'`：为兼容 dsh 前端可能的内联样式；
- 若 dsh 前端使用了 WASM / `eval`，按需在 `script-src` 追加 `'wasm-unsafe-eval'` / `'unsafe-eval'`。

> 调试期如被 CSP 阻断，可临时 `"csp": null` 定位缺失的来源，定位后写回精确策略——不要在生产保留 `null`。

## 3. Capabilities（权限）

`src-tauri/capabilities/default.json`：

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "window-state:default"
  ]
}
```

- 快捷键、菜单、托盘全部在 **Rust 侧**实现，前端不需要任何 IPC 命令权限；
- 新增前端能力时应显式声明，遵循最小权限原则；
- 权限明细由 `npm run tauri dev` 后生成的 `src-tauri/gen/schemas/` 提供（`tauri permission` 命令可增删）。

## 4. 环境变量

| 变量 | 用途 | 默认 |
|---|---|---|
| `DSH_PORT` | 固定 dsh 端口 | 3080，占用则自动换 |
| `DSH_NODE_BIN` | 指定 Node 路径 | 捆绑 node（若打包）→ PATH 中 `node` |
| `DSH_HOME` | dsh 数据目录 | 应用 app_data_dir（应用内部设置） |

说明：`DSH_HOME` 由 Rust 代码强制设为应用数据目录，用于隔离 profiles / 设置 / 凭据，用户一般不覆盖。

## 5. 关键配置文件

| 文件 | 作用 | 关键字段 |
|---|---|---|
| `src-tauri/tauri.conf.json` | 窗口、CSP、打包 | `app.windows`、`app.security.csp`、`bundle.*` |
| `src-tauri/Cargo.toml` | Rust 依赖 | `tauri`、三个插件、`serde` |
| `src-tauri/capabilities/default.json` | 前端权限 | `permissions` |
| `package.json` | 前端脚本 | `scripts.tauri`、`setup:server` |
| `server/package.json` | dsh 版本锁定 | `@deepseek-ai/dsh` |
| `vite.config.ts` | 前端构建/端口 | `server.port=1420` |

## 6. 安全加固建议

1. **不对外暴露服务**：spawn 时显式 `--host 127.0.0.1`（dsh 官方也不支持 `0.0.0.0`），杜绝局域网访问；
2. **禁用远程内容**：`on_navigation` 阻止非本地 URL 在 WebView 内打开；
3. **生产移除 `unsafe-eval`**：确认 dsh 前端不需要后收紧 CSP；
4. **升级策略**：dsh 处于快速迭代期（`0.1.0-rc.x`），升级时重跑 `npm run setup:server` 并回归快捷键/通信。
