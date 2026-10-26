//! [`Microphone`]: live input for the glow (`useMicrophone.ts`), behind the
//! `microphone` cargo feature.
//!
//! The browser hook asks `getUserMedia` for the default input with echo
//! cancellation, noise suppression and auto gain **off** (the glow wants the
//! raw signal), and exposes a `state` of `idle | requesting | live | denied |
//! unsupported | error`. This helper does the same with `cpal`: it opens the
//! default input device, mixes it down to mono and feeds a [`PcmStream`] that
//! [`LevelSource::pcm`](super::LevelSource::pcm) reads.
//!
//! `cpal` offers no signal processing, so the three `MediaTrackConstraints`
//! flags have no equivalent: capture is always raw, which is also the
//! upstream default.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};

use super::audio::{LevelSource, PcmStream};

/// The microphone's lifecycle (`MicrophoneState`).
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::MicrophoneState;
///
/// assert_eq!(MicrophoneState::default(), MicrophoneState::Idle);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MicrophoneState {
    /// Not started.
    #[default]
    Idle,
    /// Opening the device.
    Requesting,
    /// Capturing.
    Live,
    /// The system refused access.
    Denied,
    /// No input device or no usable format.
    Unsupported,
    /// Anything else went wrong; see [`Microphone::error`].
    Error,
}

/// Which input to open.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::voice_glow::MicrophoneConstraints;
///
/// let constraints = MicrophoneConstraints::default();
/// assert!(!constraints.echo_cancellation && !constraints.noise_suppression);
/// assert_eq!(constraints.device, None);
/// ```
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MicrophoneConstraints {
    /// Accepted for parity with `MediaTrackConstraints`; `cpal` cannot
    /// process, so capture is raw whatever this says. Default `false`.
    pub echo_cancellation: bool,
    /// See [`echo_cancellation`](Self::echo_cancellation). Default `false`.
    pub noise_suppression: bool,
    /// See [`echo_cancellation`](Self::echo_cancellation). Default `false`.
    pub auto_gain_control: bool,
    /// A device name; `None` is the system default input.
    pub device: Option<String>,
}

/// A live microphone feeding a [`PcmStream`].
///
/// The stream handle `cpal` returns is not `Send`, so keep a `Microphone` in
/// your application state (iced applications need not be `Send`).
///
/// # Examples
///
/// ```rust,no_run
/// use enchant_iced::widgets::voice_glow::{Microphone, MicrophoneState, VoiceGlow};
///
/// let mut mic = Microphone::new();
/// if mic.start() == MicrophoneState::Live {
///     if let Some(source) = mic.level_source() {
///         let _glow = VoiceGlow::new().level_source(source);
///     }
/// }
/// ```
pub struct Microphone {
    state: MicrophoneState,
    error: Option<String>,
    constraints: MicrophoneConstraints,
    pcm: Option<PcmStream>,
    stream: Option<Stream>,
    /// Set by the stream's error callback when the device goes away (the
    /// track's `ended` event upstream).
    ended: Arc<AtomicBool>,
}

impl fmt::Debug for Microphone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Microphone")
            .field("state", &self.state)
            .field("error", &self.error)
            .field("constraints", &self.constraints)
            .finish_non_exhaustive()
    }
}

impl Default for Microphone {
    fn default() -> Self {
        Self::new()
    }
}

impl Microphone {
    /// An idle microphone with the default constraints.
    #[must_use]
    pub fn new() -> Self {
        Self::with_constraints(MicrophoneConstraints::default())
    }

    /// An idle microphone with `constraints`.
    #[must_use]
    pub fn with_constraints(constraints: MicrophoneConstraints) -> Self {
        Self {
            state: MicrophoneState::Idle,
            error: None,
            constraints,
            pcm: None,
            stream: None,
            ended: Arc::new(AtomicBool::new(false)),
        }
    }

    /// A microphone that asks for the input right away (`autoStart`); check
    /// [`state`](Self::state) for the outcome.
    #[must_use]
    pub fn started(constraints: MicrophoneConstraints) -> Self {
        let mut mic = Self::with_constraints(constraints);
        mic.start();
        mic
    }

    /// Whether this machine can capture audio at all (`supported`,
    /// `isAudioSupported()`): there is a default input device with a usable
    /// configuration.
    #[must_use]
    pub fn is_supported() -> bool {
        cpal::default_host()
            .default_input_device()
            .is_some_and(|d| d.default_input_config().is_ok())
    }

    /// The current state. A live microphone whose device went away (unplugged,
    /// permission revoked: the track's `ended` event) reads as
    /// [`MicrophoneState::Idle`].
    #[must_use]
    pub fn state(&self) -> MicrophoneState {
        if self.state == MicrophoneState::Live && self.ended.load(Ordering::Relaxed) {
            MicrophoneState::Idle
        } else {
            self.state
        }
    }

    /// The last failure's message, for [`MicrophoneState::Error`] and
    /// [`MicrophoneState::Denied`].
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The PCM stream while live.
    #[must_use]
    pub fn stream(&self) -> Option<&PcmStream> {
        self.pcm
            .as_ref()
            .filter(|_| !self.ended.load(Ordering::Relaxed))
    }

    /// The live stream as a level source for [`VoiceGlow::level_source`](super::VoiceGlow::level_source).
    #[must_use]
    pub fn level_source(&self) -> Option<LevelSource> {
        self.stream().cloned().map(LevelSource::pcm)
    }

