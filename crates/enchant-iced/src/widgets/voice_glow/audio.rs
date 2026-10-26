//! Level sources and audio analysis.
//!
//! Upstream reads an `AnalyserNode` on a `MediaStream`; here the same
//! measurement runs on a PCM buffer you feed ([`PcmStream`]), so any audio
//! backend works. The analyser reproduces the Web Audio one: a 1024-point
//! FFT over a Blackman window, 0.5 smoothing between reads, magnitudes
//! mapped to bytes over -100..-30 dB, then the driver's band averages
//! (`readAnalyser` in `voiceDriver.ts`, `acquireAnalyser` in `audio.ts`).

use std::collections::VecDeque;
use std::f64::consts::PI;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

/// Gain applied before the user's `sensitivity`: a laptop microphone at
/// conversational distance gives an RMS of roughly 0.03-0.2, which this
/// lifts into the 0.15-1 range the shaping curve expects.
pub const BASE_GAIN: f64 = 5.0;
/// Gain on the per-band spectrum average.
pub const BAND_GAIN: f64 = 1.7;
/// Voice bands in Hz: fundamentals and chest, vowels and presence, sibilance.
pub const BANDS_HZ: [(f64, f64); 3] = [(80.0, 300.0), (300.0, 2000.0), (2000.0, 6000.0)];
/// The analyser's FFT size (`analyser.fftSize`).
pub const FFT_SIZE: usize = 1024;
/// The analyser's smoothing constant (`smoothingTimeConstant`).
pub const SMOOTHING: f64 = 0.5;

const MIN_DECIBELS: f64 = -100.0;
const MAX_DECIBELS: f64 = -30.0;
/// Samples kept per stream: enough for several analysis windows.
const RING_CAPACITY: usize = 8 * FFT_SIZE;

#[derive(Debug)]
struct PcmInner {
    samples: VecDeque<f32>,
    sample_rate: f64,
}

/// A shared, thread-safe PCM tap: the audio thread pushes mono samples, the
/// glow reads the most recent window each frame. Cloning shares the buffer.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::PcmStream;
///
/// let stream = PcmStream::new(48_000.0);
/// stream.push(&[0.1, -0.1, 0.1, -0.1]);
/// assert_eq!(stream.sample_rate(), 48_000.0);
/// ```
#[derive(Clone, Debug)]
pub struct PcmStream {
    inner: Arc<Mutex<PcmInner>>,
}

impl PcmStream {
    /// An empty stream at `sample_rate` Hz (non-positive or non-finite rates
    /// fall back to 48 kHz).
    #[must_use]
    pub fn new(sample_rate: f64) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            48_000.0
        };
        Self {
            inner: Arc::new(Mutex::new(PcmInner {
                samples: VecDeque::with_capacity(RING_CAPACITY),
                sample_rate,
            })),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PcmInner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Appends mono samples (`-1.0..=1.0`); the oldest are dropped past the
    /// ring's capacity. Non-finite samples are stored as silence.
    pub fn push(&self, samples: &[f32]) {
        let mut inner = self.lock();
        for &s in samples {
            if inner.samples.len() == RING_CAPACITY {
                inner.samples.pop_front();
            }
            inner.samples.push_back(if s.is_finite() { s } else { 0.0 });
        }
    }

    /// The stream's sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.lock().sample_rate
    }

    /// Copies the most recent `out.len()` samples, oldest first; a shorter
    /// history is left-padded with silence.
    pub fn latest(&self, out: &mut [f32]) {
        let inner = self.lock();
        let have = inner.samples.len().min(out.len());
        let pad = out.len() - have;
        out[..pad].fill(0.0);
        let skip = inner.samples.len() - have;
        for (slot, s) in out[pad..].iter_mut().zip(inner.samples.iter().skip(skip)) {
            *slot = *s;
        }
    }
}

impl PartialEq for PcmStream {
    /// Two handles are equal when they share one buffer.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

type LevelFn = dyn Fn() -> f64 + Send + Sync;

/// Where the glow's level comes from.
///
/// Upstream: the `level` prop (a number or a getter) and the `stream` prop,
/// which wins over `level` when set.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::LevelSource;
///
/// let constant = LevelSource::constant(0.5);
/// let meter = LevelSource::getter(|| 0.25);
/// assert_ne!(constant, meter);
/// ```
#[non_exhaustive]
#[derive(Clone)]
pub enum LevelSource {
    /// A fixed level, 0-1.
    Constant(f64),
    /// A getter sampled once per frame (a meter, a volume slider).
    Getter(Arc<LevelFn>),
    /// A PCM stream: level and the low / mid / high bands come from analysis.
    Pcm(PcmStream),
}

impl LevelSource {
    /// A fixed level.
    #[must_use]
    pub fn constant(level: f64) -> Self {
        Self::Constant(level)
    }

