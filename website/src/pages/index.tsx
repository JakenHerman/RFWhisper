import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';

import { SpectrumPanel } from '@site/src/components/landing/SpectrumPanel';
import HomepagePillars from '@site/src/components/landing/HomepagePillars';
import DocStrip from '@site/src/components/landing/DocStrip';

/**
 * Front page: “workbench + spectrum” — intentionally not a two-column
 * hero + code terminal + feature-card grid (common distributed-computing pattern).
 */
export default function Home(): JSX.Element {
  const { siteConfig } = useDocusaurusContext();
  return (
    <Layout
      title={siteConfig.title}
      description="Real-time, fully local AI denoising for amateur radio SDRs and rigs. DeepFilterNet3, GNU Radio, SoapySDR, ONNX Runtime. GPLv3.">
      <div className="rfw-landing">
        <header className="rfw-landing__head">
          <p className="rfw-landing__kicker">Amateur radio · open source</p>
          <h1 className="rfw-landing__h1">Clean copy from a noisy aether</h1>
          <p className="rfw-landing__lede">
            RFWhisper pulls weak voice and data modes out of modern QRM and QRN
            with ham-tuned deep learning — on your machine, in real time, under a
            testable latency budget. No cloud, no “send my audio to a server,”
            just better copy for SSB, CW, FT8, and whatever the bands throw at
            you.
          </p>
          <div className="rfw-landing__actions">
            <Link className="button button--primary button--lg" to="/docs/next/quickstart/">
              Read the quick start
            </Link>
            <Link
              className="button button--secondary button--lg"
              to="/docs/next/installation/">
              Install for your OS
            </Link>
            <a
              className="button button--outline button--lg"
              href="https://github.com/jakenherman/rfwhisper">
              Source on GitHub
            </a>
          </div>
          <ul className="rfw-landing__badges" aria-label="Key facts">
            <li>GPLv3 — inspect everything</li>
            <li>Target &lt; 100 ms latency (v0.1)</li>
            <li>ONNX + SoapySDR + GNU Radio</li>
          </ul>
        </header>

        <section className="rfw-landing__plot" aria-labelledby="plot-h">
          <h2 id="plot-h" className="rfw-landing__plot-title">
            What we’re after (simplified)
          </h2>
          <p className="rfw-landing__plot-sub">
            Not a product chart — a sketch. Push the <strong>broad</strong> mess
            down without smearing the parts of the passband you care about. Real
            metrics live in the{' '}
            <Link to="/docs/next/quickstart/v0_1-test-guide">test guide</Link>.
          </p>
          <SpectrumPanel />
        </section>

        <div className="rfw-landing__split">
          <HomepagePillars />
          <DocStrip />
        </div>

        <footer className="rfw-landing__foot">
          <p>
            Community-driven. If a milestone doesn’t help an operator on the
            air, it doesn’t ship — see the{' '}
            <Link to="/docs/next/roadmap">roadmap</Link>.{' '}
            <span className="rfw-73" aria-label="73">
              73
            </span>
          </p>
        </footer>
      </div>
    </Layout>
  );
}
