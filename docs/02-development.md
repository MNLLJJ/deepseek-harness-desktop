# 初始化与开发流程

## 1. 首次初始化

```bash
# 克隆/进入项目后
npm install            # 前端依赖 + Tauri CLI + Vite
npm run setup:server   # dsh 服务端运行时（server/node_modules/@deepseek-ai/dsh）
npm run icons          # （可选）用正式 logo 重新生成图标
```

`server/` 内的 `@deepseek-ai/dsh` 版本由 `server/package.json` 锁定，升级时改版本号后重跑 `npm run setup:server`。

## 2. 开发模式

```bash
npm run tauri dev
```

发生的事：

1. `beforeDevCommand` 启动 Vite（`http://localhost:1420`），WebView 先显示 `index.html` 启动页；
2. Rust `setup` 阶段调用 `dsh::spawn`：`node server/launch-dsh.js web --host 127.0.0.1 --port <port>`；
3. 端口探测（优先 3080，被占则自动换空闲端口）并轮询就绪；
4. 就绪后 `window.location.replace("http://127.0.0.1:<port>")` 进入 dsh UI。

### 常用开发参数

```bash
DSH_PORT=8090 npm run tauri dev     # 固定端口
DSH_NODE_BIN=/path/to/node npm run tauri dev
```

### 日志与排障

- dsh 输出写入应用日志目录下的 `dsh.log`：
  - macOS：`~/Library/Logs/com.deepseek.harness.desktop/dsh.log`
  - Linux：`~/.config/com.deepseek.harness.desktop/`（或 `$XDG_CONFIG_HOME`）下的日志目录
  - Windows：`%APPDATA%\com.deepseek.harness.desktop\logs\dsh.log`
- 单独验证 dsh 是否正常：`cd server && npx dsh web --port 3080`，浏览器打开 `http://127.0.0.1:3080`。

## 3. 改动 dsh 集成时的注意点

- **不要改动 dsh 源码**：本方案完全黑盒集成，只需保证 `server/node_modules/@deepseek-ai/dsh` 可被 `launch-dsh.js` 定位。
- **端口**：WebView 的 CSP 已放行 `http://127.0.0.1:*` 与 `ws://127.0.0.1:*`，动态端口无需改 CSP。
- **快捷键选择器**：`bridge.rs` 的 `BRIDGE_JS` 里 `sendSelectors` / `cancelSelectors` 是基于通用约定（`aria-label` / `data-testid`）的默认值。若实际 dsh UI 的发送/停止按钮 selector 不同，按 DOM 实测调整这两组选择器即可。

## 4. 目录约定

| 路径 | 作用 |
|---|---|
| `index.html`、`src/main.ts` | 启动过渡页（dsh 就绪前展示） |
| `server/` | dsh 运行时，随应用打包为 `bundle.resources` |
| `src-tauri/src/*.rs` | 桌面壳逻辑 |
| `src-tauri/capabilities/default.json` | 前端最小权限 |

## 5. 常见问题

| 现象 | 处理 |
|---|---|
| 启动页显示「未找到 @deepseek-ai/dsh」 | 运行 `npm run setup:server` |
| 提示 Node 未找到 | 确认 Node 在 PATH，或设 `DSH_NODE_BIN` |
| 端口冲突 | 设 `DSH_PORT`，或结束占用 3080 的进程 |
| 首次 `tauri dev` 编译很慢 | 正常，Rust 依赖首次需全量下载编译 |
