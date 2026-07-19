//! Self-contained before/after HTML report for `rfwhisper denoise` (#115).
//!
//! Fills in the `spectrogram_path` the #16 report schema has always reported as
//! `null`. One offline HTML file per denoise run — inline CSS and JS, all data
//! embedded, no external asset it could fail to load — so any PR's testing
//! criteria, and the docs/website before-after imagery, can show the comparison
//! without a plotting toolchain.
//!
//! Three panels, matching the format prototyped for the maintainer:
//!
//! 1. SNR before / after / Δ stat tiles (only with a clean `--reference`)
//! 2. Side-by-side noisy vs denoised spectrograms **on one shared dB scale** —
//!    the whole point is that the two are directly comparable by eye
//! 3. A median-spectrum line chart (noisy / denoised / clean reference) showing
//!    *where in frequency* the denoiser is working
//!
//! The spectrogram is a 2048-pt FFT with a 512-sample hop, displayed 0–4 kHz —
//! the band where SSB/CW/FT8 energy lives. Magnitudes are quantized to one byte
//! over a shared dB window and base64-embedded, so a six-second clip is tens of
//! kilobytes rather than a megabyte of JSON.

use std::path::Path;

use realfft::num_complex::Complex;
use realfft::RealFftPlanner;

use crate::dsp::features::hann_window;
use crate::dsp::metrics::matched_filter_snr_db;
use crate::dsp::DspError;

/// FFT size for the report STFT (issue #115 design target).
pub const REPORT_N_FFT: usize = 2048;
/// Hop between report STFT frames.
pub const REPORT_HOP: usize = 512;
/// Top of the displayed frequency band, in Hz.
pub const REPORT_F_MAX_HZ: f64 = 4_000.0;
/// Displayed dynamic range below the pair's peak, in dB. Cells quieter than this
/// clamp to the floor color, which keeps the shared scale from being dominated
/// by a single loud bin.
pub const REPORT_DYNAMIC_RANGE_DB: f64 = 80.0;

/// A magnitude spectrogram in dB, row-major `[time * n_freq + freq]`.
#[derive(Debug, Clone)]
pub struct Spectrogram {
    pub n_time: usize,
    pub n_freq: usize,
    /// Hz of the highest retained bin (`freq index n_freq - 1`).
    pub top_hz: f64,
    /// dB magnitude per (time, freq), unclamped.
    pub db: Vec<f64>,
}

impl Spectrogram {
    /// Median dB per frequency bin across all time frames.
    ///
    /// The median rather than the mean, so a handful of loud transient frames
    /// (a static crash, a CW key-down) do not drag the "typical" spectrum up.
    fn median_spectrum(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.n_freq];
        let mut col = vec![0.0; self.n_time];
        for (f, o) in out.iter_mut().enumerate() {
            for (t, c) in col.iter_mut().enumerate() {
                *c = self.db[t * self.n_freq + f];
            }
            col.sort_by(|a, b| a.partial_cmp(b).unwrap());
            *o = if self.n_time == 0 {
                f64::NEG_INFINITY
            } else if self.n_time % 2 == 1 {
                col[self.n_time / 2]
            } else {
                0.5 * (col[self.n_time / 2 - 1] + col[self.n_time / 2])
            };
        }
        out
    }
}

/// Compute the 0–`REPORT_F_MAX_HZ` magnitude spectrogram of `x` at rate `sr`.
///
/// Errors when the clip is shorter than one analysis window — a report needs at
/// least one time column to mean anything.
pub fn spectrogram(x: &[f64], sr: u32) -> Result<Spectrogram, DspError> {
    if sr == 0 {
        return Err(DspError::new("sr must be positive"));
    }
    if x.len() < REPORT_N_FFT {
        return Err(DspError::new(format!(
            "signal is {} samples; the spectrogram needs at least {REPORT_N_FFT}",
            x.len()
        )));
    }
    let bin_hz = sr as f64 / REPORT_N_FFT as f64;
    // Inclusive top bin, capped at Nyquist for low sample rates.
    let n_freq = ((REPORT_F_MAX_HZ / bin_hz).floor() as usize + 1).min(REPORT_N_FFT / 2 + 1);
    let window = hann_window(REPORT_N_FFT)?;

    let mut planner = RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(REPORT_N_FFT);
    let mut buf = r2c.make_input_vec();
    let mut spec: Vec<Complex<f64>> = r2c.make_output_vec();

    let n_time = 1 + (x.len() - REPORT_N_FFT) / REPORT_HOP;
    let mut db = vec![0.0; n_time * n_freq];
    // 20*log10 of a magnitude normalized by the window's coherent gain, so the dB
    // numbers are comparable frame to frame regardless of FFT size.
    let norm = 2.0 / window.iter().sum::<f64>();
    for t in 0..n_time {
        let start = t * REPORT_HOP;
        for (b, (v, w)) in buf
            .iter_mut()
            .zip(x[start..start + REPORT_N_FFT].iter().zip(&window))
        {
            *b = v * w;
        }
        r2c.process(&mut buf, &mut spec).expect("realfft forward");
        for f in 0..n_freq {
            let mag = spec[f].norm() * norm;
            db[t * n_freq + f] = 20.0 * (mag + 1e-12).log10();
        }
    }
    Ok(Spectrogram {
        n_time,
        n_freq,
        top_hz: (n_freq - 1) as f64 * bin_hz,
        db,
    })
}

