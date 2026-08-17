//! 系统托盘：左键单击切换显隐，右键菜单显示/隐藏/退出。

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

use crate::menu;

pub fn setup(app: &mut App) -> tauri::Result<()> {
    // 图标缺失时跳过托盘而不是 panic，保证应用其余功能可用
    let Some(icon) = app.default_window_icon().cloned() else {
        eprintln!("[tray] 缺少默认窗口图标，跳过托盘创建");
        return Ok(());
    };

    let show = MenuItem::with_id(app, "tray_show", "显示窗口", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "tray_hide", "隐藏到托盘", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "tray_quit", "退出", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&show, &hide, &sep, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("DeepSeek Harness")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray_show" => show_window(app),
            "tray_hide" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            "tray_quit" => menu::quit_app(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        show_window(app);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
