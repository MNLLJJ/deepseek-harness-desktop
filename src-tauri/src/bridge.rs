//! 快捷键/菜单/托盘动作的桥接层：把动作注入到 WebView 内触发 dsh 的 UI。

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

/// 注入到每个页面的桥接脚本（幂等，捕获阶段，仅处理真实按键事件）。
///
/// 设计要点：
/// - `window.__DSH_BRIDGE__` 防重复注入；
/// - `dsh-shortcut` 自定义事件由原生菜单/托盘经 `webview.eval` 派发；
/// - Ctrl/Cmd+Enter → 发送（点击发送按钮，找不到则派发 Enter 键事件）；
/// - Esc → 仅在存在「取消/停止」控件时拦截并点击，避免破坏 UI 里其它 Esc 用法；
/// - 捕获阶段 + isTrusted 守卫 + preventDefault/stopPropagation，保证只触发一次。
pub const BRIDGE_JS: &str = r#"
(function () {
  if (window.__DSH_BRIDGE__) { return; }
  window.__DSH_BRIDGE__ = true;

  var CFG = {
    sendSelectors: [
      'button[aria-label="send"]',
      'button[aria-label*="send" i]',
      'button[aria-label*="发送" i]',
      'button[data-testid="send-button"]',
      'button[data-testid="send"]',
      'button[type="submit"]'
    ],
    cancelSelectors: [
      'button[aria-label="stop"]',
      'button[aria-label="cancel"]',
      'button[aria-label*="stop" i]',
      'button[aria-label*="停止" i]',
      'button[data-testid="stop-button"]',
      'button[data-testid="stop"]'
    ]
  };

  function firstMatch(list) {
    for (var i = 0; i < list.length; i++) {
      try { var el = document.querySelector(list[i]); if (el) { return el; } } catch (e) {}
    }
    return null;
  }

  function fireKey(el, type, opts) {
    if (!el || typeof el.dispatchEvent !== 'function') { return; }
    var init = Object.assign({ bubbles: true, cancelable: true }, opts || {});
    el.dispatchEvent(new KeyboardEvent(type, init));
  }

  function doSend() {
    var btn = firstMatch(CFG.sendSelectors);
    if (btn) { btn.click(); return; }
    var active = document.activeElement;
    fireKey(active, 'keydown', { key: 'Enter', code: 'Enter', keyCode: 13, which: 13 });
    fireKey(active, 'keyup', { key: 'Enter', code: 'Enter', keyCode: 13, which: 13 });
  }

  function doCancel() {
    var btn = firstMatch(CFG.cancelSelectors);
    if (btn) { btn.click(); return; }
    var active = document.activeElement;
    fireKey(active, 'keydown', { key: 'Escape', code: 'Escape', keyCode: 27, which: 27 });
  }

  // 原生菜单 / 托盘动作
  window.addEventListener('dsh-shortcut', function (e) {
    var a = e && e.detail && e.detail.action;
    if (a === 'send') { doSend(); }
    else if (a === 'cancel') { doCancel(); }
  });

  // 页内快捷键（捕获阶段，仅真实按键）
  document.addEventListener('keydown', function (e) {
    if (!e.isTrusted) { return; }
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();
      doSend();
    } else if (e.key === 'Escape') {
      if (firstMatch(CFG.cancelSelectors)) {
        e.preventDefault();
        e.stopPropagation();
        doCancel();
      }
    }
  }, true);
})();
"#;

/// 派发动作到主窗口（send / cancel）。
pub fn emit(app: &AppHandle, action: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let js = format!(
            "window.dispatchEvent(new CustomEvent('dsh-shortcut', {{ detail: {{ action: '{}' }} }}));",
            action
        );
        let _ = w.eval(&js);
    }
}

/// 导航策略：放行本地服务与 splash，外部链接交给系统浏览器，未知 scheme 一律拒绝。
pub fn handle_navigation(app: &tauri::AppHandle, url: &tauri::Url) -> bool {
    let scheme = url.scheme();
    if scheme == "tauri" {
        return true;
    }
    if scheme == "http" || scheme == "https" {
        let host = url.host_str().unwrap_or("");
        if host == "127.0.0.1" || host == "localhost" || host == "ipc.localhost" {
            return true;
        }
        // 外链：系统浏览器打开，WebView 内阻止
        let _ = app
            .opener()
            .open_url(url.as_str(), None::<&str>);
        return false;
    }
    // 其它 scheme（data:/file:/about: 等）不在白名单内，默认拒绝，避免绕过 CSP 的意外加载
    false
}
