#!/usr/bin/env node
/**
 * sync-all.mjs
 * Runs every docs-sync step in the correct order. Called automatically by
 * `prestart` / `prebuild` in package.json so `npm start` / `npm run build`
 * always operate on fresh generated content.
 *
 * Steps are independent; a failure in one does not abort the others, but the
 * overall exit code is non-zero if anything failed (so CI notices).
 */
import { spawn } from 'node:child_process';
import path from 'node:path';
import { siteRoot } from './lib/fs-utils.mjs';

const STEPS = [
  { name: 'readme',  script: 'sync-readme.mjs' },
  { name: 'roadmap', script: 'sync-roadmap.mjs' },
  { name: 'misc',    script: 'sync-misc.mjs' },
  { name: 'grc',     script: 'grc-to-mermaid.mjs' },
  { name: 'api',     script: 'extract-api-docs.mjs' },
];

function run(script) {
  return new Promise((resolve) => {
    const p = spawn(process.execPath, [path.join(siteRoot(), 'scripts', script)], {
      stdio: 'inherit',
    });
    p.on('exit', (code) => resolve(code ?? 1));
  });
}

async function main() {
  let failed = 0;
  for (const step of STEPS) {
    process.stdout.write(`\n▶ sync:${step.name}\n`);
    const code = await run(step.script);
    if (code !== 0) {
      console.error(`  ✗ sync:${step.name} exited ${code}`);
      failed++;
    }
  }
  if (failed > 0) {
    console.error(`\nsync-all: ${failed} step(s) failed`);
    process.exit(1);
  }
  console.log('\nsync-all: all steps ok');
}

main();