    /// Opens the input and starts capturing (`start()`); a running
    /// microphone is left as is. Returns the new state.
    pub fn start(&mut self) -> MicrophoneState {
        if self.state() == MicrophoneState::Live {
            return self.state;
        }
        self.state = MicrophoneState::Requesting;
        self.error = None;
        self.ended.store(false, Ordering::Relaxed);
        match self.open() {
            Ok((stream, pcm)) => {
                self.stream = Some(stream);
                self.pcm = Some(pcm);
                self.state = MicrophoneState::Live;
            }
            Err((state, message)) => {
                self.stream = None;
                self.pcm = None;
                self.error = Some(message);
                self.state = state;
            }
        }
        self.state
    }

    /// Stops capturing and releases the device (`stop()`).
    pub fn stop(&mut self) {
        self.stream = None;
        self.pcm = None;
        self.state = MicrophoneState::Idle;
        self.error = None;
        self.ended.store(false, Ordering::Relaxed);
    }

    fn open(&self) -> Result<(Stream, PcmStream), (MicrophoneState, String)> {
        let host = cpal::default_host();
        let device = match &self.constraints.device {
            Some(name) => host
                .input_devices()
                .map_err(classify)?
                .find(|d| d.name().is_ok_and(|n| &n == name))
                .ok_or_else(|| {
                    (
                        MicrophoneState::Unsupported,
                        format!("no input device named {name}"),
                    )
                })?,
            None => host.default_input_device().ok_or_else(|| {
                (
                    MicrophoneState::Unsupported,
                    "no default input device".to_owned(),
                )
            })?,
        };
        let supported = device.default_input_config().map_err(classify)?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let pcm = PcmStream::new(f64::from(config.sample_rate.0));
        let channels = usize::from(config.channels).max(1);
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, &config, &pcm, channels, &self.ended),
            SampleFormat::I16 => build::<i16>(&device, &config, &pcm, channels, &self.ended),
            SampleFormat::U16 => build::<u16>(&device, &config, &pcm, channels, &self.ended),
            SampleFormat::I32 => build::<i32>(&device, &config, &pcm, channels, &self.ended),
            other => {
                return Err((
                    MicrophoneState::Unsupported,
                    format!("unsupported sample format {other:?}"),
                ));
            }
        }
        .map_err(classify)?;
        stream.play().map_err(classify)?;
        Ok((stream, pcm))
    }
}

/// Opens an input stream of `T` samples that mixes down to mono.
fn build<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    pcm: &PcmStream,
    channels: usize,
    ended: &Arc<AtomicBool>,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let pcm = pcm.clone();
    let ended = Arc::clone(ended);
    let mut mono: Vec<f32> = Vec::new();
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            mono.clear();
            mono.extend(data.chunks(channels).map(|frame| {
                frame.iter().map(|s| f32::from_sample(*s)).sum::<f32>() / frame.len() as f32
            }));
            pcm.push(&mono);
        },
        move |err| {
            eprintln!("voice_glow microphone stream error: {err}");
            if matches!(err, cpal::StreamError::DeviceNotAvailable) {
                ended.store(true, Ordering::Relaxed);
            }
        },
        None,
    )
}

/// Maps a `cpal` failure onto a state: permission problems are `Denied`, a
/// missing device `Unsupported`, the rest `Error`.
fn classify(err: impl fmt::Display + fmt::Debug) -> (MicrophoneState, String) {
    let message = err.to_string();
    let lower = format!("{message} {err:?}").to_lowercase();
    let state =
        if lower.contains("denied") || lower.contains("permission") || lower.contains("access") {
            MicrophoneState::Denied
        } else if lower.contains("devicenotavailable") || lower.contains("no longer available") {
            MicrophoneState::Unsupported
        } else {
            MicrophoneState::Error
        };
    (state, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_microphone_is_idle_and_has_no_stream() {
        let mic = Microphone::new();
        assert_eq!(mic.state(), MicrophoneState::Idle);
        assert!(mic.stream().is_none());
        assert!(mic.level_source().is_none());
        assert!(mic.error().is_none());
    }

    #[test]
    fn stop_returns_to_idle() {
        let mut mic = Microphone::new();
        mic.stop();
        assert_eq!(mic.state(), MicrophoneState::Idle);
    }

    /// Needs a real input device: `cargo test --features microphone -- --ignored`.
    #[test]
    #[ignore = "opens the real microphone"]
    fn start_opens_the_default_input_when_there_is_one() {
        let mut mic = Microphone::new();
        let state = mic.start();
        eprintln!("microphone state: {state:?} ({:?})", mic.error());
        assert_ne!(state, MicrophoneState::Requesting);
        if state == MicrophoneState::Live {
            std::thread::sleep(std::time::Duration::from_millis(300));
            assert!(mic.stream().is_some());
            mic.stop();
            assert_eq!(mic.state(), MicrophoneState::Idle);
        }
    }

    #[test]
    fn errors_classify() {
        assert_eq!(classify("Access is denied").0, MicrophoneState::Denied);
        assert_eq!(
            classify("DeviceNotAvailable").0,
            MicrophoneState::Unsupported
        );
        assert_eq!(classify("boom").0, MicrophoneState::Error);
    }
}
