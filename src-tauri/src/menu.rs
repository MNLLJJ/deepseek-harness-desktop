//! 原生菜单：发送/取消动作、主题切换、重载、退出。

use std::sync::atomic::Ordering;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{App, AppHandle, Manager};

use crate::bridge;
use crate::AppState;

pub fn setup(app: &mut App) -> tauri::Result<()> {
    let send = MenuItem::with_id(app, "send", "发送消息", true, None::<&str>)?;
    let cancel = MenuItem::with_id(app, "cancel", "取消当前任务", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "重新加载", true, Some("CmdOrCtrl+R"))?;
    let theme_dark = MenuItem::with_id(app, "theme_dark", "深色主题", true, None::<&str>)?;
    let theme_light = MenuItem::with_id(app, "theme_light", "浅色主题", true, None::<&str>)?;
    let theme_system = MenuItem::with_id(app, "theme_system", "跟随系统", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, Some("CmdOrCtrl+Q"))?;

    let file_menu = Submenu::with_items(app, "文件", true, &[&reload, &quit])?;
    let action_menu = Submenu::with_items(app, "操作", true, &[&send, &cancel])?;
    let view_menu = Submenu::with_items(
        app,
        "视图",
        true,
        &[&theme_dark, &theme_light, &theme_system],
    )?;

    #[cfg(target_os = "macos")]
    let menu = {
        let about = PredefinedMenuItem::about(app, Some("关于 DeepSeek Harness"), None)?;
        let sep = PredefinedMenuItem::separator(app)?;
        let services = PredefinedMenuItem::services(app, None::<&str>)?;
        let hide = PredefinedMenuItem::hide(app, None::<&str>)?;
        let hide_others = PredefinedMenuItem::hide_others(app, None::<&str>)?;
        let show_all = PredefinedMenuItem::show_all(app, None::<&str>)?;
        let app_submenu = Submenu::with_items(
            app,
            "DeepSeek Harness",
            true,
            &[&about, &sep, &services, &sep, &hide, &hide_others, &show_all, &sep, &quit],
        )?;
        Menu::with_items(app, &[&app_submenu, &file_menu, &action_menu, &view_menu])?
    };

    #[cfg(not(target_os = "macos"))]
    let menu = Menu::with_items(app, &[&file_menu, &action_menu, &view_menu])?;

    app.set_menu(menu)?;

    app.on_menu_event(|app, event| match event.id().as_ref() {
        "send" => bridge::emit(app, "send"),
        "cancel" => bridge::emit(app, "cancel"),
        "reload" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval("window.location.reload()");
            }
        }
        "theme_dark" => set_theme(app, Some(tauri::Theme::Dark)),
        "theme_light" => set_theme(app, Some(tauri::Theme::Light)),
        "theme_system" => set_theme(app, None),
        "quit" => quit_app(app),
        _ => {}
    });

    Ok(())
}

fn set_theme(app: &AppHandle, theme: Option<tauri::Theme>) {
    let _ = app.set_theme(theme);
    let pref = match theme {
        Some(tauri::Theme::Dark) => "dark",
        Some(tauri::Theme::Light) => "light",
        _ => "system",
    };
    let dir = app.path().app_config_dir().unwrap_or_default();
    // 首次运行时配置目录可能不存在，先创建再写入，否则主题偏好无法持久化
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("theme.txt"), pref);
}

/// 真正的退出：置 quitting 标志后再退出，绕过「最小化到托盘」拦截。
pub fn quit_app(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.quitting.store(true, Ordering::SeqCst);
    app.exit(0);
}
