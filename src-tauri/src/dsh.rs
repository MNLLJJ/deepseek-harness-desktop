//! dsh 服务进程生命周期管理：端口探测、进程启动、就绪等待。

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::Manager;

/// 运行中的 dsh 服务。
pub struct DshServer {
    pub child: Child,
    pub port: u16,
}

impl DshServer {
    /// 优雅终止：先 SIGTERM（launch-dsh.js 会级联转发给真正的 dsh 进程），
    /// 等待其自行退出；超时后兜底 SIGKILL / TerminateProcess，避免孤儿进程残留并占用端口。
    pub fn stop(&mut self) {
        #[cfg(unix)]
        {
            let pid = self.child.id() as i32;
            // 通过系统 kill 发送 SIGTERM，让 launch-dsh.js 的转发逻辑有机会执行
            let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                match self.child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(_) => break,
                }
            }
        }
        // 兜底：SIGKILL（Unix）/ TerminateProcess（Windows）
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// 解析目标端口：优先 `DSH_PORT`，否则尝试默认 3080，被占用则用系统分配的空闲端口。
pub fn resolve_port() -> u16 {
    if let Ok(p) = std::env::var("DSH_PORT") {
        if let Ok(n) = p.trim().parse::<u16>() {
            // 端口 0 无意义（服务端会监听随机端口，而调用方无法预知），直接回退到自动选择
            if n != 0 {
                return n;
            }
        }
    }
    find_free_port(3080)
}

/// 先尝试 preferred，占用则返回系统分配的空闲端口。
pub fn find_free_port(preferred: u16) -> u16 {
    if TcpListener::bind(("127.0.0.1", preferred)).is_ok() {
        return preferred;
    }
    TcpListener::bind(("127.0.0.1", 0))
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| a.port())
        .unwrap_or(preferred)
}

/// 资源目录内的候选根目录：Tauri 2 把 bundle.resources 放在 `<resource_dir>/_up_/` 下
/// （updater 兼容布局，v2.1+ 默认），旧布局直接放在 `<resource_dir>/` 下，两种都探测。
fn resource_bases(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        bases.push(res.join("_up_"));
        bases.push(res);
    }
    bases
}

/// 常见 Node 安装位置（GUI 应用从 launchd 启动时 PATH 不含用户/包管理器目录）。
fn common_node_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/usr/local/bin/node")); // Intel Homebrew
        paths.push(PathBuf::from("/opt/homebrew/bin/node")); // Apple Silicon Homebrew
        if let Ok(home) = std::env::var("HOME") {
            // nvm 多版本，取版本号最高的一个
            let nvm = PathBuf::from(&home).join(".nvm/versions/node");
            if let Ok(entries) = std::fs::read_dir(&nvm) {
                let mut vers: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect();
                vers.sort();
                if let Some(last) = vers.last() {
                    paths.push(last.join("bin/node"));
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        paths.push(PathBuf::from(r"C:\Program Files\nodejs\node.exe"));
        paths.push(PathBuf::from(r"C:\Program Files (x86)\nodejs\node.exe"));
    }
    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/usr/local/bin/node"));
    }
    paths
}

/// 解析 Node 可执行文件：`DSH_NODE_BIN` → 资源目录内捆绑的 node → PATH → 常见安装位置。
pub fn resolve_node(app: &tauri::AppHandle) -> String {
    // 1) 显式指定
    if let Ok(p) = std::env::var("DSH_NODE_BIN") {
        if !p.trim().is_empty() {
            return p.trim().to_string();
        }
    }

    // 2) 资源目录内捆绑的 node（externalBin 解压位置，见 docs/03-build-packaging.md）
    #[cfg(target_os = "windows")]
    let node_name = "node.exe";
    #[cfg(not(target_os = "windows"))]
    let node_name = "node";
    for base in resource_bases(app) {
        for cand in [base.join("binaries").join(node_name), base.join(node_name)] {
            // 同时校验存在性与可执行性，避免选中无执行权限的占位文件
            if cand.is_file() && Command::new(&cand).arg("--version").output().is_ok() {
                return cand.to_string_lossy().into_owned();
            }
        }
    }

    // 3) PATH
    for cand in [node_name, "node", "nodejs"] {
        if Command::new(cand).arg("--version").output().is_ok() {
            return cand.to_string();
        }
    }

    // 4) 常见安装位置（覆盖 GUI 环境 PATH 不含 node 的情况）
    for cand in common_node_paths() {
        if cand.is_file() && Command::new(&cand).arg("--version").output().is_ok() {
            return cand.to_string_lossy().into_owned();
        }
    }
    node_name.to_string()
}

/// 定位启动器脚本：打包后取资源目录（兼容 `_up_` 布局），开发期取源码目录。
pub fn launcher_path(app: &tauri::AppHandle) -> PathBuf {
    for base in resource_bases(app) {
        let p = base.join("server").join("launch-dsh.js");
        if p.exists() {
            return p;
        }
    }
    // dev 回退：<repo>/server/launch-dsh.js
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("server")
        .join("launch-dsh.js")
}

/// 启动 dsh：`node launch-dsh.js web --host 127.0.0.1 --port <port>`，stdout/stderr 写入日志。
pub fn spawn(
    app: &tauri::AppHandle,
    launcher: &Path,
    port: u16,
    home: &Path,
    log_file: &Path,
) -> std::io::Result<DshServer> {
    let node = resolve_node(app);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    let err = log.try_clone()?;

    let child = Command::new(&node)
        .arg(launcher)
        .arg("web")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .env("DSH_HOME", home)
        .env("NO_COLOR", "1")
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err))
        .spawn()?;

    Ok(DshServer { child, port })
}

/// 轮询端口直到可连接或超时，同时监控 dsh 进程存活：
/// 若子进程提前退出（启动失败/崩溃/端口被占），立即判定失败，
/// 避免 wait_ready 误连上占用同一端口的其它服务。
pub fn wait_ready_while_alive(
    port: u16,
    timeout: Duration,
    server: &Mutex<Option<DshServer>>,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        {
            let mut guard = server.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(s) = guard.as_mut() {
                match s.child.try_wait() {
                    Ok(Some(_)) => return false,
                    Ok(None) => {}
                    Err(_) => return false,
                }
            }
        }
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

/// 辅助：向日志写入一行（用于调试）。
#[allow(dead_code)]
pub fn log_line(log_file: &Path, msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
        let _ = writeln!(f, "{msg}");
    }
}
