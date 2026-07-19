//! Report tests (#115): spectrogram dimensions, self-containment, section
//! presence, and the SNR-tile / reference behaviour.

use rfwhisper::report::{
    build_report, render_html, spectrogram, REPORT_F_MAX_HZ, REPORT_HOP, REPORT_N_FFT,
};

const SR: u32 = 48_000;

/// A deterministic clean tone + noise fixture, without leaning on the samples
/// module — this test exercises the report, not the generators.
fn tone(freq: f64, n: usize, amp: f64) -> Vec<f64> {
    (0..n)
        .map(|i| amp * (std::f64::consts::TAU * freq * i as f64 / SR as f64).sin())
        .collect()
}

fn noisy_fixture(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let clean = tone(1_000.0, n, 0.5);
    // A crude "denoiser": clean plus a little noise (denoised) vs clean plus a
    // lot (noisy). The report does not care how they were made.
    let mut state = 0x1234_5678u64;
    let mut rng = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
    };
    let noisy: Vec<f64> = clean.iter().map(|c| c + 0.4 * rng()).collect();
    let denoised: Vec<f64> = clean.iter().map(|c| c + 0.1 * rng()).collect();
    (clean, noisy, denoised)
}

#[test]
fn spectrogram_has_expected_dimensions() {
    let n = SR as usize * 2; // 2 s
    let spec = spectrogram(&tone(1_000.0, n, 0.5), SR).unwrap();

    // One column per hop that fits a full window.
    let expected_time = 1 + (n - REPORT_N_FFT) / REPORT_HOP;
    assert_eq!(spec.n_time, expected_time);

    // Freq bins run 0..=REPORT_F_MAX_HZ at sr/n_fft spacing.
    let bin_hz = SR as f64 / REPORT_N_FFT as f64;
    let expected_freq = (REPORT_F_MAX_HZ / bin_hz).floor() as usize + 1;
    assert_eq!(spec.n_freq, expected_freq);
    assert_eq!(spec.db.len(), spec.n_time * spec.n_freq);
    assert!(spec.top_hz <= REPORT_F_MAX_HZ);
    assert!(spec.top_hz > REPORT_F_MAX_HZ - bin_hz);
}

#[test]
fn spectrogram_locates_a_pure_tone() {
    let spec = spectrogram(&tone(1_000.0, SR as usize, 0.5), SR).unwrap();
    let bin_hz = SR as f64 / REPORT_N_FFT as f64;
    let tone_bin = (1_000.0 / bin_hz).round() as usize;
    // The tone's bin should be the loudest in a mid-clip frame by a wide margin.
    let t = spec.n_time / 2;
    let row = &spec.db[t * spec.n_freq..(t + 1) * spec.n_freq];
    let peak_bin = row
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap()
        .0;
    assert!(
        peak_bin.abs_diff(tone_bin) <= 1,
        "peak at bin {peak_bin}, tone at {tone_bin}"
    );
}

#[test]
fn spectrogram_rejects_too_short_input() {
    assert!(spectrogram(&tone(1_000.0, REPORT_N_FFT - 1, 0.5), SR).is_err());
    // Exactly one window is the minimum that works.
    assert!(spectrogram(&tone(1_000.0, REPORT_N_FFT, 0.5), SR).is_ok());
}

#[test]
fn report_with_reference_has_all_sections() {
    let (clean, noisy, denoised) = noisy_fixture(SR as usize * 2);
    let data = build_report("spectral_stub", &noisy, &denoised, Some(&clean), SR).unwrap();
    let html = render_html(&data);

    // The three panels and the median chart.
    for marker in ["cv-noisy", "cv-denoised", "cv-clean", "cv-median"] {
        assert!(html.contains(marker), "missing canvas {marker}");
    }
    // SNR tiles appear only with a reference.
    for marker in ["SNR before", "SNR after", "Δ gain"] {
        assert!(html.contains(marker), "missing tile {marker}");
    }
    assert!(html.contains("Median spectrum"));

    // Gain should be positive here: denoised is much closer to clean than noisy.
    let snr = data.snr.unwrap();
    assert!(
        snr.gain_db > 3.0,
        "expected clear gain on this fixture, got {}",
        snr.gain_db
    );
}

