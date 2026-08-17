mod bridge;
mod dsh;
mod menu;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

/// 应用级共享状态。
pub struct AppState {
    /// 正在运行的 dsh 服务进程（退出时负责清理）。
    pub dsh: Mutex<Option<dsh::DshServer>>,
    /// 是否真正退出（区分「关闭窗口→最小化到托盘」与「退出应用」）。
    pub quitting: AtomicBool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            dsh: Mutex::new(None),
            quitting: AtomicBool::new(false),
        })
        // 单实例：重复启动时聚焦已有窗口，避免起多个 dsh 服务。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        // 窗口位置/尺寸/最大化状态记忆。
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // 外部链接在系统浏览器打开。
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 恢复上次主题偏好
            restore_theme(app)?;
            // 原生菜单（发送/取消/主题/重载/退出）
            menu::setup(app)?;
            // 系统托盘（显示/隐藏/退出）
            tray::setup(app)?;
            // 取出 AppHandle 供导航策略闭包持有（'static）
            let app_handle = app.handle().clone();
            // 创建主窗口（带页面加载桥接 + 导航策略）
            let _main_window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("DeepSeek Harness")
            .inner_size(1280.0, 820.0)
            .min_inner_size(960.0, 600.0)
            .center()
            .resizable(true)
            .on_page_load(|webview, _| {
                let _ = webview.eval(bridge::BRIDGE_JS);
            })
            .on_navigation(move |url| bridge::handle_navigation(&app_handle, url))
            .build()?;
            // 启动 dsh 服务
            spawn_server(app);
            Ok(())
        })
        // 启动 dsh 服务（主窗口在 setup 中创建）
        // 关闭窗口 → 最小化到托盘（除非显式退出）。
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if !state.quitting.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building the DeepSeek Harness desktop app")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                stop_server(app_handle);
            }
        });
}

/// 启动 dsh 服务并等待就绪后把 WebView 导航过去。
fn spawn_server(app: &tauri::App) {
    let port = dsh::resolve_port();
    let home = app.path().app_data_dir().unwrap_or_else(|e| {
        eprintln!("[dsh] 无法获取应用数据目录（{e}），回退到当前目录作为 DSH_HOME");
        std::path::PathBuf::from(".")
    });
    let log_dir = app.path().app_log_dir().unwrap_or_else(|e| {
        eprintln!("[dsh] 无法获取日志目录（{e}），回退到当前目录");
        std::path::PathBuf::from(".")
    });
    let log_file = log_dir.join("dsh.log");
    if let Some(parent) = log_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let launcher = dsh::launcher_path(app.handle());

    let server = match dsh::spawn(app.handle(), &launcher, port, &home, &log_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[dsh] 启动服务失败: {e}（请确认 Node.js 已安装，或设置 DSH_NODE_BIN）");
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval(
                    "window.dispatchEvent(new CustomEvent('dsh-status', { detail: { phase: 'error', message: '启动服务失败：未找到 Node.js，请安装或设置 DSH_NODE_BIN' } }));",
                );
            }
            return;
        }
    };

    {
        let state = app.state::<AppState>();
        *state.dsh.lock().unwrap_or_else(|e| e.into_inner()) = Some(server);
    }

    let url = format!("http://127.0.0.1:{port}");
    let window = app.get_webview_window("main");
    let handle = app.handle().clone();

    std::thread::spawn(move || {
        let state = handle.state::<AppState>();
        if dsh::wait_ready_while_alive(port, std::time::Duration::from_secs(60), &state.dsh) {
            if let Some(w) = window {
                let js = format!(
                    "window.dispatchEvent(new CustomEvent('dsh-status', {{ detail: {{ phase: 'ready', message: '服务已就绪' }} }})); window.location.replace('{url}');"
                );
                let _ = w.eval(&js);
            }
            let _ = handle.emit("dsh-ready", port);
        } else {
            eprintln!("[dsh] 服务在超时时间内未就绪或进程已退出");
            if let Some(w) = window {
                let _ = w.eval(
                    "window.dispatchEvent(new CustomEvent('dsh-status', { detail: { phase: 'error', message: '服务启动超时' } }));",
                );
            }
        }
    });
}

/// 退出时优雅终止 dsh 子进程。
fn stop_server(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    // 先把 Option 取出，MutexGuard 临时量立即释放
    let server_opt = state
        .dsh
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    if let Some(mut server) = server_opt {
        server.stop();
    }
}

/// 读取主题偏好文件并应用。
fn restore_theme(app: &tauri::App) -> tauri::Result<()> {
    let path = app
        .path()
        .app_config_dir()
        .unwrap_or_default()
        .join("theme.txt");
    let theme = match std::fs::read_to_string(&path).ok().map(|s| s.trim().to_string()) {
        Some(s) if s == "dark" => Some(tauri::Theme::Dark),
        Some(s) if s == "light" => Some(tauri::Theme::Light),
        _ => None,
    };
    app.set_theme(theme);
    Ok(())
}
