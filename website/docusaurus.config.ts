import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

// -----------------------------------------------------------------------------
// RFWhisper documentation site configuration.
//
// Key choices:
//   * Canonical public URL: https://rfwhisper.org/ (CNAME for GitHub Pages;
//     override with DOCS_SITE_URL / DOCS_BASE_URL in CI or for local testing).
//   * Versioning is enabled via the built-in `docs` plugin. Tagged releases
//     should run `npm run docs:version <version>` via the workflow.
//   * Mermaid is enabled (used by FlowgraphRenderer and generated diagrams).
//   * `_generated/` content under /docs is refreshed pre-build by
//     scripts/sync-all.mjs (README sections, ROADMAP milestones, GRC → Mermaid,
//     Python docstrings → MDX).
// -----------------------------------------------------------------------------

const GH_ORG = process.env.GH_ORG || 'jakenherman';
const GH_REPO = process.env.GH_REPO || 'rfwhisper';
const SITE_URL = process.env.DOCS_SITE_URL || 'https://rfwhisper.org';
const BASE_URL = process.env.DOCS_BASE_URL || '/';
const EDIT_BASE = `https://github.com/${GH_ORG}/${GH_REPO}/edit/main/website/`;

const config: Config = {
  title: 'RFWhisper',
  tagline: 'Real-time AI denoising for ham radio. No cloud. No compromises.',
  favicon: 'img/favicon.svg',

  url: SITE_URL,
  baseUrl: BASE_URL,

  organizationName: GH_ORG,
  projectName: GH_REPO,
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'warn',
  onDuplicateRoutes: 'warn',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  markdown: {
    mermaid: true,
    format: 'detect',
    // v4: top-level onBrokenMarkdownLinks is deprecated; use hooks
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  themes: ['@docusaurus/theme-mermaid'],

  plugins: [
    [
      '@docusaurus/plugin-ideal-image',
      {
        quality: 85,
        max: 1600,
        min: 640,
        steps: 4,
        disableInDev: false,
      },
    ],
  ],

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: 'docs',
          editUrl: EDIT_BASE,
          showLastUpdateAuthor: true,
          showLastUpdateTime: true,
          // Versioning is enabled here. `next` (unreleased) lives in /docs.
          // Archived versions get pinned by `docusaurus docs:version`.
          includeCurrentVersion: true,
          lastVersion: 'current',
          versions: {
            current: {
              label: 'Next (v0.1-dev)',
              path: 'next',
              banner: 'unreleased',
            },
          },
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
        sitemap: {
          changefreq: 'weekly',
          priority: 0.5,
          ignorePatterns: ['/tags/**'],
          filename: 'sitemap.xml',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/rfwhisper-social-card.png',
    metadata: [
      { name: 'keywords', content: 'ham radio, sdr, noise reduction, deepfilternet, gnuradio, soapysdr, ft8, cw, ssb, rnnoise, onnx, onnxruntime, rfi, qrm, amateur-radio' },
      { name: 'theme-color', content: '#0b1220' },
    ],
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    announcementBar: {
      id: 'alpha-banner',
      content:
        '📡 RFWhisper is in <b>alpha</b> — help us ship v0.1 by contributing <a href="/docs/next/contributing">testable improvements</a>. 73!',
      backgroundColor: '#0d3b66',
      textColor: '#f4f6ff',
      isCloseable: true,
    },
    navbar: {
      title: 'RFWhisper',
      logo: {
        alt: 'RFWhisper — clean copy from the noisy RF aether',
        src: 'img/logo.svg',
        srcDark: 'img/logo-dark.svg',
      },
      hideOnScroll: false,
      items: [
        { type: 'docSidebar', sidebarId: 'docsSidebar', position: 'left', label: 'Docs' },
        { to: '/docs/next/roadmap', label: 'Roadmap', position: 'left' },
        { to: '/docs/next/hardware/', label: 'Hardware', position: 'left' },
        { to: '/docs/next/use-cases/pota', label: 'Use Cases', position: 'left' },
        {
          type: 'docsVersionDropdown',
          position: 'right',
          dropdownActiveClassDisabled: true,
        },
        {
          href: `https://github.com/${GH_ORG}/${GH_REPO}`,
          position: 'right',
          className: 'header-github-link',
          'aria-label': 'GitHub repository',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            { label: 'Introduction', to: '/docs/next/intro' },
            { label: 'Quick Start', to: '/docs/next/quickstart/' },
            { label: 'Architecture', to: '/docs/next/architecture/' },
            { label: 'Roadmap', to: '/docs/next/roadmap' },
            { label: 'FAQ', to: '/docs/next/faq' },
          ],
        },
        {
          title: 'Community',
          items: [
            { label: 'GitHub Discussions', href: `https://github.com/${GH_ORG}/${GH_REPO}/discussions` },
            { label: 'Issue Tracker', href: `https://github.com/${GH_ORG}/${GH_REPO}/issues` },
            { label: 'Contributing', to: '/docs/next/contributing' },
            { label: 'Code of Conduct', to: '/docs/next/code-of-conduct' },
          ],
        },
        {
          title: 'Project',
          items: [
            { label: 'GitHub', href: `https://github.com/${GH_ORG}/${GH_REPO}` },
            { label: 'Releases', href: `https://github.com/${GH_ORG}/${GH_REPO}/releases` },
            { label: 'License (GPLv3)', href: `https://github.com/${GH_ORG}/${GH_REPO}/blob/main/LICENSE` },
            { label: 'AGENTS.md', href: `https://github.com/${GH_ORG}/${GH_REPO}/blob/main/AGENTS.md` },
          ],
        },
      ],
      copyright: `© ${new Date().getFullYear()} RFWhisper contributors. GPL-3.0-or-later. <br/> <em>73 — keep the signal clean.</em>`,
    },
    prism: {
      theme: prismThemes.oneLight,
      darkTheme: prismThemes.vsDark,
      additionalLanguages: [
        'bash',
        'diff',
        'json',
        'yaml',
        'toml',
        'python',
        'cpp',
        'cmake',
        'ini',
        'rust',
        'markdown',
      ],
      magicComments: [
        { className: 'theme-code-block-highlighted-line', line: 'highlight-next-line', block: { start: 'highlight-start', end: 'highlight-end' } },
        { className: 'code-block-ham-line', line: 'ham-callout' },
      ],
    },
    mermaid: {
      theme: { light: 'neutral', dark: 'dark' },
      options: {
        fontFamily: 'JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, monospace',
      },
    },
    algolia: process.env.ALGOLIA_APP_ID
      ? {
          appId: process.env.ALGOLIA_APP_ID,
          apiKey: process.env.ALGOLIA_API_KEY!,
          indexName: process.env.ALGOLIA_INDEX || 'rfwhisper',
          contextualSearch: true,
          searchPagePath: 'search',
        }
      : undefined,
    tableOfContents: {
      minHeadingLevel: 2,
      maxHeadingLevel: 4,
    },
    docs: {
      sidebar: {
        hideable: true,
        autoCollapseCategories: false,
      },
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
