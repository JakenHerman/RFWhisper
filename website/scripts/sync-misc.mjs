#!/usr/bin/env node
/**
 * sync-misc.mjs
 * Mirrors short repo-root markdown files (CONTRIBUTING, CODE_OF_CONDUCT) into
 * docs/_generated/ so the Docusaurus pages under the same slug can embed them.
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

const MIRRORS = [
  {
    src: 'CONTRIBUTING.md',
    out: 'contributing.md',
    links: {
      'CODE_OF_CONDUCT.md': '/docs/next/code-of-conduct',
      'README.md': '/docs/next/',
      'ROADMAP.md': '/docs/next/roadmap',
      'LICENSE':
        'https://github.com/jakenherman/rfwhisper/blob/main/LICENSE',
      'AGENTS.md':
        'https://github.com/jakenherman/rfwhisper/blob/main/AGENTS.md',
      '.github/workflows/basic-ci.yml':
        'https://github.com/jakenherman/rfwhisper/blob/main/.github/workflows/basic-ci.yml',
    },
  },
  {
    src: 'CODE_OF_CONDUCT.md',
    out: 'code-of-conduct.md',
    links: {
      'CONTRIBUTING.md': '/docs/next/contributing',
    },
  },
];

async function main() {
  const outDir = path.join(siteRoot(), 'docs', '_generated');
  const stamp = nowStamp();

  for (const { src, out, links } of MIRRORS) {
    let md = await readFile(path.join(repoRoot(), src));
    md = stripFirstH1(md);
    md = rewriteRepoLinks(md, links);
    md = escapeMdx(md);
    const banner = `<!--\n  Auto-generated from ${src} at ${stamp}.\n  Do not edit — run 'npm run sync:all'.\n-->\n\n`;
    await writeFile(path.join(outDir, out), banner + md);
    console.log(`[sync-misc] wrote ${out}`);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