/// SNR of noisy and denoised audio against a clean reference (A1 stat tiles).
#[derive(Debug, Clone, Copy)]
pub struct SnrStats {
    pub before_db: f64,
    pub after_db: f64,
    pub gain_db: f64,
}

/// Everything the HTML needs, computed once from the three signals.
pub struct ReportData {
    pub model: String,
    pub sr: u32,
    pub duration_s: f64,
    pub noisy: Spectrogram,
    pub denoised: Spectrogram,
    pub clean: Option<Spectrogram>,
    pub snr: Option<SnrStats>,
}

/// Build the report from noisy input, denoised output, and an optional reference.
///
/// All three must share a sample rate and be roughly the same length (the STFT
/// simply uses each clip's own frames; SNR alignment is handled by the metrics
/// layer). `clean` present turns on the SNR tiles and the clean median series.
pub fn build_report(
    model: &str,
    noisy: &[f64],
    denoised: &[f64],
    clean: Option<&[f64]>,
    sr: u32,
) -> Result<ReportData, DspError> {
    let noisy_spec = spectrogram(noisy, sr)?;
    let denoised_spec = spectrogram(denoised, sr)?;
    let (clean_spec, snr) = match clean {
        Some(c) => {
            let before = matched_filter_snr_db(noisy, c, sr)?;
            let after = matched_filter_snr_db(denoised, c, sr)?;
            (
                Some(spectrogram(c, sr)?),
                Some(SnrStats {
                    before_db: before,
                    after_db: after,
                    gain_db: after - before,
                }),
            )
        }
        None => (None, None),
    };
    Ok(ReportData {
        model: model.to_string(),
        sr,
        duration_s: noisy.len() as f64 / sr as f64,
        noisy: noisy_spec,
        denoised: denoised_spec,
        clean: clean_spec,
        snr,
    })
}

/// Render `data` to a standalone HTML document and write it to `path`.
pub fn write_html(path: &Path, data: &ReportData) -> Result<(), DspError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DspError::new(format!("cannot create {}: {e}", parent.display())))?;
        }
    }
    std::fs::write(path, render_html(data))
        .map_err(|e| DspError::new(format!("cannot write {}: {e}", path.display())))
}

/// The shared dB window `[max - range, max]` across the noisy/denoised pair.
///
/// Taken over *both* spectrograms so the two color ramps mean the same thing —
/// comparability is the entire reason the panels sit side by side.
fn shared_db_range(a: &Spectrogram, b: &Spectrogram) -> (f64, f64) {
    let peak =
        a.db.iter()
            .chain(&b.db)
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
    let peak = if peak.is_finite() { peak } else { 0.0 };
    (peak - REPORT_DYNAMIC_RANGE_DB, peak)
}

/// Quantize a spectrogram's dB to one byte each over `[lo, hi]`.
fn quantize(spec: &Spectrogram, lo: f64, hi: f64) -> Vec<u8> {
    let span = (hi - lo).max(1e-9);
    spec.db
        .iter()
        .map(|&v| {
            let t = ((v - lo) / span).clamp(0.0, 1.0);
            (t * 255.0).round() as u8
        })
        .collect()
}

/// Standard base64 (no line breaks) — avoids a dependency for a few KB of data.
fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// A JS numeric array literal from an f64 slice; non-finite becomes `null`.
fn js_array(values: &[f64]) -> String {
    let mut s = String::from("[");
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        if v.is_finite() {
            s.push_str(&format!("{v:.2}"));
        } else {
            s.push_str("null");
        }
    }
    s.push(']');
    s
}

fn spec_js(name: &str, spec: &Spectrogram, bytes: &[u8]) -> String {
    format!(
        "{name}:{{w:{},h:{},top:{:.1},data:\"{}\"}}",
        spec.n_time,
        spec.n_freq,
        spec.top_hz,
        base64(bytes)
    )
}

