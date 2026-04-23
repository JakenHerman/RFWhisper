import React from 'react';

/**
 * Hand-drawn / analog feel spectrum: messy raised noise floor + cleaner trace.
 * Pure SVG — no Mermaid, no terminal chrome.
 */
export function SpectrumPanel(): JSX.Element {
  return (
    <div className="rfw-landing__spectrum" aria-hidden>
      <div className="rfw-landing__spectrum-legend">
        <span className="rfw-landing__legend rfw-landing__legend--qrm">RF noise (QRM / QRN)</span>
        <span className="rfw-landing__legend rfw-landing__legend--ok">After RFWhisper</span>
      </div>
      <svg viewBox="0 0 420 120" className="rfw-landing__svg" role="img">
        <title>Stylized audio spectrum: reduced broadband noise, speech band preserved</title>
        {/* grid */}
        <g opacity="0.35" stroke="currentColor" strokeWidth="0.5" className="rfw-landing__grid">
          {[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((i) => (
            <line key={i} x1={i * 42} y1="0" x2={i * 42} y2="120" />
          ))}
          {[0, 30, 60, 90, 120].map((y) => (
            <line key={y} x1="0" y1={y} x2="420" y2={y} />
          ))}
        </g>
        {/* “dirty” wideband hump + narrow carriers */}
        <path
          fill="var(--rfw-landing-spectrum-messy)"
          fillOpacity="0.45"
          d="M0,95 C40,70 60,75 100,50 C130,32 150,40 200,45 C250,50 300,30 360,40 C380,45 400,50 420,55 L420,120 L0,120 Z"
        />
        <path
          stroke="var(--rfw-landing-spectrum-messy)"
          strokeWidth="1.2"
          fill="none"
          d="M0,95 C40,70 60,75 100,50 C130,32 150,40 200,45 C250,50 300,30 360,40 C380,45 400,50 420,55"
        />
        {/* narrow spikes (birdies) */}
        {[110, 190, 280].map((x) => (
          <line
            key={x}
            x1={x}
            y1="115"
            x2={x}
            y2="38"
            stroke="var(--rfw-landing-birdie)"
            strokeWidth="2"
            opacity="0.6"
          />
        ))}
        {/* “clean” curve — formants-ish preserved */}
        <path
          fill="var(--rfw-landing-spectrum-clean)"
          fillOpacity="0.5"
          d="M0,100 C50,100 80,90 120,75 C150,64 200,60 250,70 C300,80 360,75 420,78 L420,120 L0,120 Z"
        />
        <path
          stroke="var(--rfw-landing-spectrum-clean)"
          strokeWidth="1.4"
          fill="none"
          d="M0,100 C50,100 80,90 120,75 C150,64 200,60 250,70 C300,80 360,75 420,78"
        />
        {/* SSB-ish passband hint */}
        <rect
          x="90"
          y="8"
          width="200"
          height="8"
          rx="2"
          fill="var(--rfw-landing-passband)"
          fillOpacity="0.35"
        />
        <text
          x="190"
          y="14.5"
          textAnchor="middle"
          className="rfw-landing__passband-label"
          fontSize="6"
        >
          ~300–3 kHz (voice) · modes vary
        </text>
      </svg>
    </div>
  );
}
