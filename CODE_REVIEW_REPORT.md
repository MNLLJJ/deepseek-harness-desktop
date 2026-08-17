# DeepSeek Harness Desktop — 代码审查与修复报告

> 审查日期：2026-08-16 · 审查范围：全部源码（Rust 后端 5 文件 + 前端 + server 启动器 + 脚本 + 配置）

---

## 1️⃣ 项目概览与架构

| 项 | 内容 |
|---|---|
| 技术栈 | Tauri 2（Rust） + Vite + TypeScript + Node.js（dsh 服务 sidecar） |
| 架构模式 | **sidecar 模式**：桌面壳启动 `node server/launch-dsh.js web --host 127.0.0.1 --port <port>`，WebView 等端口就绪后 `location.replace` 到本地 dsh UI |
| 数据流 | 壳 → dsh（HTTP + WebSocket）→ DeepSeek 推理；DSH_HOME 指向应用数据目录做隔离 |
| 核心模块 | `dsh.rs`（进程生命周期）、`bridge.rs`（快捷键桥/导航策略）、`menu.rs`（原生菜单）、`tray.rs`（系统托盘）、`lib.rs`（装配） |

```
Tauri 壳（Rust） ──spawn──▶ node launch-dsh.js ──spawn──▶ dsh (bin.js, HTTP+WS :3080)
     │                                                           ▲
     └──WebView── eval BRIDGE_JS ── window.location.replace ─────┘
```

## 2️⃣ 审查范围（逐文件清单）

| 模块 | 文件 | 行数 | 审查结论 |
|---|---|---|---|
| Rust 后端 | `src-tauri/src/lib.rs` | 175 | 🔴 3 处修复 |
| Rust 后端 | `src-tauri/src/dsh.rs` | 161 | 🔴 3 处修复 |
| Rust 后端 | `src-tauri/src/bridge.rs` | 121 | 🟡 1 处修复 |
| Rust 后端 | `src-tauri/src/menu.rs` | 90 | 🟡 1 处修复 |
| Rust 后端 | `src-tauri/src/tray.rs` | 62 | 🟡 1 处修复 |
| 前端 | `index.html` / `src/main.ts` | 114 / 34 | 🟢 未发现问题 |
| Server | `server/launch-dsh.js` | 71 | 🔴 2 处修复 |
| Server | `server/package.json` | 9 | 🟢 未发现问题 |
| 脚本 | `scripts/gen_icons.py` | 129 | 🟢 未发现问题 |
| 配置 | `tauri.conf.json` / `Cargo.toml` / `capabilities/default.json` / `vite.config.ts` / `tsconfig.json` / `package.json` | — | 🟢 未发现问题 |

## 3️⃣ 修复明细

### 🔴 P0 — 功能/资源泄漏/进程管理（4 项）

| # | 文件:行号（修复前） | 问题描述 | 修复方案 |
|---|---|---|---|
| 1 | `launch-dsh.js:56-59` | **launcher 信号转发死锁**：`child.on('exit')` 里 `process.kill(process.pid, signal)` 会再次触发已注册的 SIGTERM/SIGINT handler；此时 `child.killed` 为 false（子进程非被本进程 kill），handler 再次 `child.kill()` 对已退出进程无效，信号被消费后 **launcher 进程永不退出**，上层 `wait_ready` 只能等到 60 秒超时 | 转发前先 `removeAllListeners(sig)`，再用默认信号行为终止自身 |
| 2 | `launch-dsh.js:44-47` | **缺 `child.on('error')`**：spawn 失败（如权限不足）时无任何输出，进程静默挂起，用户无法定位原因 | 增加 error 监听：打印错误并 `process.exit(1)` |
| 3 | `lib.rs:148-149` + `dsh.rs`（无 stop） | **子进程无法优雅退出、dsh 成为孤儿**：`Child::kill()` 在 Unix 发 SIGKILL，launch-dsh.js 的 SIGTERM 转发逻辑无法执行，真正的 dsh 进程残留并持续占用端口（下次启动端口冲突） | `DshServer` 新增 `stop()`：先 `kill -TERM`（级联转发），最多等 3 秒，再 SIGKILL 兜底 |
| 4 | `lib.rs:122-139` + `dsh.rs:99-108` | **就绪轮询不感知进程死亡**：若 dsh 因端口被占/崩溃提前退出，`wait_ready` 仍盲目轮询，且可能**误连上占用同一端口的其它服务**并跳转过去 | 新增 `wait_ready_while_alive()`：每轮轮询先 `child.try_wait()`，进程已退出立即判定失败 |

