# 构建与打包

## 1. 构建前准备

- 确认环境已按 [01-environment.md](01-environment.md) 配好；
- 确认 `server/node_modules` 已安装（`npm run setup:server`）；
- 确认图标已生成（`npm run icons` 或保留占位图标）。

## 2. 本地构建（不打包）

```bash
npm run tauri build -- --no-bundle
# 等价于先 npm run build 再 cargo build
```

产物：`src-tauri/target/release/deepseek-harness-desktop`（或 `.exe`）。

## 3. 打包安装包

```bash
npm run tauri build
```

`tauri.conf.json` 里 `bundle.targets` 为 `"all"`，会在当前平台生成对应安装包，输出到 `src-tauri/target/release/bundle/`：

| 平台 | 产物 | 说明 |
|---|---|---|
| Windows | `*.msi` + `*.exe`（NSIS） | 需 WebView2 Runtime（系统已装则免） |
| macOS | `*.app` + `*.dmg` | 需在 macOS 上构建 |
| Linux | `*.deb` + `*.rpm` + `*.AppImage` | AppImage 为通用便携格式 |

### 指定平台产物

```bash
npm run tauri build -- --bundles msi        # 仅 MSI
npm run tauri build -- --bundles dmg        # 仅 DMG
npm run tauri build -- --bundles appimage   # 仅 AppImage
```

### macOS 签名/公证（分发到其他机器）

> ⚠️ **未签名/未公证的 .app 在其它 Mac 上打开会提示「软件已损坏，无法打开」**
> （Gatekeeper 拦截带 `com.apple.quarantine` 隔离属性的未签名应用）。这是**预期行为**，不是安装包损坏。
>
> **临时绕过（自用/内部分发）**：
> ```bash
> # 挂载 dmg 后，对应用移除隔离属性即可打开（实测可用）
> sudo xattr -dr com.apple.quarantine "/Applications/DeepSeek Harness.app"
> # 或先把应用拖入 /Applications，再执行上面的命令
> ```

**正式分发请配置 Developer ID 签名 + 公证**，这样用户在其它 Mac 上双击即可打开：

1. 在仓库 Settings → Secrets and variables → Actions 配置：
   | Secret | 说明 |
   |---|---|
   | `APPLE_CERTIFICATE` | Developer ID Application 证书导出的 .p12 的 base64 |
   | `APPLE_CERTIFICATE_PASSWORD` | .p12 密码 |
   | `APPLE_SIGNING_IDENTITY` | 证书名，如 `Developer ID Application: Your Name (TEAMID)` |
   | `APPLE_ID` / `APPLE_PASSWORD` | Apple ID + app 专用密码（用于公证） |
   | `APPLE_TEAM_ID` | 开发者 Team ID |

2. 推送 `v*` tag 构建时，CI 的 macOS job 检测到 secrets 后会自动签名 + 公证（未配置则回退为 ad-hoc 签名，见下方说明）。

> 说明：`tauri.conf.json` 已设 `bundle.macOS.signingIdentity: "-"`，即默认对 .app 做 **ad-hoc 签名**（修复 Gatekeeper 对无签名 bundle 的额外校验问题）；配置 `APPLE_SIGNING_IDENTITY` secret 后 CI 会用 Developer ID 覆盖它。

### Windows 代码签名

```bash
export TAURI_SIGNING_PRIVATE_KEY="..."   # 或使用 --ci 时的私钥
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="..."
npm run tauri build
```

## 4. 关于 Node.js 运行时

打包产物**默认仍依赖目标机器安装 Node.js**（`dsh::resolve_node` 依次查找 `DSH_NODE_BIN` → 捆绑 node → PATH）。这是刻意设计：`dsh` 的插件是运行时从 `node_modules` 加载的 JS，无法轻易编译成单文件。

### 高级：捆绑 Node 运行时（免安装分发）

1. 从 [nodejs.org](https://nodejs.org/) 下载目标平台的预编译 node 二进制；
2. 放入 `src-tauri/binaries/`（Windows 用 `node.exe`）；
3. 在 `tauri.conf.json` 增加 sidecar 声明：

```json
{
  "bundle": {
    "externalBin": ["binaries/node"]
  }
}
```

4. 无需改动代码：`dsh.rs::resolve_node` 已内置该查找逻辑，运行时按 `DSH_NODE_BIN` → `<resource_dir>/binaries/node`（Windows 为 `node.exe`）→ PATH 的顺序解析。Tauri 会把 externalBin 解压到资源目录的 `binaries/` 下（去掉 target triple 后缀），并与 `app.path().resource_dir()` 拼接。

> 注意：`bundle.resources` 已包含整个 `server/`（含 `node_modules`），体积较大（数百 MB）。如需精简，可只在安装包内保留 dsh 运行时必需包，或用 `server` 的 `--omit=dev` 安装。

## 5. 产物验证

```bash
# macOS
open "src-tauri/target/release/bundle/macos/DeepSeek Harness.app"

# Linux
./src-tauri/target/release/bundle/appimage/*.AppImage

# Windows（PowerShell）
.\src-tauri\target\release\bundle\msi\*.msi
```

验证点：启动 → 自动起 dsh → 进入 UI → 模型配置/会话/推理正常 → 菜单/托盘/快捷键/主题生效 → 关闭窗口最小化到托盘 → 托盘退出真正结束进程。