/// The full HTML document as a string. Pure function of `data` — the file writer
/// and the tests both go through here.
pub fn render_html(data: &ReportData) -> String {
    let (lo, hi) = shared_db_range(&data.noisy, &data.denoised);
    let noisy_q = quantize(&data.noisy, lo, hi);
    let denoised_q = quantize(&data.denoised, lo, hi);

    let mut specs = format!(
        "{},{}",
        spec_js("noisy", &data.noisy, &noisy_q),
        spec_js("denoised", &data.denoised, &denoised_q),
    );
    let mut medians = format!(
        "medianNoisy:{},medianDenoised:{}",
        js_array(&data.noisy.median_spectrum()),
        js_array(&data.denoised.median_spectrum()),
    );
    match &data.clean {
        Some(c) => {
            let cq = quantize(c, lo, hi);
            specs.push_str(&format!(",{}", spec_js("clean", c, &cq)));
            medians.push_str(&format!(",medianClean:{}", js_array(&c.median_spectrum())));
        }
        None => medians.push_str(",medianClean:null"),
    }

    // Frequency axis for the median chart: bin centers of the noisy spectrogram.
    let bin_hz = data.sr as f64 / REPORT_N_FFT as f64;
    let median_hz: Vec<f64> = (0..data.noisy.n_freq).map(|f| f as f64 * bin_hz).collect();

    let tiles = match data.snr {
        Some(s) => format!(
            "<div class=tiles>\
             <div class=tile><div class=k>SNR before</div><div class=v>{}</div></div>\
             <div class=tile><div class=k>SNR after</div><div class=v>{}</div></div>\
             <div class=tile><div class=k>Δ gain</div><div class='v {}'>{}</div></div>\
             </div>",
            fmt_db(s.before_db),
            fmt_db(s.after_db),
            if s.gain_db >= 0.0 { "pos" } else { "neg" },
            fmt_db_signed(s.gain_db),
        ),
        None => String::from(
            "<p class=note>No <code>--reference</code> given, so SNR is not measurable — \
             the spectrograms still show what changed.</p>",
        ),
    };

    let has_clean = data.clean.is_some();
    format!(
        r#"<main>
<h1>RFWhisper denoise report</h1>
<p class=meta>model <b>{model}</b> · {sr} Hz · {dur:.1}s · shared scale {lo:.0} to {hi:.0} dB · 2048-pt FFT, {hop}-sample hop, 0–{fmax:.0} Hz</p>
{tiles}
<section class=grid>
  <figure><figcaption>Noisy (input)</figcaption><canvas id=cv-noisy></canvas></figure>
  <figure><figcaption>Denoised (output)</figcaption><canvas id=cv-denoised></canvas></figure>
  {clean_fig}
</section>
<section>
  <h2>Median spectrum</h2>
  <p class=note>Where in frequency the denoiser removes (or keeps) energy. Lower is quieter.</p>
  <canvas id=cv-median width=900 height=320></canvas>
  <div id=legend class=legend></div>
</section>
<footer>Generated offline by <code>rfwhisper denoise --spectrogram</code>. No network required.</footer>
</main>
<style>
:root{{color-scheme:light dark}}
*{{box-sizing:border-box}}
body{{margin:0}}
main{{font:15px/1.5 system-ui,sans-serif;max-width:1000px;margin:0 auto;padding:24px;color:#e8e8ea;background:#14151a}}
h1{{font-size:22px;margin:0 0 4px}}
h2{{font-size:17px;margin:28px 0 6px}}
.meta{{color:#9aa0aa;font-size:13px;margin:0 0 18px}}
.note{{color:#9aa0aa;font-size:13px;margin:4px 0 10px}}
.tiles{{display:flex;gap:12px;flex-wrap:wrap;margin:0 0 8px}}
.tile{{background:#1e2029;border:1px solid #2c2f3a;border-radius:10px;padding:12px 18px;min-width:120px}}
.tile .k{{color:#9aa0aa;font-size:12px;text-transform:uppercase;letter-spacing:.04em}}
.tile .v{{font-size:24px;font-weight:650;margin-top:2px}}
.tile .v.pos{{color:#4ec98f}}.tile .v.neg{{color:#e2777a}}
.grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:16px}}
figure{{margin:0}}
figcaption{{color:#c7ccd6;font-size:13px;margin-bottom:6px}}
canvas{{width:100%;height:auto;border:1px solid #2c2f3a;border-radius:8px;image-rendering:pixelated;background:#0c0d11}}
#cv-median{{image-rendering:auto}}
.legend{{display:flex;gap:16px;font-size:13px;color:#c7ccd6;margin-top:8px}}
.legend span{{display:inline-flex;align-items:center;gap:6px}}
.legend i{{width:14px;height:3px;border-radius:2px;display:inline-block}}
footer{{margin-top:28px;color:#6b7180;font-size:12px}}
@media (prefers-color-scheme:light){{main{{color:#1a1c22;background:#fff}}.tile{{background:#f5f6f8;border-color:#e2e4ea}}canvas{{background:#fafbfc}}.meta,.note{{color:#606672}}figcaption{{color:#333}}}}
</style>
<script>
const D={{sr:{sr},binHz:{bin_hz},hasClean:{has_clean},medianHz:{median_hz},specs:{{{specs}}},{medians}}};
// Magma-ish 8-stop ramp; enough anchors that a spectrogram reads cleanly.
const CM=[[0,0,4],[40,11,84],[101,21,110],[159,42,99],[212,72,66],[245,125,21],[250,193,39],[252,255,164]];
function color(t){{t=Math.max(0,Math.min(1,t));const x=t*(CM.length-1),i=Math.floor(x),f=x-i,a=CM[i],b=CM[Math.min(i+1,CM.length-1)];return[a[0]+(b[0]-a[0])*f,a[1]+(b[1]-a[1])*f,a[2]+(b[2]-a[2])*f]}}
function b64(s){{const bin=atob(s),u=new Uint8Array(bin.length);for(let i=0;i<bin.length;i++)u[i]=bin.charCodeAt(i);return u}}
function drawSpec(id,s){{const cv=document.getElementById(id);if(!cv)return;cv.width=s.w;cv.height=s.h;const ctx=cv.getContext('2d'),img=ctx.createImageData(s.w,s.h),px=b64(s.data);
for(let t=0;t<s.w;t++)for(let f=0;f<s.h;f++){{const v=px[t*s.h+f]/255,c=color(v),y=s.h-1-f,o=(y*s.w+t)*4;img.data[o]=c[0];img.data[o+1]=c[1];img.data[o+2]=c[2];img.data[o+3]=255}}
ctx.putImageData(img,0,0)}}
drawSpec('cv-noisy',D.specs.noisy);drawSpec('cv-denoised',D.specs.denoised);if(D.specs.clean)drawSpec('cv-clean',D.specs.clean);
// Median line chart.
(function(){{const cv=document.getElementById('cv-median');if(!cv)return;const ctx=cv.getContext('2d'),W=cv.width,H=cv.height,PL=48,PB=28,PT=10,PR=12;
const series=[['Noisy',D.medianNoisy,'#e2777a'],['Denoised',D.medianDenoised,'#4ec98f']];if(D.medianClean)series.push(['Clean ref',D.medianClean,'#7aa2f7']);
let lo=1e9,hi=-1e9;for(const[,a]of series)for(const v of a)if(v!=null){{if(v<lo)lo=v;if(v>hi)hi=v}}if(!isFinite(lo)){{lo=-80;hi=0}}lo=Math.floor(lo/10)*10;hi=Math.ceil(hi/10)*10;
const fmax=D.medianHz[D.medianHz.length-1]||4000;
const X=hz=>PL+(W-PL-PR)*(hz/fmax),Y=db=>PT+(H-PT-PB)*(1-(db-lo)/(hi-lo));
const css=getComputedStyle(document.body).color;ctx.strokeStyle=ctx.fillStyle=css;ctx.globalAlpha=.25;ctx.lineWidth=1;
for(let db=lo;db<=hi;db+=10){{ctx.beginPath();ctx.moveTo(PL,Y(db));ctx.lineTo(W-PR,Y(db));ctx.stroke()}}ctx.globalAlpha=1;
ctx.font='11px system-ui';ctx.textAlign='right';for(let db=lo;db<=hi;db+=20)ctx.fillText(db+' dB',PL-6,Y(db)+3);
ctx.textAlign='center';for(let k=0;k<=4;k++){{const hz=fmax*k/4;ctx.fillText((hz/1000).toFixed(1)+'k',X(hz),H-10)}}
for(const[,a,col]of series){{ctx.strokeStyle=col;ctx.lineWidth=2;ctx.beginPath();let started=false;for(let f=0;f<a.length;f++){{const v=a[f];if(v==null)continue;const x=X(D.medianHz[f]),y=Y(v);if(started)ctx.lineTo(x,y);else{{ctx.moveTo(x,y);started=true}}}}ctx.stroke()}}
const lg=document.getElementById('legend');lg.innerHTML=series.map(([n,,c])=>`<span><i style="background:${{c}}"></i>${{n}}</span>`).join('')}})();
</script>"#,
        model = html_escape(&data.model),
        sr = data.sr,
        dur = data.duration_s,
        hop = REPORT_HOP,
        fmax = data.noisy.top_hz,
        bin_hz = bin_hz,
        has_clean = has_clean,
        median_hz = js_array(&median_hz),
        clean_fig = if has_clean {
            "<figure><figcaption>Clean reference</figcaption><canvas id=cv-clean></canvas></figure>"
        } else {
            ""
        },
    )
}

fn fmt_db(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.1} dB")
    } else if v > 0.0 {
        "∞".into()
    } else {
        "−∞".into()
    }
}

fn fmt_db_signed(v: f64) -> String {
    if v.is_finite() {
        format!("{v:+.1} dB")
    } else {
        fmt_db(v)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