### 🟡 P1 — 健壮性/边界条件（5 项）

| # | 文件:行号（修复前） | 问题描述 | 修复方案 |
|---|---|---|---|
| 5 | `menu.rs:76-81` | **主题偏好静默丢失**：`theme.txt` 写入前未 `create_dir_all`，首次运行时 `app_config_dir` 不存在，`std::fs::write` 失败被 `let _` 吞掉，切换主题重启后不生效 | 写入前先 `create_dir_all(&dir)` |
| 6 | `dsh.rs:18-25` | **`DSH_PORT=0` 边界**：parse 成功即返回 0，而服务端会监听随机端口，调用方无法预知，`wait_ready` 永远连不上 → 60 秒假超时 | `n != 0` 才采用，0 回退到自动选端口 |
| 7 | `bridge.rs:103-120` | **导航白名单放行未知 scheme**：`data:`/`file:`/`about:` 等非 http(s) scheme 一律 `return true`，可绕过 CSP 加载意外内容 | 默认返回 `false`，仅放行 `tauri` 与白名单内 http(s) |
| 8 | `lib.rs:89-97` | **路径解析失败静默降级为空路径**：`app_data_dir()`/`app_log_dir()` 失败时 `unwrap_or_default()` 得到空串 → `DSH_HOME=""`（dsh 数据落到默认位置，破坏数据隔离承诺）、日志写入 CWD | 改为 `unwrap_or_else` 回退当前目录并 `eprintln!` 告警 |
| 9 | `tray.rs:17` | **图标缺失直接 panic**：`default_window_icon().unwrap()`，若打包配置缺图标则应用启动崩溃 | `let Some(icon) = ... else { 跳过托盘并告警 }` |

## 4️⃣ 验证结果

| 验证项 | 命令 | 结果 |
|---|---|---|
| Rust 编译 | `cargo check`（src-tauri） | ✅ 15.4s 通过，无警告无错误 |
| 前端类型检查 | `tsc --noEmit` | ✅ 通过 |
| JS 语法 | `node --check server/launch-dsh.js` | ✅ 通过 |

## 5️⃣ 观察项（未改动，仅记录）

| 文件 | 说明 |
|---|---|
| `bridge.rs` BRIDGE_JS `fireKey` | `KeyboardEvent` 构造器不识别 `keyCode`/`which`（会静默忽略），仅影响「找不到发送按钮」时的兜底路径；现代 UI 均按 `event.key` 判断，风险低，未改动 |
| `dsh.rs` `resolve_node` | ~~未实现资源目录内捆绑 node 的查找~~（2026-08-17 已补齐：现在按 `DSH_NODE_BIN` → `<resource_dir>/binaries/node` → PATH 解析，`spawn` 同步接收 `AppHandle`） |

> **2026-08-17 更新**：上一轮报告中「观察项」提及的 `resolve_node` 未实现捆绑 node 查找，现已补上实现（见上表），并同步更新了 `README.md`、`docs/03-build-packaging.md`、`docs/04-security-config.md` 中的相关说明。
| `Cargo.toml` | `log = "0.4"` 为未使用依赖（代码用 `eprintln!`），不影响运行，未删除 |
| `main.ts:26` | 超时文案判断基于「是否含『失败』字样」，dsh 超时时文案会被二次覆盖为更明确的提示，无害 |

## 6️⃣ 修改文件汇总

| 文件 | 改动要点 |
|---|---|
| `server/launch-dsh.js` | 修复信号转发死锁（P0#1）+ 增加 error 处理（P0#2） |
| `src-tauri/src/dsh.rs` | 新增 `DshServer::stop()` 优雅终止（P0#3）、`wait_ready_while_alive()`（P0#4）、`DSH_PORT=0` 过滤（P1#6） |
| `src-tauri/src/lib.rs` | 接入进程存活感知的就绪等待（P0#4）、路径回退告警（P1#8）、`stop_server` 用优雅终止（P0#3）、锁 poison 容忍 |
| `src-tauri/src/menu.rs` | 主题写入前建目录（P1#5） |
| `src-tauri/src/bridge.rs` | 导航白名单默认拒绝未知 scheme（P1#7） |
| `src-tauri/src/tray.rs` | 图标缺失不 panic（P1#9） |
