#!/usr/bin/env node
/**
 * CI 变体配置准备脚本。
 *
 * 用法：
 *   VARIANT=plain    node scripts/ci-prepare.mjs   # 未捆绑 Node（目标机器需安装 Node.js）
 *   VARIANT=bundled  node scripts/ci-prepare.mjs   # 捆绑 Node（免依赖分发）
 *
 * 作用：按变体在 src-tauri/tauri.conf.json 中注入 / 移除 bundle.externalBin。
 * 说明：Tauri 2 的 bundle 不支持 v1 的 fileName 字段，产物文件名的变体区分
 *       由 workflow（build-release.yml）在构建后重命名完成，脚本不做文件名处理。
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const confPath = path.resolve(__dirname, '..', 'src-tauri', 'tauri.conf.json');
const variant = process.env.VARIANT || 'plain';

if (!['plain', 'bundled'].includes(variant)) {
  console.error(`[ci-prepare] 未知变体: ${variant}（仅支持 plain / bundled）`);
  process.exit(1);
}

const conf = JSON.parse(fs.readFileSync(confPath, 'utf8'));
conf.bundle = conf.bundle || {};

if (variant === 'bundled') {
  // 与 src-tauri/binaries/node-<target-triple> 对应（下载与命名见 build-release.yml）
  conf.bundle.externalBin = ['binaries/node'];
} else {
  delete conf.bundle.externalBin;
}

fs.writeFileSync(confPath, JSON.stringify(conf, null, 2) + '\n');
console.log(
  `[ci-prepare] variant=${variant} externalBin=${JSON.stringify(conf.bundle.externalBin)}`,
);
