import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'intro',
    'why-rfwhisper',
    {
      type: 'category',
      label: 'Installation',
      link: { type: 'doc', id: 'installation/index' },
      collapsed: false,
      items: [
        'installation/linux',
        'installation/macos',
        'installation/windows',
        'installation/raspberry-pi',
      ],
    },
    {
      type: 'category',
      label: 'Quick Start',
      link: { type: 'doc', id: 'quickstart/index' },
      collapsed: false,
      items: [
        'quickstart/v0_1-test-guide',
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      link: { type: 'doc', id: 'architecture/index' },
      collapsed: true,
      items: [
        'architecture/signal-flow',
        'architecture/latency-budget',
        'architecture/flowgraphs',
      ],
    },
    {
      type: 'category',
      label: 'Models & Training',
      link: { type: 'doc', id: 'models/index' },
      collapsed: true,
      items: [
        'models/training',
        'models/fine-tuning',
        'models/model-cards',
      ],
    },
    {
      type: 'category',
      label: 'Hardware',
      link: { type: 'doc', id: 'hardware/index' },
      collapsed: true,
      items: [
        'hardware/sdrs',
        'hardware/rigs',
        'hardware/virtual-cables',
      ],
    },
    {
      type: 'category',
      label: 'Use Cases',
      collapsed: true,
      items: [
        'use-cases/pota',
        'use-cases/contesting',
        'use-cases/weak-signal',
      ],
    },
    'roadmap',
    'faq',
    'contributing',
    'code-of-conduct',
  ],
};

export default sidebars;