    /// A per-frame getter.
    #[must_use]
    pub fn getter(f: impl Fn() -> f64 + Send + Sync + 'static) -> Self {
        Self::Getter(Arc::new(f))
    }

    /// An analysed PCM stream.
    #[must_use]
    pub fn pcm(stream: PcmStream) -> Self {
        Self::Pcm(stream)
    }
}

impl Default for LevelSource {
    /// Silence (`level = 0`).
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl fmt::Debug for LevelSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(v) => f.debug_tuple("Constant").field(v).finish(),
            Self::Getter(_) => f.write_str("Getter(..)"),
            Self::Pcm(s) => f.debug_tuple("Pcm").field(s).finish(),
        }
    }
}

impl PartialEq for LevelSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Constant(a), Self::Constant(b)) => a == b,
            (Self::Getter(a), Self::Getter(b)) => Arc::ptr_eq(a, b),
            (Self::Pcm(a), Self::Pcm(b)) => a == b,
            _ => false,
        }
    }
}

impl From<f64> for LevelSource {
    fn from(level: f64) -> Self {
        Self::Constant(level)
    }
}

impl From<PcmStream> for LevelSource {
    fn from(stream: PcmStream) -> Self {
        Self::Pcm(stream)
    }
}

/// One analysis read: the raw (pre-shaping) level and the three band
/// energies, already scaled by gain and sensitivity.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reading {
    /// RMS of the window times [`BASE_GAIN`] times sensitivity.
    pub level: f64,
    /// Low / mid / high band averages times [`BAND_GAIN`] times sensitivity.
    pub bands: [f64; 3],
}

/// The Web Audio `AnalyserNode`, reproduced (`fftSize` 1024, smoothing 0.5,
/// Blackman window, -100..-30 dB byte mapping).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::{PcmStream, SpectrumAnalyser};
///
/// let stream = PcmStream::new(48_000.0);
/// let tone: Vec<f32> = (0..2048)
///     .map(|n| (n as f32 * 2.0 * std::f32::consts::PI * 150.0 / 48_000.0).sin() * 0.2)
///     .collect();
/// stream.push(&tone);
/// let mut analyser = SpectrumAnalyser::new();
/// let reading = analyser.read(&stream, 3.1);
/// assert!(reading.bands[0] > reading.bands[2]);
/// ```
#[derive(Clone, Debug)]
pub struct SpectrumAnalyser {
    window: Vec<f64>,
    smoothed: Vec<f64>,
    time: Vec<f32>,
    re: Vec<f64>,
    im: Vec<f64>,
}

impl Default for SpectrumAnalyser {
    fn default() -> Self {
        Self::new()
    }
}

impl SpectrumAnalyser {
    /// A fresh analyser with an empty smoothing history.
    #[must_use]
    pub fn new() -> Self {
        let alpha = 0.16;
        let (a0, a1, a2) = ((1.0 - alpha) / 2.0, 0.5, alpha / 2.0);
        let n = FFT_SIZE as f64;
        let window = (0..FFT_SIZE)
            .map(|i| {
                let x = i as f64 / n;
                a0 - a1 * (2.0 * PI * x).cos() + a2 * (4.0 * PI * x).cos()
            })
            .collect();
        Self {
            window,
            smoothed: vec![0.0; FFT_SIZE / 2],
            time: vec![0.0; FFT_SIZE],
            re: vec![0.0; FFT_SIZE],
            im: vec![0.0; FFT_SIZE],
        }
    }

    /// Forgets the smoothing history (a re-registration upstream).
    pub fn reset(&mut self) {
        self.smoothed.fill(0.0);
    }

