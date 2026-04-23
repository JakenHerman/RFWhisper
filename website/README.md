# RFWhisper Documentation Site

This is the Docusaurus v3 source for <https://rfwhisper.org/>.

Configure the live site with a [GitHub custom domain](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site) (`website/static/CNAME` → `rfwhisper.org`) and point DNS to GitHub Pages.

## Local development

```bash
cd website
npm install
npm start        # opens http://localhost:3000
```

`npm start` automatically runs `npm run sync:all` first, which:

- Extracts sections from the repo-root [`README.md`](../README.md) into `docs/_generated/readme-*.md`
- Mirrors [`ROADMAP.md`](../ROADMAP.md), [`CONTRIBUTING.md`](../CONTRIBUTING.md), and [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md) into `docs/_generated/`
- Converts every `flowgraphs/*.grc` into a Mermaid diagram under `docs/_generated/flowgraphs/`
- Extracts Python docstrings from `rfwhisper/*.py` into `docs/_generated/api/`

## Build

```bash
npm run build            # full static site at ./build
npm run serve            # serve the built site
```

## Deploy

Automatic via [`.github/workflows/deploy-docs.yml`](../.github/workflows/deploy-docs.yml) on every push to `main`. Tagged releases (`vX.Y.Z`) also snapshot the current docs as a version.

Manual:

```bash
GIT_USER=<your-github-user> npm run deploy
```

## Versioning

```bash
npm run docs:version 0.1.0
```

Captures the current `docs/` tree into `versioned_docs/version-0.1.0/`. The live `docs/` tree becomes the new "Next" (unreleased) version.

## Editing content

- **Pages under `docs/`** are hand-maintained MDX / Markdown.
- **Pages under `docs/_generated/`** are produced by `scripts/` — edit the repo-root source, not the generated copy.
- **Components** live under `src/components/`. The custom `FlowgraphRenderer` is used by both hand-authored flowgraph pages and the generated ones.

## Project structure

```
website/
├── docs/
│   ├── _generated/          ← auto-synced (do NOT edit by hand)
│   ├── architecture/
│   ├── hardware/
│   ├── installation/
│   ├── models/
│   ├── quickstart/
│   ├── use-cases/
│   ├── intro.md
│   ├── why-rfwhisper.md
│   ├── roadmap.mdx          ← MDX wrapper (imports _generated/roadmap.md)
│   ├── contributing.mdx     ← ditto _generated/contributing.md
│   ├── code-of-conduct.mdx  ← ditto _generated/code-of-conduct.md
│   └── faq.md
├── scripts/
│   ├── sync-all.mjs         ← orchestrator (runs in prestart/prebuild)
│   ├── sync-readme.mjs
│   ├── sync-roadmap.mjs
│   ├── sync-misc.mjs
│   ├── grc-to-mermaid.mjs
│   ├── extract-api-docs.mjs
│   └── lib/fs-utils.mjs
├── src/
│   ├── components/
│   │   ├── FlowgraphRenderer/
│   │   └── landing/         ← homepage (spectrum panel, pillars, doc strip)
│   ├── css/custom.css       ← theme + dedicated landing (warm “shack” canvas)
│   └── pages/index.tsx      ← home (workbench + spectrum, not a code terminal)
├── static/img/
├── docusaurus.config.ts
├── sidebars.ts
└── package.json
```

73 and keep the docs green.
