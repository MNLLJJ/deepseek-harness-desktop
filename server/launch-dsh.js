#!/usr/bin/env node
/**
 * DeepSeek Harness 服务端启动器。
 *
 * Tauri 侧以 `node launch-dsh.js web --host 127.0.0.1 --port <port>` 启动本脚本，
 * 本脚本定位 `@deepseek-ai/dsh` 的 CLI 入口 (lib/bin.js) 并转发参数与 stdio，
 * 使 dsh 的输出能够被上层捕获并写入日志。
 */
'use strict';

const path = require('node:path');
const fs = require('node:fs');
const { spawn } = require('node:child_process');

function resolveDshBin() {
  // 1) 优先取相对于本脚本的本地安装（server/node_modules/@deepseek-ai/dsh）
  const local = path.join(
    __dirname,
    'node_modules',
    '@deepseek-ai',
    'dsh',
    'lib',
    'bin.js',
  );
  if (fs.existsSync(local)) return local;

  // 2) 回退到 Node 的模块解析
  try {
    const pkgRoot = path.dirname(require.resolve('@deepseek-ai/dsh/package.json'));
    const viaResolve = path.join(pkgRoot, 'lib', 'bin.js');
    if (fs.existsSync(viaResolve)) return viaResolve;
  } catch (_) {
    /* ignore */
  }

  throw new Error(
    '未找到 @deepseek-ai/dsh。请先在 server/ 目录执行 `npm install`（或运行 npm run setup:server）。',
  );
}

const bin = resolveDshBin();
const args = process.argv.slice(2);

const child = spawn(process.execPath, [bin, ...args], {
  stdio: 'inherit',
  env: process.env,
});

// 转发终止信号，确保上层 kill 能级联到 dsh 进程
for (const sig of ['SIGTERM', 'SIGINT']) {
  process.on(sig, () => {
    if (!child.killed) child.kill(sig);
  });
}

child.on('error', (err) => {
  console.error('[launch-dsh] 启动 dsh 失败:', err);
  process.exit(1);
});

child.on('exit', (code, signal) => {
  // 先移除信号转发监听：若直接 process.kill(pid, signal)，会再次触发上面的 handler，
  // 而 handler 里 child.killed 此时为 false，会再次 child.kill()（对已退出进程无效），
  // 信号被消费后本进程将卡死而不退出，导致上层 wait_ready 一直等到超时。
  for (const sig of ['SIGTERM', 'SIGINT']) process.removeAllListeners(sig);
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 0);
});
