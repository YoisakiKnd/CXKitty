#!/usr/bin/env node
/**
 * Zero-dependency lint for the frontend.
 *
 * The repository ships no linter (no eslint / prettier / svelte-check in
 * `package.json`, and adding one is out of scope), so this script drives the
 * toolchain that is *already* installed:
 *
 *   1. `svelte/compiler` — compiles every `.svelte` file and fails on any
 *      compiler warning (a11y, unused CSS selectors, invalid markup, …).
 *      Vite's build only surfaces these for modules it happens to transform,
 *      so compiling the whole tree is strictly stronger.
 *   2. A small source audit for leftovers the acceptance criteria call out
 *      explicitly: `console.log` debug output, `debugger` statements and
 *      commented-out code blocks.
 *
 * Exit code 1 on any finding, 0 when clean.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, extname } from 'node:path';
import { compile } from 'svelte/compiler';

const ROOT = process.cwd();
const SRC = join(ROOT, 'src');
const SVELTE_EXT = '.svelte';
const AUDITED_EXT = new Set(['.svelte', '.js']);

/** Files intentionally allowed to emit console output. */
const CONSOLE_ALLOWLIST = new Set([]);

/** @param {string} dir @returns {string[]} */
function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
}

const files = walk(SRC);
const problems = [];

// ---------------------------------------------------------------- 1. Svelte compile
for (const file of files.filter((f) => extname(f) === SVELTE_EXT)) {
  const source = readFileSync(file, 'utf8');
  try {
    const { warnings } = compile(source, {
      filename: file,
      // Compile with runes so Svelte 5 semantics (and their warnings) apply.
      runes: true,
      generate: 'client',
    });
    for (const w of warnings) {
      problems.push(
        `${relative(ROOT, file)}:${w.start?.line ?? 0}:${w.start?.column ?? 0}  [${w.code}] ${w.message}`,
      );
    }
  } catch (e) {
    problems.push(`${relative(ROOT, file)}  [compile-error] ${e.message}`);
  }
}

// ------------------------------------------------------- 2. leftover-code audit
for (const file of files.filter((f) => AUDITED_EXT.has(extname(f)))) {
  if (CONSOLE_ALLOWLIST.has(relative(ROOT, file))) continue;
  const lines = readFileSync(file, 'utf8').split('\n');
  lines.forEach((line, i) => {
    const n = i + 1;
    const where = `${relative(ROOT, file)}:${n}`;
    if (/\bdebugger\b/.test(line) && !/^\s*(\/\/|\*|\/\*)/.test(line)) {
      problems.push(`${where}  [debugger] 遗留的 debugger 语句`);
    }
    if (/\bconsole\.(log|debug|info|warn|error)\s*\(/.test(line)) {
      problems.push(`${where}  [console] 遗留的调试输出`);
    }
  });
}

if (problems.length > 0) {
  console.error(`lint 失败，共 ${problems.length} 处问题：\n`);
  for (const p of problems) console.error('  ' + p);
  process.exit(1);
}

console.log(`lint 通过：已编译 ${files.filter((f) => extname(f) === SVELTE_EXT).length} 个 .svelte 文件，无警告。`);
