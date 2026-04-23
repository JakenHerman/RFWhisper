import { promises as fs } from 'node:fs';
import path from 'node:path';

/** Resolve a path relative to the website/ root (regardless of cwd). */
export function siteRoot() {
  return path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
}

/** Resolve a path relative to the repository root (one up from website/). */
export function repoRoot() {
  return path.resolve(siteRoot(), '..');
}

export async function readFile(p) {
  return fs.readFile(p, 'utf8');
}

export async function writeFile(p, contents) {
  await fs.mkdir(path.dirname(p), { recursive: true });
  await fs.writeFile(p, contents, 'utf8');
}

export async function ensureDir(p) {
  await fs.mkdir(p, { recursive: true });
}

export async function fileExists(p) {
  try {
    await fs.access(p);
    return true;
  } catch {
    return false;
  }
}

/** MDX-escape angle brackets that aren't part of intentional JSX/HTML. */
export function escapeMdx(markdown) {
  // Escape angle-bracket URLs and generic-typed expressions that MDX parses as JSX.
  // This is conservative — we do NOT touch fenced code blocks.
  const parts = markdown.split(/(```[\s\S]*?```)/g);
  return parts
    .map((part, i) => {
      if (i % 2 === 1) return part; // code block — leave alone
      return part
        // <https://...> autolinks
        .replace(/<(https?:\/\/[^>\s]+)>/g, '[$1]($1)')
        // Generic "<Name>" that isn't a known component looks like JSX to MDX —
        // escape lone angle brackets that precede a lowercase letter plus '>'.
        // Very conservative: only target patterns like "<foo>" where foo is
        // not a valid MDX/JSX tag we know about.
        .replace(/<(\/?[a-z][a-z0-9-]*)(\s|>)/g, (m, tag, rest) => {
          const safe = new Set(['br', 'hr', 'a', 'b', 'i', 'em', 'strong', 'code', 'pre', 'kbd', 'span', 'sup', 'sub']);
          if (safe.has(tag.replace('/', ''))) return m;
          return `&lt;${tag}${rest}`;
        });
    })
    .join('');
}

/** Rewrite links that target repo-root files into doc-site anchors where possible. */
export function rewriteRepoLinks(markdown, mapping) {
  let out = markdown;
  for (const [from, to] of Object.entries(mapping)) {
    const re = new RegExp(`\\(\\.\\/${from}([^)]*)\\)`, 'g');
    out = out.replace(re, (_m, hash) => `(${to}${hash || ''})`);
  }
  return out;
}

/** Strip the first H1 heading (so the Docusaurus front-matter title doesn't double). */
export function stripFirstH1(markdown) {
  return markdown.replace(/^# [^\n]*\n+/, '');
}

/** Build a Docusaurus front-matter block. */
export function fm(meta) {
  const body = Object.entries(meta)
    .map(([k, v]) => `${k}: ${typeof v === 'string' ? v : JSON.stringify(v)}`)
    .join('\n');
  return `---\n${body}\n---\n\n`;
}

/** Stable timestamp string for the "last synced" banner. */
export function nowStamp() {
  return new Date().toISOString().replace('T', ' ').replace(/\..+/, 'Z');
}
