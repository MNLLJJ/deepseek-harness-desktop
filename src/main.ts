// 启动过渡页逻辑：Rust 侧就绪后会用 location.replace 跳到 dsh 服务地址。
// 这里只负责：显示加载状态、超时提示。
const statusEl = document.getElementById("status") as HTMLElement | null;
const spinnerEl = document.getElementById("spinner") as HTMLElement | null;
const hintEl = document.getElementById("hint") as HTMLElement | null;

const STATUS_READY_TIMEOUT_MS = 90_000;

// Rust 侧通过 webview.eval 派发该事件更新状态
window.addEventListener("dsh-status", (e) => {
  const detail = (e as CustomEvent).detail;
  if (!detail || !statusEl) return;
  if (detail.phase === "starting") {
    statusEl.textContent = detail.message ?? "正在启动本地服务…";
  } else if (detail.phase === "ready") {
    statusEl.textContent = detail.message ?? "服务已就绪，正在进入…";
  } else if (detail.phase === "error") {
    statusEl.textContent = detail.message ?? "启动失败";
    if (spinnerEl) spinnerEl.style.display = "none";
    if (hintEl) hintEl.style.display = "block";
  }
});

// 兜底超时：若 Rust 迟迟未导航，展示排查提示
setTimeout(() => {
  if (statusEl && statusEl.textContent && !statusEl.textContent.includes("失败")) {
    statusEl.textContent = "启动超时，请查看下方排查提示";
    if (spinnerEl) spinnerEl.style.display = "none";
    if (hintEl) hintEl.style.display = "block";
  }
}, STATUS_READY_TIMEOUT_MS);

export {};