#[test]
fn report_without_reference_omits_snr_but_keeps_spectrograms() {
    let (_, noisy, denoised) = noisy_fixture(SR as usize * 2);
    let data = build_report("spectral_stub", &noisy, &denoised, None, SR).unwrap();
    assert!(data.snr.is_none());
    assert!(data.clean.is_none());
    let html = render_html(&data);

    // The canvas *elements* — `id=cv-*` — distinguish a real panel from the
    // unconditional `drawSpec('cv-clean', …)` reference in the script.
    assert!(html.contains("id=cv-noisy") && html.contains("id=cv-denoised"));
    assert!(
        !html.contains("id=cv-clean"),
        "no clean panel without a reference"
    );
    assert!(
        !html.contains("SNR before"),
        "no SNR tiles without a reference"
    );
    assert!(html.contains("No <code>--reference</code>"));
}

/// The one hard promise: the file must open offline. No http(s) asset anywhere.
#[test]
fn report_is_fully_self_contained() {
    let (clean, noisy, denoised) = noisy_fixture(SR as usize * 2);
    let data = build_report("spectral_stub", &noisy, &denoised, Some(&clean), SR).unwrap();
    let html = render_html(&data);
    assert!(
        !html.contains("http://") && !html.contains("https://"),
        "report references an external URL and is not self-contained"
    );
    assert!(
        !html.contains("src=\""),
        "report pulls an external resource"
    );
}

/// The embedded data must carry one median value per frequency bin, and the
/// base64 spectrogram must decode to exactly n_time * n_freq bytes.
#[test]
fn embedded_data_matches_spectrogram_size() {
    let (clean, noisy, denoised) = noisy_fixture(SR as usize * 2);
    let data = build_report("spectral_stub", &noisy, &denoised, Some(&clean), SR).unwrap();
    let n_freq = data.noisy.n_freq;
    let n_time = data.noisy.n_time;
    let html = render_html(&data);

    // medianHz should list every frequency bin.
    let hz = extract_js_array(&html, "medianHz:");
    assert_eq!(hz.len(), n_freq, "medianHz has wrong bin count");

    // The base64 for the noisy spectrogram decodes to n_time * n_freq bytes.
    let b64 = extract_json_string(&html, "noisy:{").expect("noisy spectrogram data");
    let decoded_len = base64_len(&b64);
    assert_eq!(
        decoded_len,
        n_time * n_freq,
        "noisy data has wrong byte count"
    );
}

#[test]
fn html_escapes_the_model_name() {
    let (clean, noisy, denoised) = noisy_fixture(SR as usize * 2);
    let data = build_report("<script>evil</script>", &noisy, &denoised, Some(&clean), SR).unwrap();
    let html = render_html(&data);
    assert!(!html.contains("<script>evil"), "model name was not escaped");
    assert!(html.contains("&lt;script&gt;evil"));
}

// --- tiny parsers for the assertions above ------------------------------------

/// Count the comma-separated entries of the JS array that follows `key`.
fn extract_js_array(html: &str, key: &str) -> Vec<String> {
    let start = html.find(key).expect("key present") + key.len();
    let rest = &html[start..];
    let open = rest.find('[').unwrap();
    let close = rest[open..].find(']').unwrap() + open;
    let body = &rest[open + 1..close];
    if body.is_empty() {
        Vec::new()
    } else {
        body.split(',').map(str::to_string).collect()
    }
}

/// Pull the `data:"..."` base64 string from the object that follows `key`.
fn extract_json_string(html: &str, key: &str) -> Option<String> {
    let start = html.find(key)? + key.len();
    let rest = &html[start..];
    let marker = "data:\"";
    let ds = rest.find(marker)? + marker.len();
    let de = rest[ds..].find('"')? + ds;
    Some(rest[ds..de].to_string())
}

/// Decoded byte length of a standard-base64 string (accounts for `=` padding).
fn base64_len(s: &str) -> usize {
    let pad = s.chars().rev().take_while(|c| *c == '=').count();
    s.len() / 4 * 3 - pad
}
