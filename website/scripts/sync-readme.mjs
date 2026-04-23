#!/usr/bin/env node
/**
 * sync-readme.mjs
 * Pulls selected sections from the repo-root README.md into the docs site.
 *
 * - The README is the canonical homepage content. We do NOT republish the
 *   whole file as a doc page; instead we pick sections (by heading) and emit
 *   them as partial MDX into docs/_generated/readme-*.mdx so they can be
 *   embedded with <ReadmeSection id="..." /> (or imported directly).
 *
 * - Run via `npm run sync:readme` (auto-invoked by `sync:all` on prebuild).
 */
import path from 'node:path';
import {
  readFile,
  writeFile,
  repoRoot,
  siteRoot,
  escapeMdx,
  nowStamp,
  fm,
} from './lib/fs-utils.mjs';

const SECTIONS = [
  { heading: 'Why RFWhisper?', out: 'readme-why.md' },
  { heading: 'Features', out: 'readme-features.md' },
  { heading: 'How It Works', out: 'readme-how-it-works.md' },
  { heading: 'Hardware Requirements', out: 'readme-hardware.md' },
  { heading: 'Testable Success Examples', out: 'readme-testable-success.md' },
];

function extractSection(md, heading) {
  const re = new RegExp(`^(##+)\\s+${heading}\\s*$`, 'm');
  const m = md.match(re);
  if (!m) return null;
  const startLevel = m[1].length;
  const start = m.index + m[0].length;
  const rest = md.slice(start);
  const next = rest.match(new RegExp(`^#{1,${startLevel}}\\s`, 'm'));
  const body = next ? rest.slice(0, next.index) : rest;
  return body.trim() + '\n';
}

async function main() {
  const readme = await readFile(path.join(repoRoot(), 'README.md'));
  const outDir = path.join(siteRoot(), 'docs', '_generated');
  const stamp = nowStamp();

  for (const { heading, out } of SECTIONS) {
    const body = extractSection(readme, heading);
    if (!body) {
      console.warn(`[sync-readme] section not found in README.md: ${heading}`);
      continue;
    }
    const header = `<!--\n  Auto-generated from README.md § ${heading}\n  at ${stamp}. Do not edit — run 'npm run sync:readme'.\n-->\n\n`;
    const file = path.join(outDir, out);
    await writeFile(file, header + escapeMdx(body));
    console.log(`[sync-readme] wrote ${path.relative(siteRoot(), file)}`);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
