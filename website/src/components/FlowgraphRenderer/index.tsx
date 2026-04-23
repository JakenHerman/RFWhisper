import React from 'react';
import Mermaid from '@theme/Mermaid';

/**
 * FlowgraphRenderer — renders a GNU Radio flowgraph as a labeled Mermaid diagram.
 *
 * Usage in MDX:
 *
 *   import FlowgraphRenderer from '@site/src/components/FlowgraphRenderer';
 *
 *   <FlowgraphRenderer
 *     name="rtl_ssb_hf"
 *     sdr="RTL-SDR v4"
 *     mode="USB (HF)"
 *     source="flowgraphs/rtl_ssb_hf.grc"
 *   >{`
 *     flowchart LR
 *       A([RTL-SDR v4<br/>2.048 Msps]) --> B[SoapySDR Source]
 *       B --> C[Freq Xlating FIR<br/>decimate 64]
 *       C --> D[SSB Demod<br/>LSB/USB]
 *       D --> E[Audio Resampler<br/>→ 48 kHz]
 *       E --> F["gr-dnn · DeepFilterNet3"]
 *       F --> G[/Virtual Cable/]
 *   `}</FlowgraphRenderer>
 *
 * The MDX diagram body is authored inline today. The build-time script
 * scripts/grc-to-mermaid.mjs converts repo-level .grc files into generated
 * MDX pages under docs/_generated/flowgraphs/ which embed this component.
 */
export interface FlowgraphRendererProps {
  /** Short identifier matching the .grc filename (without extension). */
  name: string;
  /** Human-readable SDR (or audio source) this flowgraph targets. */
  sdr?: string;
  /** Mode / band description, e.g. "USB (HF)" or "NBFM (VHF)". */
  mode?: string;
  /** Relative path to the .grc file, rendered as a link. */
  source?: string;
  /** Optional subtitle / one-line description. */
  description?: string;
  /** Inline Mermaid diagram body (with or without a leading ```mermaid fence). */
  children: React.ReactNode;
}

export default function FlowgraphRenderer({
  name,
  sdr,
  mode,
  source,
  description,
  children,
}: FlowgraphRendererProps): JSX.Element {
  const raw =
    typeof children === 'string'
      ? children
      : Array.isArray(children)
      ? children.map((c) => (typeof c === 'string' ? c : '')).join('')
      : '';

  const cleaned = raw
    .replace(/^\s*```mermaid\s*/i, '')
    .replace(/\s*```\s*$/i, '')
    .trim();

  return (
    <figure className="rfw-flowgraph" aria-labelledby={`flowgraph-${name}`}>
      <header className="rfw-flowgraph__head">
        <h4 className="rfw-flowgraph__title" id={`flowgraph-${name}`}>
          {sdr ? `${sdr} · ` : ''}
          {mode ? `${mode} · ` : ''}
          <code>{name}</code>
        </h4>
        <span className="rfw-flowgraph__meta">
          {source ? (
            <a
              href={`https://github.com/jakenherman/rfwhisper/blob/main/${source}`}
              rel="noopener noreferrer"
              target="_blank">
              {source}
            </a>
          ) : (
            <em>inline</em>
          )}
        </span>
      </header>
      {description ? <p style={{ margin: '0 0 0.75rem', opacity: 0.85 }}>{description}</p> : null}
      <Mermaid value={cleaned} />
    </figure>
  );
}
