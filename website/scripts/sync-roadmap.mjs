#!/usr/bin/env node
/**
 * sync-roadmap.mjs
 * Converts the repo-root ROADMAP.md into a Docusaurus-ready MDX doc at
 * docs/_generated/roadmap.md. Adds:
 *   - A one-line "last synced" banner
 *   - Admonition conversion for "stretch goals" and "exit criteria"
 *   - Anchors for every acceptance criterion (A1, B3, …) so other docs can
 *     deep-link to them.
 */
import path from 'node:path';
import {
  readFile,
  writeFile,
  repoRoot,
  siteRoot,
  stripFirstH1,
  escapeMdx,
  rewriteRepoLinks,
  nowStamp,
} from './lib/fs-utils.mjs';

function addCriterionAnchors(md) {
  // Add an invisible anchor before any table row starting with | A1 |, | B3 |, etc.
  // So other pages can link to /docs/roadmap#a1, #b3, …
  return md.replace(/^\| ([A-F]\d+) \|/gm, (line, id) => {
    return `<span id="${id.toLowerCase()}" />\n${line}`;
  });
}

function promoteExitCriteria(md) {
  return md.replace(
    /## Exit Criteria \(Tag (v[\d.]+)\)\n\n> ([^\n]+)/g,
    (_m, ver, body) =>
      `## Exit Criteria — ${ver}\n\n:::tip Ship gate\n${body}\n:::\n`
  );
}

async function main() {
  const src = path.join(repoRoot(), 'ROADMAP.md');
  const dst = path.join(siteRoot(), 'docs', '_generated', 'roadmap.md');

  let md = await readFile(src);
  md = stripFirstH1(md);
  md = addCriterionAnchors(md);
  md = promoteExitCriteria(md);
  md = rewriteRepoLinks(md, {
    'README.md': '/docs/next/',
    'CONTRIBUTING.md': '/docs/next/contributing',
    'AGENTS.md': 'https://github.com/jakenherman/rfwhisper/blob/main/AGENTS.md',
    'LICENSE': 'https://github.com/jakenherman/rfwhisper/blob/main/LICENSE',
    '.github/workflows/basic-ci.yml':
      'https://github.com/jakenherman/rfwhisper/blob/main/.github/workflows/basic-ci.yml',
  });
  md = escapeMdx(md);

  const banner =
    `<!--\n  Auto-generated from ROADMAP.md at ${nowStamp()}.\n  Do not edit — run 'npm run sync:roadmap'.\n-->\n\n` +
    `:::note Synced from \`ROADMAP.md\`\nLast update: ${nowStamp()} · source of truth: [\`ROADMAP.md\`](https://github.com/jakenherman/rfwhisper/blob/main/ROADMAP.md)\n:::\n\n`;

  await writeFile(dst, banner + md);
  console.log(`[sync-roadmap] wrote ${path.relative(siteRoot(), dst)}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
