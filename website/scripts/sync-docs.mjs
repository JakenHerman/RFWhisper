#!/usr/bin/env node
/**
 * sync-docs.mjs
 * Mirrors hand-written markdown from the repo-root `docs/` directory into the
 * Docusaurus site at docs/_generated/handbook/.
 *
 * Until this step existed, nothing published `docs/` at all — pages written there
 * (virtual-cable-setup, reports, models) simply never appeared on rfwhisper.org,
 * even though issues referred to "the Docusaurus sync picks it up automatically
 * from docs/". This closes that gap.
 *
 * Output is a *partial*, following the same pattern as sync-roadmap / sync-misc: the
 * front matter and H1 are stripped here, and a thin wrapper page under website/docs/
 * supplies the title and imports the body. That keeps the repo-root file readable as
 * plain markdown on GitHub while the site owns its own metadata.
 */
import path from 'node:path';
import { promises as fs } from 'node:fs';
import {
  readFile,
  writeFile,
  repoRoot,
  siteRoot,
  ensureDir,
  stripFirstH1,
  nowStamp,
} from './lib/fs-utils.mjs';

const SRC_DIR = path.join(repoRoot(), 'docs');
const OUT_DIR = path.join(siteRoot(), 'docs', '_generated', 'handbook');

const BLOB = 'https://github.com/JakenHerman/RFWhisper/blob/master';

/**
 * Repo-relative link targets that would 404 on the site, mapped to real destinations.
 *
 * Pages that have a wrapper under website/docs/ get a site route. Pages that do not are
 * sent to GitHub instead — a working link to source beats a broken internal one.
 */
// Routes carry the /docs/next/ prefix: the site is versioned with includeCurrentVersion,
// so unreleased docs live under `next`. Matches the convention in sync-misc.mjs.
const LINK_MAP = {
  '../ROADMAP.md': '/docs/next/roadmap',
  '../README.md': '/docs/next/',
  '../CONTRIBUTING.md': '/docs/next/contributing',
  './virtual-cable-setup.md': '/docs/next/virtual-cable-setup',
  '../website/docs/hardware/virtual-cables.md': '/docs/next/hardware/virtual-cables',
  './reports.md': `${BLOB}/docs/reports.md`,
  './models.md': `${BLOB}/docs/models.md`,
  '../rfwhisper/models/manifest.json': `${BLOB}/rfwhisper/models/manifest.json`,
};

/**
 * Rewrite exact markdown link targets.
 *
 * Deliberately not fs-utils' `rewriteRepoLinks`, which hardcodes a `./` prefix into its
 * pattern and so cannot express `../ROADMAP.md`. Changing that shared helper would touch
 * sync-misc and sync-roadmap for no benefit to them.
 */
function rewriteLinks(md, mapping) {
  let out = md;
  for (const [from, to] of Object.entries(mapping)) {
    const escaped = from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const re = new RegExp(`\\]\\(${escaped}([^)]*)\\)`, 'g');
    out = out.replace(re, (_m, tail) => `](${to}${tail || ''})`);
  }
  return out;
}

/** Drop a leading YAML front-matter block; the wrapper page owns the metadata. */
function stripFrontMatter(md) {
  if (!md.startsWith('---\n')) return md;
  const end = md.indexOf('\n---', 4);
  if (end === -1) return md;
  return md.slice(md.indexOf('\n', end + 1) + 1).replace(/^\s+/, '');
}

async function listMarkdown(dir) {
  let entries;
  try {
    entries = await fs.readdir(dir, { withFileTypes: true });
  } catch (err) {
    if (err.code === 'ENOENT') return [];
    throw err;
  }
  return entries
    .filter((e) => e.isFile() && e.name.endsWith('.md'))
    .map((e) => path.join(dir, e.name));
}

async function main() {
  const sources = await listMarkdown(SRC_DIR);
  if (sources.length === 0) {
    console.log('  (no markdown in docs/ — nothing to sync)');
    return 0;
  }

  await ensureDir(OUT_DIR);

  for (const src of sources) {
    const slug = path.basename(src, '.md');
    let md = await readFile(src);
    md = stripFrontMatter(md);
    md = stripFirstH1(md);
    md = rewriteLinks(md, LINK_MAP);

    md += `\n\n<!-- synced from docs/${slug}.md at ${nowStamp()} -->\n`;
    await writeFile(path.join(OUT_DIR, `${slug}.md`), md);
    console.log(`  ✓ ${slug}`);
  }

  console.log(`  synced ${sources.length} page(s) from docs/`);
  return 0;
}

main().then(
  (code) => process.exit(code),
  (err) => {
    console.error(err);
    process.exit(1);
  }
);
