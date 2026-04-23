import React from 'react';
import Link from '@docusaurus/Link';

const PILLARS = [
  {
    k: '1',
    title: 'Your CPU, your shack',
    text: 'No cloud inference. No accounts. Audio stays on the machine in front of you — laptop, field box, or Raspberry Pi.',
    to: '/docs/next/architecture/',
  },
  {
    k: '2',
    title: 'Models built for our bands',
    text: 'DeepFilterNet3 as the primary; RNNoise when you need something tiny. Trained and fine-tuned on real QRM, not just speech corpora.',
    to: '/docs/next/models/',
  },
  {
    k: '3',
    title: 'Proof, not marketing',
    text: 'Roadmap criteria you can run: FT8 non-regressions, CW transient checks, end-to-end latency. If it does not help on-air, it does not ship.',
    to: '/docs/next/quickstart/v0_1-test-guide',
  },
  {
    k: '4',
    title: 'GNU Radio + SoapySDR',
    text: 'Universal hardware via SoapySDR; v0.2 brings flowgraphs and gr-dnn ONNX. Bring whatever rig or dongle the bands give you.',
    to: '/docs/next/architecture/flowgraphs',
  },
] as const;

export default function HomepagePillars(): JSX.Element {
  return (
    <section className="rfw-landing__pillars" aria-labelledby="pillars-h">
      <h2 id="pillars-h" className="rfw-landing__h2">
        What we care about
      </h2>
      <ol className="rfw-landing__pillar-list">
        {PILLARS.map((p) => (
          <li key={p.k} className="rfw-landing__pillar">
            <span className="rfw-landing__pillar-idx" aria-hidden>
              {p.k}
            </span>
            <div>
              <h3 className="rfw-landing__pillar-title">
                {p.to ? (
                  <Link to={p.to} className="rfw-landing__pillar-link">
                    {p.title}
                  </Link>
                ) : (
                  p.title
                )}
              </h3>
              <p className="rfw-landing__pillar-text">{p.text}</p>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
