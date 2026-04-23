#!/usr/bin/env node
/**
 * grc-to-mermaid.mjs
 *
 * GNU Radio Companion files (.grc) are YAML documents describing a flowgraph
 * with `blocks` (each with an `id` and `parameters`) and `connections` (source
 * block + port → sink block + port).
 *
 * This script walks every .grc under <repo>/flowgraphs/, parses it with a
 * minimal YAML reader (no external runtime dep), and emits a Mermaid
 * `flowchart LR` diagram at docs/_generated/flowgraphs/<name>.mmd plus an
 * MDX wrapper at docs/_generated/flowgraphs/<name>.mdx that uses our
 * FlowgraphRenderer component.
 *
 * Pure-Node implementation: no `js-yaml` required. We lean on the fact that
 * GRC's YAML schema is well-structured and we only need top-level `blocks`
 * and `connections` — a ~200-line hand parser is enough and avoids pulling
 * js-yaml into the docs devDeps.
 *
 * If parsing fails for any flowgraph, we log a warning and skip it; this
 * never blocks the docs build.
 */
import path from 'node:path';
import { promises as fs } from 'node:fs';
import {
  readFile,
  writeFile,
  repoRoot,
  siteRoot,
  ensureDir,
  fileExists,
  nowStamp,
} from './lib/fs-utils.mjs';

const FLOWGRAPHS_DIR = path.join(repoRoot(), 'flowgraphs');
const OUT_DIR = path.join(siteRoot(), 'docs', '_generated', 'flowgraphs');

/**
 * Tiny GRC YAML subset reader. Handles:
 *   - scalar: "value"
 *   - scalar: value
 *   - nested key/value under 2-space indents
 *   - `- key: value` list items
 * This is enough for .grc's blocks/connections sections.
 */
function parseYamlGrc(text) {
  const lines = text.split(/\r?\n/);
  const root = {};
  const stack = [{ indent: -1, container: root, keyPending: null }];

  const popTo = (indent) => {
    while (stack.length && stack[stack.length - 1].indent >= indent) {
      stack.pop();
    }
  };

  for (let raw of lines) {
    if (!raw.trim() || raw.trim().startsWith('#')) continue;
    const indent = raw.match(/^\s*/)[0].length;
    let line = raw.slice(indent);

    const listItem = line.startsWith('- ');
    if (listItem) line = line.slice(2);

    popTo(indent);
    const parent = stack[stack.length - 1].container;

    const m = line.match(/^([A-Za-z0-9_.\-]+)\s*:\s*(.*)$/);
    if (!m) continue;
    const key = m[1];
    let val = m[2];

    const parseScalar = (v) => {
      if (v === '') return null;
      if (/^".*"$/.test(v)) return v.slice(1, -1);
      if (/^'.*'$/.test(v)) return v.slice(1, -1);
      if (/^-?\d+(\.\d+)?$/.test(v)) return Number(v);
      if (v === 'true') return true;
      if (v === 'false') return false;
      return v;
    };

    if (listItem) {
      if (!Array.isArray(parent.__list)) parent.__list = [];
      const obj = {};
      parent.__list.push(obj);
      if (val !== '') obj[key] = parseScalar(val);
      stack.push({ indent: indent + 2, container: obj, keyPending: null });
    } else if (val === '') {
      // Nested structure begins.
      const child = {};
      parent[key] = child;
      stack.push({ indent: indent + 2, container: child, keyPending: null });
    } else {
      parent[key] = parseScalar(val);
    }
  }

  // Promote any `__list` children to the array form expected by consumers.
  const unwrap = (node) => {
    if (node && typeof node === 'object') {
      for (const [k, v] of Object.entries(node)) {
        if (v && typeof v === 'object' && Array.isArray(v.__list)) {
          node[k] = v.__list.map(unwrap);
        } else {
          node[k] = unwrap(v);
        }
      }
    }
    return node;
  };
  return unwrap(root);
}

