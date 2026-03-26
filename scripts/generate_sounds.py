"""Generate warm, pleasant notification sounds for YOLO Voice.

Each sound uses sine-wave synthesis with harmonics, smooth attack/decay
envelopes, and optional reverb to create a natural, ear-friendly feel.
"""

import numpy as np
from scipy.io import wavfile
from scipy.signal import fftconvolve
import os

SAMPLE_RATE = 44100
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "sounds")


def normalize(signal: np.ndarray, peak: float = 0.85) -> np.ndarray:
    mx = np.max(np.abs(signal))
    if mx > 0:
        signal = signal / mx * peak
    return signal


def envelope(length: int, attack_ms: float = 10, decay_ms: float = 200) -> np.ndarray:
    """Smooth attack-sustain-decay envelope."""
    attack = int(SAMPLE_RATE * attack_ms / 1000)
    decay = int(SAMPLE_RATE * decay_ms / 1000)
    env = np.ones(length)
    # smooth attack (raised cosine)
    if attack > 0:
        env[:attack] = 0.5 * (1 - np.cos(np.pi * np.arange(attack) / attack))
    # smooth decay
    if decay > 0 and decay < length:
        env[-decay:] = 0.5 * (1 + np.cos(np.pi * np.arange(decay) / decay))
    return env


def add_reverb(signal: np.ndarray, decay: float = 0.3, delay_ms: float = 30) -> np.ndarray:
    """Simple convolution reverb for warmth."""
    delay_samples = int(SAMPLE_RATE * delay_ms / 1000)
    ir_length = int(SAMPLE_RATE * 0.4)  # 400ms impulse response
    ir = np.zeros(ir_length)
    ir[0] = 1.0
    # A few early reflections
    for i, (d, g) in enumerate([(1.0, 0.4), (1.7, 0.25), (2.5, 0.15), (3.8, 0.08)]):
        idx = int(delay_samples * d)
        if idx < ir_length:
            ir[idx] = decay * g
    # Exponential tail
    t = np.arange(ir_length) / SAMPLE_RATE
    ir *= np.exp(-t * 8)
    wet = fftconvolve(signal, ir, mode="full")[: len(signal)]
    return normalize(0.7 * signal + 0.3 * wet)


def tone(freq: float, duration_ms: float, harmonics: list[tuple[int, float]] | None = None,
         attack_ms: float = 10, decay_ms: float = 200) -> np.ndarray:
    """Generate a warm tone with optional harmonics."""
    n_samples = int(SAMPLE_RATE * duration_ms / 1000)
    t = np.arange(n_samples) / SAMPLE_RATE
    signal = np.sin(2 * np.pi * freq * t)
    if harmonics:
        for mult, amp in harmonics:
            signal += amp * np.sin(2 * np.pi * freq * mult * t)
    signal *= envelope(n_samples, attack_ms, decay_ms)
    return signal


def save(name: str, signal: np.ndarray):
    signal = normalize(signal)
    data = (signal * 32767).astype(np.int16)
    path = os.path.join(OUTPUT_DIR, f"{name}.wav")
    wavfile.write(path, SAMPLE_RATE, data)
    print(f"  wrote {name}.wav ({len(data) / SAMPLE_RATE:.2f}s)")


# ---------------------------------------------------------------------------
# Sound definitions — warm, gentle, ear-friendly
# ---------------------------------------------------------------------------

def gen_chime():
    """Two-note ascending major third — warm and inviting."""
    harmonics = [(2, 0.3), (3, 0.1), (5, 0.03)]
    note1 = tone(523.25, 200, harmonics, attack_ms=8, decay_ms=150)   # C5
    gap = np.zeros(int(SAMPLE_RATE * 0.06))
    note2 = tone(659.25, 350, harmonics, attack_ms=8, decay_ms=280)   # E5
    return add_reverb(np.concatenate([note1, gap, note2]))


def gen_pop():
    """Soft bubble pop — quick and satisfying."""
    n = int(SAMPLE_RATE * 0.12)
    t = np.arange(n) / SAMPLE_RATE
    # Frequency sweep down (pop character)
    freq = 800 * np.exp(-t * 20)
    signal = np.sin(2 * np.pi * np.cumsum(freq) / SAMPLE_RATE)
    signal *= envelope(n, attack_ms=2, decay_ms=80)
    return add_reverb(signal, decay=0.2)


def gen_bell():
    """Warm tubular bell — rich harmonics with long decay."""
    harmonics = [(2, 0.5), (3, 0.25), (4.2, 0.12), (5.4, 0.06)]
    return add_reverb(tone(440, 600, harmonics, attack_ms=5, decay_ms=500))


def gen_ding():
    """Single warm ding — clean and pleasant completion sound."""
    harmonics = [(2, 0.2), (3, 0.08), (4, 0.03)]
    return add_reverb(tone(880, 400, harmonics, attack_ms=5, decay_ms=350))


def gen_click():
    """Soft tactile click — minimal and unobtrusive."""
    n = int(SAMPLE_RATE * 0.05)
    t = np.arange(n) / SAMPLE_RATE
    signal = np.sin(2 * np.pi * 1200 * t) * np.exp(-t * 80)
    signal += 0.3 * np.sin(2 * np.pi * 600 * t) * np.exp(-t * 60)
    return signal


