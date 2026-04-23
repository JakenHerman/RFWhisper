import React from 'react';
import Link from '@docusaurus/Link';

const LINKS = [
  { label: 'Install (all platforms + RPi)', to: '/docs/next/installation/' },
  { label: 'Quick start', to: '/docs/next/quickstart/' },
  { label: 'v0.1 test guide (acceptance)', to: '/docs/next/quickstart/v0_1-test-guide' },
  { label: 'Hardware matrix', to: '/docs/next/hardware/sdrs' },
  { label: 'Roadmap (testable)', to: '/docs/next/roadmap' },
] as const;

export default function DocStrip(): JSX.Element {
  return (
    <nav className="rfw-landing__docstrip" aria-label="Key documentation">
      <h2 className="rfw-landing__docstrip-label">Handbook</h2>
      <ul>
        {LINKS.map((l) => (
          <li key={l.to}>
            <Link to={l.to} className="rfw-landing__doclink">
              {l.label}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