/** Attempt to extract a human label for a GRC block id. */
function labelFor(block) {
  const id = block.id || '?';
  const name = block.name || '';
  const friendly = {
    soapy_source: 'SoapySDR Source',
    soapy_sink: 'SoapySDR Sink',
    audio_sink: 'Audio Sink',
    audio_source: 'Audio Source',
    rational_resampler_xxx: 'Rational Resampler',
    pfb_arb_resampler_xxx: 'PFB Resampler',
    freq_xlating_fir_filter_xxx: 'Freq Xlating FIR',
    analog_am_demod_cf: 'AM Demod',
    analog_ssb_demod_cf: 'SSB Demod',
    analog_nbfm_rx: 'NBFM RX',
    analog_wfm_rcv: 'WBFM RX',
    dnn_inference: 'gr-dnn · DeepFilterNet3',
    blocks_multiply_const_vxx: 'Gain',
    blocks_throttle: 'Throttle',
    blocks_float_to_short: 'Float → Short',
  }[id] || id;
  return name ? `${friendly}<br/>${name}` : friendly;
}

function toMermaid(grc, sourcePath) {
  const blocks = (grc.blocks || []).map((b) => ({
    id: b.name || b.id || 'block',
    label: labelFor(b),
  }));
  const connections = (grc.connections || []).map((c) => {
    // GRC stores connections as arrays: [src_block, src_port, dst_block, dst_port]
    if (Array.isArray(c)) return { from: c[0], to: c[2] };
    return { from: c.from, to: c.to };
  });

  const idx = new Map();
  blocks.forEach((b, i) => idx.set(b.id, `N${i}`));

  const lines = ['flowchart LR'];
  for (const b of blocks) {
    const nid = idx.get(b.id);
    lines.push(`  ${nid}["${b.label}"]`);
  }
  for (const c of connections) {
    const a = idx.get(c.from);
    const z = idx.get(c.to);
    if (!a || !z) continue;
    lines.push(`  ${a} --> ${z}`);
  }
  lines.push(`  %% source: ${sourcePath}`);
  return lines.join('\n');
}

async function processOne(grcPath) {
  const text = await readFile(grcPath);
  const grc = parseYamlGrc(text);
  const name = path.basename(grcPath, '.grc');
  const mermaid = toMermaid(grc, path.relative(repoRoot(), grcPath));

  const mdx = `---
id: ${name}
title: ${name}
hide_title: true
---

import FlowgraphRenderer from '@site/src/components/FlowgraphRenderer';

<FlowgraphRenderer
  name="${name}"
  source="flowgraphs/${name}.grc">
{\`
${mermaid}
\`}
</FlowgraphRenderer>
`;
  await writeFile(path.join(OUT_DIR, `${name}.mdx`), mdx);
  await writeFile(path.join(OUT_DIR, `${name}.mmd`), mermaid);
  console.log(`[grc→mermaid] ${name}`);
}

async function main() {
  await ensureDir(OUT_DIR);
  if (!(await fileExists(FLOWGRAPHS_DIR))) {
    console.log('[grc→mermaid] no flowgraphs/ directory yet — skipping.');
    return;
  }
  const entries = await fs.readdir(FLOWGRAPHS_DIR);
  const grcs = entries.filter((e) => e.endsWith('.grc'));
  if (grcs.length === 0) {
    console.log('[grc→mermaid] no .grc files — skipping.');
    return;
  }
  for (const e of grcs) {
    try {
      await processOne(path.join(FLOWGRAPHS_DIR, e));
    } catch (err) {
      console.warn(`[grc→mermaid] failed to parse ${e}: ${err.message}`);
    }
  }
  // Index file listing all extracted flowgraphs (useful for a future index page).
  const index =
    `<!-- auto-generated ${nowStamp()} -->\n\n` +
    grcs.map((g) => `- [\`${g}\`](./${path.basename(g, '.grc')})`).join('\n') +
    '\n';
  await writeFile(path.join(OUT_DIR, 'index.md'), index);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
