# Sample attribution

Provenance and license for every real clip in `samples/`. One row per committed
`.wav`. A clip without a row here (and a matching `<clip>.meta.json` sidecar) does
not get merged — see [README.md](./README.md#licensing-and-consent--read-before-committing-a-qso).

Synthetic fixtures (`rfwhisper samples synth`) are generated from code and are not
listed here.

| File | Mode | Band | Source | License | Contributor | Captured (UTC) | Notes |
|------|------|------|--------|---------|-------------|----------------|-------|
| _(template — copy this row)_ | ssb | 40m | own-transmission \| consented \| licensed | CC-BY-4.0 | CALLSIGN | 2026-07-19 | one line |

<!--
Example of a filled row once a real clip lands:

| onair/ssb_40m_k0abc_01.wav | ssb | 40m | own-transmission | CC-BY-4.0 | N0XYZ | 2026-07-19 | weak DX under powerline buzz; A/B candidate |

Rules:
- `Source` must be one of: own-transmission, consented (third party gave OK),
  licensed (clip carries a redistributable license from its origin).
- `License` must be redistributable (CC-BY-4.0 or CC0-1.0 recommended).
- Every row here has a matching `<clip>.meta.json` sidecar with the full metadata.
-->
