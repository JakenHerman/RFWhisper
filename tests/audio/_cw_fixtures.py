"""Synthetic CW fixtures for the A3 keying-transient gate.

Generated on demand rather than committed: the generators are deterministic, so a
fixture cannot drift between runs, and the repo stays free of binary test audio.

Timing follows the PARIS standard — a dit is ``1.2 / wpm`` seconds, a dah is three dits,
elements within a character are separated by one dit, characters by three, words by
seven. Keying uses a raised-cosine envelope, which is what a click-free transmitter
actually puts on the air; a hard rectangular key would splatter and would make the gate
measure an artefact no real signal has.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from numpy.typing import NDArray

from rfwhisper.samples.synth import atmospheric_qrn, mix

# ROADMAP testable-success example 3: 25 WPM at 600 Hz.
DEFAULT_WPM: int = 25
DEFAULT_TONE_HZ: float = 600.0
DEFAULT_SR: int = 48_000

# Raised-cosine key envelope. 4 ms is the usual click-free compromise; it sits just
# inside the 5 ms A3 measurement window, so the gate looks at the ramp itself.
RISE_MS: float = 4.0

MORSE: dict[str, str] = {
    "A": ".-", "B": "-...", "C": "-.-.", "D": "-..", "E": ".", "F": "..-.",
    "G": "--.", "H": "....", "I": "..", "J": ".---", "K": "-.-", "L": ".-..",
    "M": "--", "N": "-.", "O": "---", "P": ".--.", "Q": "--.-", "R": ".-.",
    "S": "...", "T": "-", "U": "..-", "V": "...-", "W": ".--", "X": "-..-",
    "Y": "-.--", "Z": "--..",
    "0": "-----", "1": ".----", "2": "..---", "3": "...--", "4": "....-",
    "5": ".....", "6": "-....", "7": "--...", "8": "---..", "9": "----.",
    "/": "-..-.", "?": "..--..", ",": "--..--", ".": ".-.-.-",
}  # fmt: skip

DEFAULT_MESSAGE: str = "CQ CQ DE W1AW K"


@dataclass(frozen=True)
class CWClip:
    """A CW test clip and everything a gate needs to measure it.

    ``clean`` is the noise-free keyed tone; ``noisy`` is that same signal plus noise at
    the requested SNR, sample-aligned with it. ``onsets`` holds the sample index of every
    key-down — known exactly because we generated the keying, which is the whole reason
    A3 can measure onset RMS without having to detect edges.
    """

    clean: NDArray[np.float64]
    noisy: NDArray[np.float64]
    onsets: NDArray[np.int64]
    sr: int
    wpm: int
    message: str

    @property
    def dit_samples(self) -> int:
        """Length of one dit in samples."""
        return int(round(1.2 / self.wpm * self.sr))


def _elements(message: str) -> list[tuple[bool, int]]:
    """Expand a message into ``(keyed, duration_in_dits)`` pairs.

    Trailing inter-element gaps are emitted between symbols only, so the clip never
    starts or ends with dead air it did not ask for.
    """
    out: list[tuple[bool, int]] = []
    words = message.upper().split()
    for w_i, word in enumerate(words):
        if w_i:
            out.append((False, 7))
        for c_i, char in enumerate(word):
            if char not in MORSE:
                raise ValueError(f"no morse mapping for {char!r} in message {message!r}")
            if c_i:
                out.append((False, 3))
            for s_i, symbol in enumerate(MORSE[char]):
                if s_i:
                    out.append((False, 1))
                out.append((True, 3 if symbol == "-" else 1))
    return out


def _key_envelope(n: int, rise: int) -> NDArray[np.float64]:
    """Raised-cosine rise and fall around a flat top, total length ``n``."""
    env = np.ones(n, dtype=np.float64)
    rise = min(rise, n // 2)
    if rise <= 0:
        return env
    ramp = 0.5 * (1.0 - np.cos(np.pi * np.arange(rise, dtype=np.float64) / rise))
    env[:rise] = ramp
    env[n - rise :] = ramp[::-1]
    return env


def synth_cw(
    wpm: int = DEFAULT_WPM,
    freq_hz: float = DEFAULT_TONE_HZ,
    duration_s: float | None = None,
    snr_db: float = 5.0,
    qrn: bool = False,
    message: str = DEFAULT_MESSAGE,
    sr: int = DEFAULT_SR,
    seed: int = 42,
) -> CWClip:
    """Synthesise a keyed CW clip plus a noisy copy at ``snr_db``.

    ``duration_s`` pads or truncates the clip; ``None`` uses exactly the length the
    message needs. With ``qrn=True`` the noise is Poisson-timed static crashes
    (:func:`rfwhisper.samples.synth.atmospheric_qrn`) rather than a flat receiver floor —
    the harder case for a denoiser, since a crash looks a lot like a key-down transient.
    """
    if wpm <= 0:
        raise ValueError("wpm must be positive")
    if freq_hz <= 0.0 or freq_hz >= sr / 2.0:
        raise ValueError(f"freq_hz must be in (0, {sr / 2.0}), got {freq_hz}")
    if sr <= 0:
        raise ValueError("sr must be positive")

    dit = int(round(1.2 / wpm * sr))
    rise = int(round(RISE_MS * sr / 1000.0))
    elements = _elements(message)
    total = sum(dit * dits for _, dits in elements)

    keying = np.zeros(total, dtype=np.float64)
    onsets: list[int] = []
    cursor = 0
    for keyed, dits in elements:
        length = dit * dits
        if keyed:
            onsets.append(cursor)
            keying[cursor : cursor + length] = _key_envelope(length, rise)
        cursor += length

    if duration_s is not None:
        want = int(round(duration_s * sr))
        if want <= 0:
            raise ValueError("duration_s must be positive")
        if want <= total:
            keying = keying[:want]
            # Drop any element the truncation cut short — a partial dit would give a
            # measurement window that runs past the end of the clip.
            onsets = [o for o in onsets if o + dit <= want]
        else:
            keying = np.concatenate([keying, np.zeros(want - total, dtype=np.float64)])
        total = keying.size

    t = np.arange(total, dtype=np.float64) / sr
    clean = keying * np.sin(2.0 * np.pi * freq_hz * t)

    duration = total / sr
    if qrn:
        noise = atmospheric_qrn(sr, duration, crash_rate_hz=3.0, seed=seed)
    else:
        rng = np.random.default_rng(seed)
        noise = rng.standard_normal(total)
    noisy = mix(clean, noise[:total], snr_db)

    return CWClip(
        clean=clean,
        noisy=noisy,
        onsets=np.asarray(onsets, dtype=np.int64),
        sr=sr,
        wpm=wpm,
        message=message,
    )
