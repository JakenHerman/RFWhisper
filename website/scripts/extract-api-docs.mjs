#!/usr/bin/env node
/**
 * extract-api-docs.mjs
 *
 * Extracts Python docstrings from the RFWhisper package (repo root
 * `rfwhisper/`) and emits one MDX page per public module under
 * docs/_generated/api/.
 *
 * Heuristics (pure regex — no Python required at doc-build time):
 *   - Module-level docstring = first triple-quoted string at top of file
 *   - Public functions / classes = top-level `def`/`class` not starting with _
 *   - Function docstring = first triple-quoted string after the signature
 *   - Google-style sections ("Args:", "Returns:", "Raises:") render as subheads
 *
 * This is deliberately simple. When the Python package matures we can swap in
 * pdoc or sphinx-api-json, but this stub is resilient (no Python in CI needed)
 * and keeps the docs structure correct from day one.
 */
import path from 'node:path';
import { promises as fs } from 'node:fs';
import {
  writeFile,
  repoRoot,
  siteRoot,
  ensureDir,
  fileExists,
  nowStamp,
} from './lib/fs-utils.mjs';

const PKG_DIR = path.join(repoRoot(), 'rfwhisper');
const OUT_DIR = path.join(siteRoot(), 'docs', '_generated', 'api');

async function listPy(dir) {
  const out = [];
  const walk = async (d) => {
    const entries = await fs.readdir(d, { withFileTypes: true });
    for (const e of entries) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) await walk(p);
      else if (e.isFile() && e.name.endsWith('.py') && !e.name.startsWith('_')) out.push(p);
    }
  };
  await walk(dir);
  return out;
}

function extractModuleDocstring(src) {
  const m = src.match(/^\s*(?:[ruRU]?)(?:"""|''')([\s\S]*?)\1\s*/);
  if (!m) return '';
  const body = m[1].trim();
  return body.replace(/\r/g, '');
}

function extractSymbols(src) {
  const out = [];
  const re = /^(def|class)\s+([A-Za-z][A-Za-z0-9_]*)\s*(?:\(([^)]*)\))?\s*(?:->\s*[^\n:]+)?\s*:/gm;
  let m;
  while ((m = re.exec(src))) {
    const [, kind, name, signature] = m;
    if (name.startsWith('_')) continue;
    const after = src.slice(m.index + m[0].length);
    const ds = after.match(/^\s*(?:[ruRU]?)(?:"""|''')([\s\S]*?)\1/);
    out.push({
      kind,
      name,
      signature: signature ? `(${signature.trim()})` : '',
      docstring: ds ? ds[1].trim() : '',
    });
  }
  return out;
}

function renderSymbol(s) {
  const lines = [];
  lines.push(`### \`${s.kind} ${s.name}${s.signature}\``);
  lines.push('');
  if (s.docstring) {
    // Promote Google-style sections
    const doc = s.docstring
      .replace(/^Args:\s*$/gm, '**Args**')
      .replace(/^Returns:\s*$/gm, '**Returns**')
      .replace(/^Raises:\s*$/gm, '**Raises**')
      .replace(/^Example[s]?:\s*$/gm, '**Example**');
    lines.push(doc);
  } else {
    lines.push('*No docstring yet.*');
  }
  lines.push('');
  return lines.join('\n');
}

async function renderOne(pyPath) {
  const src = await fs.readFile(pyPath, 'utf8');
  const rel = path.relative(PKG_DIR, pyPath).replace(/\.py$/, '').replace(/\//g, '.');
  const title = `rfwhisper.${rel}`;
  const moduleDoc = extractModuleDocstring(src);
  const syms = extractSymbols(src);

  const body =
    `---\nid: ${title}\ntitle: \`${title}\`\nhide_title: false\n---\n\n` +
    `<!-- auto-generated ${nowStamp()} — do not edit -->\n\n` +
    (moduleDoc ? moduleDoc + '\n\n' : '') +
    (syms.length ? '## API\n\n' + syms.map(renderSymbol).join('\n') : '') +
    `\n---\n\n[\`${path.relative(repoRoot(), pyPath)}\` on GitHub](https://github.com/jakenherman/rfwhisper/blob/main/${path.relative(repoRoot(), pyPath)})\n`;

  const outPath = path.join(OUT_DIR, `${title}.md`);
  await writeFile(outPath, body);
}

async function main() {
  await ensureDir(OUT_DIR);
  if (!(await fileExists(PKG_DIR))) {
    console.log('[extract-api] no rfwhisper/ package yet — skipping.');
    // Emit an index placeholder so the docs sidebar doesn't complain later.
    await writeFile(
      path.join(OUT_DIR, 'index.md'),
      `# API Reference\n\n*The Python package \`rfwhisper/\` has not landed yet — once v0.1 code merges, this section auto-generates from docstrings.*\n`,
    );
    return;
  }
  const files = await listPy(PKG_DIR);
  for (const f of files) await renderOne(f);
  console.log(`[extract-api] wrote ${files.length} module pages`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