    /// Reads the latest window of `stream`: level from the time domain,
    /// bands from the smoothed byte spectrum (`readAnalyser`).
    pub fn read(&mut self, stream: &PcmStream, sensitivity: f64) -> Reading {
        stream.latest(&mut self.time);
        let sum: f64 = self.time.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
        let level = (sum / FFT_SIZE as f64).sqrt() * BASE_GAIN * sensitivity;

        for i in 0..FFT_SIZE {
            self.re[i] = f64::from(self.time[i]) * self.window[i];
            self.im[i] = 0.0;
        }
        fft(&mut self.re, &mut self.im);
        let n = FFT_SIZE as f64;
        let bytes: Vec<f64> = (0..FFT_SIZE / 2)
            .map(|k| {
                let mag = (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt() / n;
                let s = SMOOTHING * self.smoothed[k] + (1.0 - SMOOTHING) * mag;
                self.smoothed[k] = s;
                let db = 20.0 * s.log10();
                let scaled = 255.0 * (db - MIN_DECIBELS) / (MAX_DECIBELS - MIN_DECIBELS);
                if scaled.is_nan() {
                    0.0
                } else {
                    scaled.clamp(0.0, 255.0).floor()
                }
            })
            .collect();

        let bin_hz = stream.sample_rate() / n;
        let last = bytes.len() - 1;
        let mut bands = [0.0; 3];
        for (band, &(lo, hi)) in bands.iter_mut().zip(BANDS_HZ.iter()) {
            let from = (lo / bin_hz).floor().max(0.0) as usize;
            let to = ((hi / bin_hz).ceil() as usize).min(last);
            let avg = if to >= from {
                bytes[from..=to].iter().sum::<f64>() / (to - from + 1) as f64 / 255.0
            } else {
                0.0
            };
            *band = avg * BAND_GAIN * sensitivity;
        }
        Reading { level, bands }
    }
}

/// In-place iterative radix-2 FFT. `re.len()` must be a power of two.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two() && im.len() == n);
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = next;
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f64, amp: f64, rate: f64, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (amp * (2.0 * PI * freq * i as f64 / rate).sin()) as f32)
            .collect()
    }

    fn read_tone(freq: f64) -> Reading {
        let stream = PcmStream::new(48_000.0);
        stream.push(&sine(freq, 0.3, 48_000.0, 4096));
        let mut a = SpectrumAnalyser::new();
        // Let the 0.5 smoothing settle.
        let mut r = Reading::default();
        for _ in 0..12 {
            r = a.read(&stream, 3.1);
        }
        r
    }

    #[test]
    fn fft_of_an_impulse_is_flat() {
        let mut re = vec![0.0; 8];
        let mut im = vec![0.0; 8];
        re[0] = 1.0;
        fft(&mut re, &mut im);
        assert!(re.iter().all(|v| (v - 1.0).abs() < 1e-12));
        assert!(im.iter().all(|v| v.abs() < 1e-12));
    }

    #[test]
    fn rms_level_is_exact() {
        let stream = PcmStream::new(48_000.0);
        stream.push(&vec![0.1_f32; 2048]);
        let r = SpectrumAnalyser::new().read(&stream, 2.0);
        assert!((r.level - 0.1 * BASE_GAIN * 2.0).abs() < 1e-6);
    }

    #[test]
    fn silence_reads_zero() {
        let r = SpectrumAnalyser::new().read(&PcmStream::new(44_100.0), 3.1);
        assert_eq!(r, Reading::default());
    }

    #[test]
    fn sines_land_in_their_band() {
        let low = read_tone(180.0);
        let mid = read_tone(1000.0);
        let high = read_tone(4000.0);
        assert!(
            low.bands[0] > low.bands[1] && low.bands[0] > low.bands[2],
            "{low:?}"
        );
        assert!(
            mid.bands[1] > mid.bands[0] && mid.bands[1] > mid.bands[2],
            "{mid:?}"
        );
        assert!(
            high.bands[2] > high.bands[0] && high.bands[2] > high.bands[1],
            "{high:?}"
        );
    }

    #[test]
    fn short_history_is_left_padded() {
        let stream = PcmStream::new(48_000.0);
        stream.push(&[1.0, 2.0]);
        let mut out = [9.0_f32; 4];
        stream.latest(&mut out);
        assert_eq!(out, [0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn ring_drops_the_oldest_samples() {
        let stream = PcmStream::new(48_000.0);
        stream.push(&vec![1.0; RING_CAPACITY]);
        stream.push(&[2.0]);
        let mut out = [0.0_f32; 2];
        stream.latest(&mut out);
        assert_eq!(out, [1.0, 2.0]);
    }

    #[test]
    fn level_sources_compare_by_identity() {
        let s = PcmStream::new(48_000.0);
        assert_eq!(LevelSource::pcm(s.clone()), LevelSource::pcm(s));
        assert_eq!(LevelSource::constant(0.1), LevelSource::from(0.1));
        assert_ne!(LevelSource::constant(0.1), LevelSource::getter(|| 0.1));
    }
}