def gen_whoosh():
    """Gentle whoosh — filtered noise sweep."""
    n = int(SAMPLE_RATE * 0.3)
    t = np.arange(n) / SAMPLE_RATE
    noise = np.random.randn(n) * 0.5
    # Bandpass sweep using amplitude modulation
    center_freq = 400 + 800 * t / t[-1]
    carrier = np.sin(2 * np.pi * np.cumsum(center_freq) / SAMPLE_RATE)
    signal = noise * carrier * envelope(n, attack_ms=30, decay_ms=200)
    return add_reverb(signal, decay=0.15)


def gen_bubble():
    """Playful water bubble — two overlapping frequency sweeps."""
    n = int(SAMPLE_RATE * 0.18)
    t = np.arange(n) / SAMPLE_RATE
    f1 = 600 + 400 * np.exp(-t * 12)
    f2 = 900 + 300 * np.exp(-t * 15)
    s1 = np.sin(2 * np.pi * np.cumsum(f1) / SAMPLE_RATE) * np.exp(-t * 10)
    s2 = 0.5 * np.sin(2 * np.pi * np.cumsum(f2) / SAMPLE_RATE) * np.exp(-t * 14)
    signal = (s1 + s2) * envelope(n, attack_ms=3, decay_ms=120)
    return add_reverb(signal, decay=0.25)


def gen_tap():
    """Soft wooden tap — brief and natural."""
    n = int(SAMPLE_RATE * 0.06)
    t = np.arange(n) / SAMPLE_RATE
    # Multiple resonances for wood-like character
    signal = (np.sin(2 * np.pi * 800 * t) * np.exp(-t * 50) +
              0.6 * np.sin(2 * np.pi * 1600 * t) * np.exp(-t * 70) +
              0.3 * np.sin(2 * np.pi * 3200 * t) * np.exp(-t * 90))
    return signal


def gen_gentle():
    """Gentle harp-like pluck — soothing two-note motif."""
    harmonics = [(2, 0.4), (3, 0.15), (4, 0.05)]
    note1 = tone(392, 300, harmonics, attack_ms=5, decay_ms=250)     # G4
    gap = np.zeros(int(SAMPLE_RATE * 0.04))
    note2 = tone(523.25, 400, harmonics, attack_ms=5, decay_ms=350)  # C5
    return add_reverb(np.concatenate([note1, gap, note2]), decay=0.35)


def gen_bright():
    """Bright sparkle — ascending arpeggio, three quick notes."""
    harmonics = [(2, 0.15), (3, 0.05)]
    notes = []
    for freq in [784, 988, 1175]:  # G5 B5 D6
        notes.append(tone(freq, 120, harmonics, attack_ms=5, decay_ms=100))
        notes.append(np.zeros(int(SAMPLE_RATE * 0.03)))
    # Longer tail on last note
    notes[-2] = tone(1175, 250, harmonics, attack_ms=5, decay_ms=220)
    return add_reverb(np.concatenate(notes))


def gen_classic_start():
    """Classic start — warm ascending perfect fifth."""
    harmonics = [(2, 0.25), (3, 0.1)]
    note1 = tone(440, 180, harmonics, attack_ms=8, decay_ms=140)    # A4
    gap = np.zeros(int(SAMPLE_RATE * 0.04))
    note2 = tone(659.25, 300, harmonics, attack_ms=8, decay_ms=250) # E5
    return add_reverb(np.concatenate([note1, gap, note2]))


def gen_classic_done():
    """Classic done — descending perfect fourth, signals completion."""
    harmonics = [(2, 0.25), (3, 0.1)]
    note1 = tone(659.25, 180, harmonics, attack_ms=8, decay_ms=140) # E5
    gap = np.zeros(int(SAMPLE_RATE * 0.04))
    note2 = tone(494, 350, harmonics, attack_ms=8, decay_ms=300)    # B4
    return add_reverb(np.concatenate([note1, gap, note2]))


def gen_done():
    """Done — satisfying two-tone completion chord."""
    harmonics = [(2, 0.2), (3, 0.08)]
    note1 = tone(587.33, 200, harmonics, attack_ms=8, decay_ms=160) # D5
    gap = np.zeros(int(SAMPLE_RATE * 0.05))
    note2 = tone(784, 400, harmonics, attack_ms=8, decay_ms=350)    # G5
    return add_reverb(np.concatenate([note1, gap, note2]))


def gen_start():
    """Start — quick upward notification, ready to go."""
    harmonics = [(2, 0.2), (3, 0.08)]
    note1 = tone(523.25, 140, harmonics, attack_ms=5, decay_ms=110) # C5
    gap = np.zeros(int(SAMPLE_RATE * 0.03))
    note2 = tone(698.46, 200, harmonics, attack_ms=5, decay_ms=170) # F5
    return add_reverb(np.concatenate([note1, gap, note2]))


if __name__ == "__main__":
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    print("Generating warm notification sounds...")
    generators = {
        "chime": gen_chime,
        "pop": gen_pop,
        "bell": gen_bell,
        "ding": gen_ding,
        "click": gen_click,
        "whoosh": gen_whoosh,
        "bubble": gen_bubble,
        "tap": gen_tap,
        "gentle": gen_gentle,
        "bright": gen_bright,
        "classic_start": gen_classic_start,
        "classic_done": gen_classic_done,
        "done": gen_done,
        "start": gen_start,
    }
    for name, gen in generators.items():
        save(name, gen())
    print("Done!")
