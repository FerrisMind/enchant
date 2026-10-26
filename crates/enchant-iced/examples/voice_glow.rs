//! Voice glow: the sound-reactive beam of Libraries.dev's `VoiceBeam`, wrapped
//! around the three upstream demo hosts (chat input, phone, recording pill).
//!
//! ```text
//! cargo run -p enchant-iced --example voice_glow --release
//! cargo run -p enchant-iced --example voice_glow --release --features microphone
//! ```
//!
//! The window mirrors the website's voice page: the two examples on top, run
//! by the demo voice, then the playground (a stage and its controls: Type,
//! Source, theme, palette). The stage starts paused, like upstream's.
//!
//! # Headless export
//!
//! `--export` renders one frame with the same renderer, without a window, for
//! comparing against the website:
//!
//! ```text
//! cargo run -p enchant-iced --example voice_glow --release -- --export \
//!     --type default --theme dark --level 0.8 --time 2.0 \
//!     --width 371 --height 104 --bg "#1c1c1c" --page "#0a0a0a" --density 1 \
//!     --out target/voice-cmp/chat.png
//! ```
//!
//! Options: `--type default|mobile|pill`, `--theme dark|light`, `--level 0..1`,
//! `--processing`, `--time <seconds>`, `--width/--height <px>`, `--bg <css
//! colour>` (host fill under the glow), `--page <css colour>` (outside the
//! corners), `--density <n>`, `--variant <palette>`, `--radius <px>`,
//! `--scale <n>`, `--strength 0..1`, `--out <png or dir>`.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use enchant_iced::widgets::voice_glow::{
    ColorVariant, GlowLayer, GlowLayering, HostSize, RenderOptions, ResolvedTheme, Rgb, VoiceBeam,
    VoiceGlow, VoiceGlowTheme, VoiceGlowType,
};
use iced::advanced::Clipboard;
use iced::advanced::Layout;
use iced::advanced::Shell;
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::text::{self as adv_text};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Renderer as _, text::Renderer as _};
use iced::mouse;
use iced::widget::{button, column, container, pick_list, row, slider, space, text, text_input};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Font, Length, Pixels, Point, Rectangle,
    Renderer, Size, Subscription, Theme, Transformation, border, window,
};
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};

/// Upstream `.mock-vchat-btn:active { transform: scale(0.96) }` —
/// `transition: transform 0.15s cubic-bezier(0.22, 1, 0.36, 1)` (examples.css).
const HOST_PRESS_SCALE: f32 = 0.96;
const HOST_PRESS_MS: f32 = 150.0;
/// CSS `cubic-bezier(0.22, 1, 0.36, 1)` control points.
const HOST_PRESS_BEZIER: (f32, f32, f32, f32) = (0.22, 1.0, 0.36, 1.0);

// ── the demo voice (voice-mocks.tsx `demoLevel`) ────────────────────────────

/// A speech-like envelope: syllables (~3 Hz) under words (~0.55 Hz), a little
/// roughness and a pause at the end of each 9 s phrase. A pure function of
/// time, so every instance agrees.
fn demo_level(t: f64) -> f64 {
    let phrase = t % 9.0;
    if phrase > 6.6 {
        return 0.0;
    }
    let tau = std::f64::consts::TAU;
    let syllable = 0.5 + 0.5 * (t * tau * 3.1).sin();
    let word = 0.5 + 0.5 * (t * tau * 0.55 + 1.0).sin();
    let rough = 0.86 + 0.14 * (t * 23.7).sin();
    (syllable.powf(1.6) * (0.5 + 0.5 * word) * rough * 1.05).min(1.0)
}

fn clock() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

const DEMO_TRANSCRIPT: &str = "Set a timer for ten minutes and remind me to water the plants";

// ── geometry of the hosts ───────────────────────────────────────────────────

const CHAT_SIZE: (f32, f32) = (371.0, 104.0);
const CHAT_RADIUS: f64 = 20.0;
const PHONE_CROP: f64 = 0.68;
const PHONE_SCALE: f64 = 1.25 * PHONE_CROP;
const PHONE_RADIUS: f64 = 66.0 * PHONE_CROP;
const PHONE_SIZE: (f32, f32) = (273.0, 357.0);
const PILL_SIZE: (f32, f32) = (149.0, 44.0);
const PILL_RADIUS: f64 = 22.0;

/// The glow for a host type with the page's tuning.
fn glow_for(kind: VoiceGlowType) -> VoiceGlow {
    let glow = VoiceGlow::new().kind(kind).theme(VoiceGlowTheme::Auto);
    match kind {
        VoiceGlowType::Mobile => glow.scale(PHONE_SCALE).border_radius(PHONE_RADIUS),
        VoiceGlowType::Pill => glow.border_radius(PILL_RADIUS),
        _ => glow.border_radius(CHAT_RADIUS),
    }
}

fn host_size(kind: VoiceGlowType) -> (f32, f32) {
    match kind {
        VoiceGlowType::Mobile => PHONE_SIZE,
        VoiceGlowType::Pill => PILL_SIZE,
        _ => CHAT_SIZE,
    }
}

// ── headless export ─────────────────────────────────────────────────────────

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn parse<T: FromStr>(args: &[String], name: &str) -> Result<Option<T>, String> {
    match flag(args, name) {
        None => Ok(None),
        Some(v) => v
            .parse()
            .map(Some)
            .map_err(|_| format!("invalid value {v:?} for {name}")),
    }
}

fn parse_rgb(args: &[String], name: &str) -> Result<Option<[u8; 4]>, String> {
    match flag(args, name) {
        None => Ok(None),
        Some(v) => Rgb::from_str(&v)
            .map(|c| Some([c.r, c.g, c.b, 255]))
            .map_err(|e| format!("invalid colour for {name}: {e}")),
    }
}

fn export(args: &[String]) -> Result<PathBuf, String> {
    let kind = match flag(args, "--type").as_deref() {
        None | Some("default") => VoiceGlowType::Default,
        Some("mobile") => VoiceGlowType::Mobile,
        Some("pill") => VoiceGlowType::Pill,
        Some(other) => return Err(format!("unknown --type {other:?}")),
    };
    let theme = match flag(args, "--theme").as_deref() {
        None | Some("dark") => ResolvedTheme::Dark,
        Some("light") => ResolvedTheme::Light,
        Some(other) => return Err(format!("unknown --theme {other:?}")),
    };
    let (dw, dh) = host_size(kind);
    let width: f64 = parse(args, "--width")?.unwrap_or(f64::from(dw));
    let height: f64 = parse(args, "--height")?.unwrap_or(f64::from(dh));
    let time: f64 = parse(args, "--time")?.unwrap_or(2.0);
    let density: f32 = parse(args, "--density")?.unwrap_or(1.0);

    let mut glow = glow_for(kind).theme(if theme.is_dark() {
        VoiceGlowTheme::Dark
    } else {
        VoiceGlowTheme::Light
    });
    if let Some(level) = parse::<f64>(args, "--level")? {
        glow = glow.level(level);
    }
    if args.iter().any(|a| a == "--processing") {
        glow = glow.processing(true);
    }
    if let Some(radius) = parse::<f64>(args, "--radius")? {
        glow = glow.border_radius(radius);
    }
    if let Some(scale) = parse::<f64>(args, "--scale")? {
        glow = glow.scale(scale);
    }
    if let Some(strength) = parse::<f64>(args, "--strength")? {
        glow = glow.strength(strength);
    }
    if let Some(name) = flag(args, "--variant") {
        let variant = ColorVariant::ALL
            .into_iter()
            .find(|v| v.name() == name)
            .ok_or_else(|| format!("unknown --variant {name:?}"))?;
        glow = glow.color_variant(variant);
    }

    if let Some(distortion) = parse::<f64>(args, "--distortion")? {
        glow = glow.distortion(distortion);
    }
    if let Some(list) = flag(args, "--colors") {
        glow = glow.colors_css(list.split(','));
    }

    let mut options = RenderOptions::new().density(density);
    if let Some(bg) = parse_rgb(args, "--bg")? {
        options = options.background(bg);
    }
    if let Some(layer) = parse::<GlowLayer>(args, "--only")? {
        options = options.solo(layer);
    }
    if let Some(page) = parse_rgb(args, "--page")? {
        options = options.page(page);
    }
    if args.iter().any(|a| a == "--dump") {
        let frame = glow.frame_at(
            Duration::from_secs_f64(time.max(0.0)),
            HostSize::new(width, height),
            theme,
        );
        let lobes: Vec<String> = frame
            .lobes
            .iter()
            .map(|l| format!("{:.1}/{:.3}@{:.1}", l.x, l.amplitude, l.center_x))
            .collect();
        println!(
            "level {:.3} cx {:.1} mw {:.3} bh {:.1} bendA {:.3} glow {:.3} h {:.3} w {:.3} hue {:.2} lobes {}",
            frame.level,
            frame.center_offset,
            frame.mask_width,
            frame.bend_height,
            frame.bend_strength,
            frame.glow,
            frame.height_scale,
            frame.width_scale,
            frame.hue_drift,
            lobes.join(" "),
        );
    }
    let bitmap = glow.render_at(
        Duration::from_secs_f64(time.max(0.0)),
        HostSize::new(width, height),
        theme,
        &options,
    );

    let default_name = format!(
        "voice-glow-{}-{}.png",
        flag(args, "--type").unwrap_or_else(|| "default".into()),
        if theme.is_dark() { "dark" } else { "light" },
    );
    let out = match flag(args, "--out") {
        Some(p) if Path::new(&p).extension().is_some() => PathBuf::from(p),
        Some(dir) => PathBuf::from(dir).join(default_name),
        None => PathBuf::from("target/voice-cmp").join(default_name),
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    image::save_buffer(
        &out,
        &bitmap.rgba,
        bitmap.width,
        bitmap.height,
        image::ColorType::Rgba8,
    )
    .map_err(|e| format!("write {}: {e}", out.display()))?;
    Ok(out)
}

// ── the app ─────────────────────────────────────────────────────────────────

pub fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--export") {
        match export(&args) {
            Ok(path) => println!("wrote {}", path.display()),
            Err(err) => {
                eprintln!("voice_glow --export: {err}");
                std::process::exit(2);
            }
        }
        return Ok(());
    }
    iced::application(Demo::new, Demo::update, Demo::view)
        .title("Voice glow")
        .theme(Demo::theme)
        .subscription(Demo::subscription)
        .font(LUCIDE_FONT_BYTES)
        .window(window::Settings {
            size: Size::new(1040.0, 940.0),
            ..Default::default()
        })
        .run()
}

/// Press targets on the mock hosts (nothing is sent anywhere — same as the
/// upstream React mocks: a real field and real buttons you can type into
/// and press while the beam runs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostAction {
    Add,
    Agent,
    Voice,
    Close,
    Stop,
}

/// Independent host instances on the page. Presses on one never touch another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostId {
    /// Top-left phone example card.
    GalleryPhone,
    /// Top-right chat example card.
    GalleryChat,
    /// Playground stage.
    Stage,
}

impl HostId {
    fn label(self) -> &'static str {
        match self {
            Self::GalleryPhone => "gallery phone",
            Self::GalleryChat => "gallery chat",
            Self::Stage => "stage",
        }
    }
}

/// Per-host UI state (draft, transcript pacing, pill timer, local listening).
#[derive(Clone, Debug, Default)]
struct HostState {
    draft: String,
    words: usize,
    last_word: f64,
    quiet_since: f64,
    pill_seconds: u64,
    pill_tick: f64,
    listening: bool,
}

impl HostState {
    fn transcript(&self) -> String {
        let words: Vec<&str> = DEMO_TRANSCRIPT.split(' ').take(self.words).collect();
        if words.is_empty() {
            "Speak, I'm listening".to_owned()
        } else {
            words.join(" ")
        }
    }

    fn pace_words(&mut self, level: f64) {
        let now = clock();
        let total = DEMO_TRANSCRIPT.split(' ').count();
        if level < 0.05 {
            if now - self.quiet_since > 2.5 {
                self.words = 0;
            }
        } else {
            self.quiet_since = now;
            if level > 0.22 && now - self.last_word > 0.32 && self.words < total {
                self.words += 1;
                self.last_word = now;
            }
        }
    }

    fn apply(&mut self, action: HostAction) {
        match action {
            HostAction::Add | HostAction::Agent => {}
            HostAction::Voice => {
                self.listening = !self.listening;
                if self.listening {
                    self.pill_tick = clock();
                }
            }
            HostAction::Close => {
                self.draft.clear();
                self.words = 0;
                self.listening = false;
            }
            HostAction::Stop => {
                self.listening = false;
                self.pill_seconds = 0;
                self.pill_tick = clock();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StageType {
    ChatInput,
    Mobile,
    Pill,
}

impl StageType {
    const ALL: [Self; 3] = [Self::ChatInput, Self::Mobile, Self::Pill];

    fn kind(self) -> VoiceGlowType {
        match self {
            Self::ChatInput => VoiceGlowType::Default,
            Self::Mobile => VoiceGlowType::Mobile,
            Self::Pill => VoiceGlowType::Pill,
        }
    }
}

impl std::fmt::Display for StageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ChatInput => "Chat input",
            Self::Mobile => "Mobile",
            Self::Pill => "Recording pill",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Demo,
    Processing,
    #[cfg(feature = "microphone")]
    Microphone,
}

impl Source {
    #[cfg(not(feature = "microphone"))]
    const ALL: [Self; 2] = [Self::Demo, Self::Processing];
    #[cfg(feature = "microphone")]
    const ALL: [Self; 3] = [Self::Demo, Self::Processing, Self::Microphone];
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Demo => "Demo voice",
            Self::Processing => "Processing",
            #[cfg(feature = "microphone")]
            Self::Microphone => "Microphone",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeChoice {
    Dark,
    Light,
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        })
    }
}

struct Demo {
    stage: StageType,
    source: Source,
    theme: ThemeChoice,
    variant: ColorVariant,
    strength: f64,
    /// Playground play/pause — only freezes the stage glow, not gallery cards.
    playing: bool,
    gallery_phone: HostState,
    gallery_chat: HostState,
    stage_host: HostState,
    /// Last host action pressed, shown briefly so the buttons feel live.
    last_action: Option<(HostId, &'static str)>,
    #[cfg(feature = "microphone")]
    mic: enchant_iced::widgets::voice_glow::Microphone,
}

#[derive(Clone, Debug)]
enum Message {
    Tick,
    Stage(StageType),
    Source(Source),
    Theme(ThemeChoice),
    Variant(ColorVariant),
    Strength(f64),
    Playing(bool),
    DraftChanged(HostId, String),
    Host(HostId, HostAction),
    PhoneLevel(f64),
    StageLevel(f64),
}

impl Demo {
    fn new() -> Self {
        Self {
            stage: StageType::ChatInput,
            source: Source::Demo,
            theme: ThemeChoice::Dark,
            variant: ColorVariant::Colorful,
            strength: 1.0,
            playing: false,
            gallery_phone: HostState::default(),
            gallery_chat: HostState::default(),
            stage_host: HostState::default(),
            last_action: None,
            #[cfg(feature = "microphone")]
            mic: enchant_iced::widgets::voice_glow::Microphone::new(),
        }
    }

    fn host(&self, id: HostId) -> &HostState {
        match id {
            HostId::GalleryPhone => &self.gallery_phone,
            HostId::GalleryChat => &self.gallery_chat,
            HostId::Stage => &self.stage_host,
        }
    }

    fn host_mut(&mut self, id: HostId) -> &mut HostState {
        match id {
            HostId::GalleryPhone => &mut self.gallery_phone,
            HostId::GalleryChat => &mut self.gallery_chat,
            HostId::Stage => &mut self.stage_host,
        }
    }

    fn theme(&self) -> Theme {
        match self.theme {
            ThemeChoice::Dark => Theme::Dark,
            ThemeChoice::Light => Theme::Light,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // The glow redraws itself; this only keeps the transcript pacing and
        // the mic readout moving.
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    }

    fn tick_pill(host: &mut HostState) {
        if !host.listening {
            return;
        }
        let now = clock();
        if now - host.pill_tick >= 1.0 {
            host.pill_seconds = host.pill_seconds.saturating_add(1);
            host.pill_tick = now;
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                Self::tick_pill(&mut self.gallery_phone);
                Self::tick_pill(&mut self.gallery_chat);
                // Stage pill advances while its own mic is live, or while the
                // playground Play is on and the stage type is the pill.
                let stage_pill_running =
                    self.stage_host.listening || (self.playing && self.stage == StageType::Pill);
                if stage_pill_running {
                    let now = clock();
                    if now - self.stage_host.pill_tick >= 1.0 {
                        self.stage_host.pill_seconds =
                            self.stage_host.pill_seconds.saturating_add(1);
                        self.stage_host.pill_tick = now;
                    }
                }
            }
            Message::Stage(s) => {
                self.stage = s;
                let stage = &mut self.stage_host;
                stage.words = 0;
                stage.pill_seconds = 0;
                stage.pill_tick = clock();
                stage.listening = false;
            }
            Message::Source(s) => {
                self.source = s;
                self.stage_host.words = 0;
                #[cfg(feature = "microphone")]
                {
                    if s == Source::Microphone {
                        self.mic.start();
                        self.playing = true;
                    } else {
                        self.mic.stop();
                    }
                }
            }
            Message::Theme(t) => self.theme = t,
            Message::Variant(v) => self.variant = v,
            Message::Strength(s) => self.strength = s,
            Message::Playing(p) => {
                self.playing = p;
                if p {
                    self.stage_host.pill_tick = clock();
                } else if self.stage == StageType::Pill {
                    self.stage_host.listening = false;
                }
            }
            Message::DraftChanged(id, value) => self.host_mut(id).draft = value,
            Message::Host(id, action) => {
                let name = match action {
                    HostAction::Add => "Add",
                    HostAction::Agent => "Agent menu",
                    HostAction::Voice => "Voice",
                    HostAction::Close => "Close",
                    HostAction::Stop => "Stop",
                };
                self.last_action = Some((id, name));
                self.host_mut(id).apply(action);
                // Stop on the stage pill also releases playground Play.
                if id == HostId::Stage && action == HostAction::Stop {
                    self.playing = false;
                }
            }
            Message::PhoneLevel(level) => self.gallery_phone.pace_words(level),
            Message::StageLevel(level) => self.stage_host.pace_words(level),
        }
    }

    // ── the three hosts ─────────────────────────────────────────────────────

    fn dark(&self) -> bool {
        self.theme == ThemeChoice::Dark
    }

    fn beam<'a>(
        &self,
        id: HostId,
        stage: StageType,
        glow: VoiceGlow,
        on_level: fn(f64) -> Message,
    ) -> Element<'a, Message> {
        let (w, h) = host_size(stage.kind());
        let dark = self.dark();
        let host = self.host(id);
        let (content, backdrop): (Element<'a, Message>, Element<'a, Message>) = match stage {
            StageType::ChatInput => (self.chat_content(id, host, w, h), chat_backdrop(w, h, dark)),
            StageType::Mobile => (
                self.phone_content(id, host, w, h),
                phone_backdrop(w, h, dark),
            ),
            StageType::Pill => (
                pill_content(id, host.pill_seconds, w, h, dark),
                pill_backdrop(w, h, dark),
            ),
        };
        VoiceBeam::new(glow, content)
            .backdrop(backdrop)
            // The host's content sits above the effect's layers (z 5).
            .layering(GlowLayering::BelowContent)
            .on_level(on_level)
            .into()
    }

    fn chat_content<'a>(
        &self,
        id: HostId,
        host: &HostState,
        w: f32,
        h: f32,
    ) -> Element<'a, Message> {
        let dark = self.dark();
        let ink = host_ink(dark);
        let agent_ink = if dark {
            Color::from_rgb8(0xca, 0xcc, 0xd2)
        } else {
            Color::from_rgb8(0x25, 0x25, 0x25)
        };
        let mic_ink = if dark {
            Color::from_rgb8(0xb2, 0xb2, 0xb2)
        } else {
            Color::from_rgb8(0x51, 0x51, 0x51)
        };
        let field = text_input("Ask me anything..", &host.draft)
            .on_input(move |value| Message::DraftChanged(id, value))
            .size(14)
            .padding(0)
            .width(Length::Fill)
            .style(move |_, status| chat_field_style(dark, status));

        container(
            column![
                container(field).padding(iced::Padding {
                    top: 7.0,
                    right: 3.0,
                    bottom: 0.0,
                    left: 3.0,
                }),
                space::vertical(),
                row![
                    icon_button(id, LucideIcon::Plus, ink, HostAction::Add, dark, 16.0),
                    space::horizontal(),
                    agent_button(id, "Agent", agent_ink, dark, HostAction::Agent),
                    icon_button(id, LucideIcon::Mic, mic_ink, HostAction::Voice, dark, 16.0),
                    icon_button(id, LucideIcon::X, ink, HostAction::Close, dark, 16.0),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ]
            .height(Length::Fill),
        )
        .padding(iced::Padding {
            top: 13.0,
            right: 13.0,
            bottom: 13.0,
            left: 11.0,
        })
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .into()
    }

    fn phone_content<'a>(
        &self,
        id: HostId,
        host: &HostState,
        w: f32,
        h: f32,
    ) -> Element<'a, Message> {
        let dark = self.dark();
        let ink = host_ink(dark);
        let live = host.words > 0;
        let prompt = text(host.transcript())
            .size(22)
            .color(if live {
                ink
            } else {
                Color::from_rgb8(0x6a, 0x6a, 0x70)
            })
            .width(Length::Fill);
        container(
            column![
                space::vertical(),
                container(prompt).padding([0, 20]),
                space::vertical().height(Length::Fixed(28.0)),
                row![
                    agent_button(id, "Agent (auto)", ink, dark, HostAction::Agent),
                    space::horizontal(),
                    icon_button(id, LucideIcon::Mic, ink, HostAction::Voice, dark, 18.0),
                    icon_button(id, LucideIcon::X, ink, HostAction::Close, dark, 16.0),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ]
            .height(Length::Fill),
        )
        .padding(iced::Padding {
            top: 0.0,
            right: 18.0,
            bottom: 24.0,
            left: 18.0,
        })
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .into()
    }

    // ── the page ────────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        let dark = self.dark();
        let stage_bg = if dark {
            Color::from_rgb8(0x0a, 0x0a, 0x0b)
        } else {
            Color::from_rgb8(0xf3, 0xf3, 0xf5)
        };
        let card_bg = if dark {
            Color::from_rgb8(0x10, 0x10, 0x12)
        } else {
            Color::WHITE
        };
        let card = move |_: &Theme| {
            container::Style::default()
                .background(card_bg)
                .border(Border {
                    color: Color::from_rgba(0.5, 0.5, 0.55, 0.18),
                    width: 1.0,
                    radius: border::radius(16),
                })
        };

        // The two examples, run by the demo voice (the phone half a phrase
        // behind, so they take turns).
        let phone_glow = glow_for(VoiceGlowType::Mobile)
            .level_source(enchant_iced::widgets::voice_glow::LevelSource::getter(
                || demo_level(clock() + 4.5),
            ))
            .color_variant(self.variant);
        let chat_glow = glow_for(VoiceGlowType::Default)
            .level_source(enchant_iced::widgets::voice_glow::LevelSource::getter(
                || demo_level(clock()),
            ))
            .color_variant(self.variant);
        let examples = row![
            container(self.beam(
                HostId::GalleryPhone,
                StageType::Mobile,
                phone_glow,
                Message::PhoneLevel
            ))
            .padding(24)
            .style(card),
            container(
                self.beam(HostId::GalleryChat, StageType::ChatInput, chat_glow, |_| {
                    Message::Tick
                })
            )
            .padding(24)
            .center_y(Length::Fill)
            .style(card),
        ]
        .spacing(20)
        .align_y(Alignment::Start);

        let stage_glow = self.stage_glow();
        let stage = container(
            container(self.beam(HostId::Stage, self.stage, stage_glow, Message::StageLevel))
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(Length::FillPortion(3))
        .height(Length::Fixed(430.0))
        .padding(24)
        .style(move |_| {
            container::Style::default()
                .background(stage_bg)
                .border(Border {
                    color: Color::from_rgba(0.5, 0.5, 0.55, 0.18),
                    width: 1.0,
                    radius: border::radius(16),
                })
        });

        let play_icon = if self.playing {
            LucideIcon::Pause
        } else {
            LucideIcon::Play
        };
        let play_label = if self.playing { "Pause" } else { "Play" };
        let play_btn = scale_press_button(
            container(
                row![
                    lucide(play_icon, host_ink(dark), 14.0),
                    text(play_label).size(13).color(host_ink(dark)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([8, 14]),
            Message::Playing(!self.playing),
            dark,
            Length::Shrink,
            Length::Shrink,
        );

        let action_note: Element<'_, Message> = match self.last_action {
            Some((id, label)) => text(format!("Pressed: {label} ({})", id.label()))
                .size(12)
                .into(),
            None => space::vertical().height(Length::Fixed(0.0)).into(),
        };

        let controls = column![
            text("Playground").size(13),
            labelled(
                "Type",
                pick_list(StageType::ALL, Some(self.stage), Message::Stage),
            ),
            labelled(
                "Source",
                pick_list(Source::ALL, Some(self.source), Message::Source),
            ),
            labelled(
                "Theme",
                pick_list(
                    [ThemeChoice::Dark, ThemeChoice::Light],
                    Some(self.theme),
                    Message::Theme
                ),
            ),
            labelled(
                "Palette",
                pick_list(ColorVariant::ALL, Some(self.variant), Message::Variant),
            ),
            labelled(
                "Strength",
                slider(0.0..=1.0, self.strength, Message::Strength).step(0.01),
            ),
            play_btn,
            action_note,
            self.status(),
        ]
        .spacing(14)
        .width(Length::FillPortion(1));

        container(
            column![
                text("Voice glow").size(28),
                examples,
                row![stage, controls].spacing(20),
            ]
            .spacing(20),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn stage_glow(&self) -> VoiceGlow {
        let mut glow = glow_for(self.stage.kind())
            .color_variant(self.variant)
            .strength(self.strength)
            .paused(!self.playing);
        glow = match self.source {
            Source::Demo => glow.level_source(
                enchant_iced::widgets::voice_glow::LevelSource::getter(|| demo_level(clock())),
            ),
            Source::Processing => glow.processing(true).level_source(
                enchant_iced::widgets::voice_glow::LevelSource::getter(|| demo_level(clock())),
            ),
            #[cfg(feature = "microphone")]
            Source::Microphone => match self.mic.level_source() {
                Some(source) => glow.level_source(source),
                None => glow,
            },
        };
        glow
    }

    #[cfg(feature = "microphone")]
    fn status(&self) -> Element<'_, Message> {
        use enchant_iced::widgets::voice_glow::MicrophoneState;
        let line = match self.mic.state() {
            MicrophoneState::Idle => String::new(),
            MicrophoneState::Requesting => "Waiting for permission...".to_owned(),
            MicrophoneState::Live => "Listening.".to_owned(),
            MicrophoneState::Denied => {
                "Microphone blocked: allow it in Windows privacy settings.".to_owned()
            }
            MicrophoneState::Unsupported => "No input device.".to_owned(),
            _ => format!(
                "Couldn't open the microphone: {}",
                self.mic.error().unwrap_or("unknown")
            ),
        };
        text(line).size(12).into()
    }

    #[cfg(not(feature = "microphone"))]
    fn status(&self) -> Element<'_, Message> {
        text("Build with --features microphone for live input.")
            .size(12)
            .into()
    }
}

// ── mock host chrome ────────────────────────────────────────────────────────

fn labelled<'a>(label: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(label).size(12), control.into()]
        .spacing(4)
        .into()
}

fn host_ink(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xea, 0xea, 0xea)
    } else {
        Color::from_rgb8(0x20, 0x20, 0x24)
    }
}

fn lucide<'a>(icon: LucideIcon, color: Color, size: f32) -> Element<'a, Message> {
    text(char::from(icon).to_string())
        .font(Font::with_name("lucide"))
        .size(size)
        .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(size)))
        .style(move |_| iced::widget::text::Style { color: Some(color) })
        .into()
}

/// Icon hit-target: chrome + Lucide via `fill_text` (not a text widget under
/// GPU/`font-size` scale — those pull Lucide ink upward and read as a top tilt).
fn icon_button<'a>(
    id: HostId,
    icon: LucideIcon,
    color: Color,
    action: HostAction,
    dark: bool,
    glyph: f32,
) -> Element<'a, Message> {
    const SIZE: f32 = 36.0;
    Element::new(ScalePressButton {
        kind: ScalePressKind::Icon { icon, color, glyph },
        on_press: Message::Host(id, action),
        dark,
        width: Length::Fixed(SIZE),
        height: Length::Fixed(SIZE),
    })
}

fn agent_button<'a>(
    id: HostId,
    label: &'static str,
    color: Color,
    dark: bool,
    action: HostAction,
) -> Element<'a, Message> {
    let chevron = if dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.4)
    } else {
        Color::from_rgba(0.07, 0.07, 0.07, 0.4)
    };
    scale_press_button(
        container(
            row![
                text(label).size(12).color(color),
                lucide(LucideIcon::ChevronDown, chevron, 14.0),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .width(Length::Shrink)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 10.0,
            bottom: 0.0,
            left: 14.0,
        }),
        Message::Host(id, action),
        dark,
        Length::Shrink,
        Length::Fixed(36.0),
    )
}

fn scale_press_button<'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Message,
    dark: bool,
    width: Length,
    height: Length,
) -> Element<'a, Message> {
    Element::new(ScalePressButton {
        kind: ScalePressKind::Widget {
            content: content.into(),
        },
        on_press,
        dark,
        width,
        height,
    })
}

fn host_button_style(dark: bool, status: button::Status) -> button::Style {
    let base = if dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.10)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
    };
    let wash = match status {
        button::Status::Hovered => {
            if dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.08)
            }
        }
        button::Status::Pressed => {
            if dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.16)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.12)
            }
        }
        button::Status::Disabled => base.scale_alpha(0.5),
        button::Status::Active => base,
    };
    button::Style {
        background: Some(Background::Color(wash)),
        text_color: host_ink(dark),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, if dark { 0.04 } else { 0.0 }),
            width: 1.0,
            radius: border::radius(36),
        },
        ..button::Style::default()
    }
}

fn chat_field_style(dark: bool, _status: text_input::Status) -> text_input::Style {
    let value = host_ink(dark);
    let placeholder = if dark {
        Color::from_rgb8(0x4e, 0x4e, 0x4e)
    } else {
        Color::from_rgb8(0x9a, 0x9a, 0xa0)
    };
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(0),
        },
        icon: value,
        placeholder,
        value,
        selection: Color::from_rgba(0.35, 0.55, 1.0, 0.35),
    }
}

/// The host's own fill, ring and shadow: `.mock-vchat`.
fn chat_backdrop<'a>(w: f32, h: f32, dark: bool) -> Element<'a, Message> {
    backdrop(
        w,
        h,
        dark,
        CHAT_RADIUS as f32,
        Color::from_rgba8(48, 48, 48, 0.4),
        Color::from_rgba8(44, 47, 54, 0.52),
    )
}

/// `.mock-phone`: the screen's bottom crop, the top corners square.
fn phone_backdrop<'a>(w: f32, h: f32, dark: bool) -> Element<'a, Message> {
    let fill = if dark {
        Color::from_rgb8(0x11, 0x11, 0x11)
    } else {
        Color::WHITE
    };
    container(space::horizontal())
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .style(move |_| {
            container::Style::default()
                .background(Background::Color(fill))
                .border(Border {
                    color: Color::from_rgba8(44, 47, 54, if dark { 0.52 } else { 0.2 }),
                    width: 1.0,
                    radius: border::Radius {
                        top_left: 0.0,
                        top_right: 0.0,
                        bottom_right: PHONE_RADIUS as f32,
                        bottom_left: PHONE_RADIUS as f32,
                    },
                })
        })
        .into()
}

fn pill_backdrop<'a>(w: f32, h: f32, dark: bool) -> Element<'a, Message> {
    backdrop(
        w,
        h,
        dark,
        PILL_RADIUS as f32,
        Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        Color::from_rgba(1.0, 1.0, 1.0, 0.04),
    )
}

fn backdrop<'a>(
    w: f32,
    h: f32,
    dark: bool,
    radius: f32,
    fill: Color,
    ring: Color,
) -> Element<'a, Message> {
    let (fill, ring) = if dark {
        (fill, ring)
    } else {
        (
            Color::from_rgba(0.0, 0.0, 0.0, 0.05),
            Color::from_rgba(0.0, 0.0, 0.0, 0.08),
        )
    };
    container(space::horizontal())
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .style(move |_| {
            container::Style::default().background(fill).border(Border {
                color: ring,
                width: 1.0,
                radius: border::radius(radius),
            })
        })
        .into()
}

/// `.mock-pill`: mic icon, the running mm:ss counter and a real stop button.
fn pill_content<'a>(id: HostId, seconds: u64, w: f32, h: f32, dark: bool) -> Element<'a, Message> {
    let ink = if dark {
        Color::from_rgb8(0x87, 0x87, 0x87)
    } else {
        // Light theme: `rgba(29, 29, 29, 0.7)`.
        Color::from_rgba8(0x1d, 0x1d, 0x1d, 0.7)
    };
    let time = format!("{:02}:{:02}", seconds / 60, seconds % 60);
    // Dark: `#d9d9d9` @ 0.6. Light: `#0f0f0f` @ 0.8 (upstream `.mock-pill-stop::after`).
    let stop_fill = if dark {
        Color::from_rgba8(0xd9, 0xd9, 0xd9, 0.6)
    } else {
        Color::from_rgba8(0x0f, 0x0f, 0x0f, 0.8)
    };
    let stop = scale_press_button(
        container(
            container(space::horizontal())
                .width(Length::Fixed(11.0))
                .height(Length::Fixed(11.0))
                .style(move |_| {
                    container::Style::default()
                        .background(stop_fill)
                        .border(border::rounded(2))
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill),
        Message::Host(id, HostAction::Stop),
        dark,
        Length::Fixed(30.0),
        Length::Fixed(30.0),
    );

    container(
        row![
            lucide(LucideIcon::Mic, ink, 16.0),
            text(time).size(13).color(ink),
            space::horizontal(),
            stop,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 0.0,
        right: 7.0,
        bottom: 0.0,
        left: 13.0,
    })
    .width(Length::Fixed(w))
    .height(Length::Fixed(h))
    .center_y(Length::Fixed(h))
    .into()
}

// ── press-scale button (upstream `:active { transform: scale(0.96) }`) ──────

enum ScalePressKind<'a> {
    /// Lucide drawn with `fill_text` at the face centre (stable optical mid).
    Icon {
        icon: LucideIcon,
        color: Color,
        glyph: f32,
    },
    /// Labels / marks — layout stays at rest size; draw uses GPU scale about centre.
    Widget { content: Element<'a, Message> },
}

struct ScalePressButton<'a> {
    kind: ScalePressKind<'a>,
    on_press: Message,
    dark: bool,
    width: Length,
    height: Length,
}

#[derive(Debug, Clone)]
struct ScalePressState {
    is_pressed: bool,
    scale: f32,
    /// Value at the start of the current press/release transition.
    from: f32,
    /// Target of the current transition (`HOST_PRESS_SCALE` or `1.0`).
    to: f32,
    /// When the current transition started.
    started: Instant,
}

impl Default for ScalePressState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            is_pressed: false,
            scale: 1.0,
            from: 1.0,
            to: 1.0,
            started: now,
        }
    }
}

impl ScalePressState {
    fn status(&self, hovered: bool) -> button::Status {
        if self.is_pressed {
            button::Status::Pressed
        } else if hovered {
            button::Status::Hovered
        } else {
            button::Status::Active
        }
    }

    fn begin(&mut self, pressed: bool) {
        self.is_pressed = pressed;
        self.from = self.scale;
        self.to = if pressed { HOST_PRESS_SCALE } else { 1.0 };
        self.started = Instant::now();
    }

    fn tick(&mut self) -> bool {
        let target = self.to;
        if (self.scale - target).abs() < 0.00005 && self.from == self.to {
            return false;
        }
        let elapsed_ms = self.started.elapsed().as_secs_f32() * 1000.0;
        if elapsed_ms >= HOST_PRESS_MS {
            let changed = self.scale != target;
            self.scale = target;
            self.from = target;
            return changed;
        }
        let t = (elapsed_ms / HOST_PRESS_MS).clamp(0.0, 1.0);
        let eased = cubic_bezier_ease(t, HOST_PRESS_BEZIER);
        let next = self.from + (self.to - self.from) * eased;
        let changed = (next - self.scale).abs() > 0.00005;
        self.scale = next;
        changed
    }
}

/// Unit-interval Y for CSS `cubic-bezier(x1,y1,x2,y2)` (X solved by Newton).
fn cubic_bezier_ease(t: f32, (x1, y1, x2, y2): (f32, f32, f32, f32)) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let mut x = t;
    for _ in 0..8 {
        let sample = cubic_bezier_sample(x, x1, x2);
        let dx = cubic_bezier_slope(x, x1, x2);
        if dx.abs() < 1e-6 {
            break;
        }
        x = (x - (sample - t) / dx).clamp(0.0, 1.0);
    }
    cubic_bezier_sample(x, y1, y2)
}

fn cubic_bezier_sample(t: f32, a: f32, b: f32) -> f32 {
    // (1-t)^3·0 + 3(1-t)^2·t·a + 3(1-t)·t^2·b + t^3·1
    let u = 1.0 - t;
    3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
}

fn cubic_bezier_slope(t: f32, a: f32, b: f32) -> f32 {
    let u = 1.0 - t;
    3.0 * u * u * a + 6.0 * u * t * (b - a) + 3.0 * t * t * (1.0 - b)
}

impl<'a> Widget<Message, Theme, Renderer> for ScalePressButton<'a> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ScalePressState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ScalePressState::default())
    }

    fn children(&self) -> Vec<Tree> {
        match &self.kind {
            ScalePressKind::Icon { .. } => Vec::new(),
            ScalePressKind::Widget { content } => vec![Tree::new(content)],
        }
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.kind {
            ScalePressKind::Icon { .. } => {
                tree.diff_children(&[] as &[&dyn Widget<Message, Theme, Renderer>]);
            }
            ScalePressKind::Widget { content } => {
                tree.diff_children(&[content.as_widget()]);
            }
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        match &mut self.kind {
            ScalePressKind::Icon { .. } => {
                let size = limits.resolve(self.width, self.height, Size::ZERO);
                layout::Node::new(size)
            }
            ScalePressKind::Widget { content } => {
                layout::padded(limits, self.width, self.height, 0.0, |limits| {
                    content
                        .as_widget_mut()
                        .layout(&mut tree.children[0], renderer, limits)
                })
            }
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let ScalePressKind::Widget { content } = &mut self.kind {
            content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout.children().next().unwrap_or(layout),
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                return;
            }
        }

        let state = tree.state.downcast_mut::<ScalePressState>();
        let bounds = layout.bounds();
        let over = cursor.is_over(bounds);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. }) => {
                if over {
                    state.begin(true);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
                if state.is_pressed {
                    state.begin(false);
                    if over {
                        shell.publish(self.on_press.clone());
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Touch(iced::touch::Event::FingerLost { .. }) => {
                if state.is_pressed {
                    state.begin(false);
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(_)) if state.tick() => {
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<ScalePressState>();
        let bounds = layout.bounds();
        let hovered = cursor.is_over(bounds);
        let appearance = host_button_style(self.dark, state.status(hovered));
        let scale = state.scale.clamp(0.01, 1.0);

        // Hit box stays at rest size (CSS transform). Chrome insets about the
        // centre; icons are fill_text at that same centre with glyph*scale.
        let face = press_face(bounds, scale);
        if let Some(background) = appearance.background {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: face,
                    border: appearance.border,
                    shadow: appearance.shadow,
                    snap: appearance.snap,
                },
                background,
            );
        }

        match &self.kind {
            ScalePressKind::Icon { icon, color, glyph } => {
                draw_press_lucide(renderer, *icon, *color, *glyph * scale, bounds, *viewport);
            }
            ScalePressKind::Widget { content } => {
                let content_layout = layout.children().next().unwrap_or(layout);
                let draw_content = |renderer: &mut Renderer| {
                    content.as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        style,
                        content_layout,
                        cursor,
                        viewport,
                    );
                };
                if (scale - 1.0).abs() < 0.0005 {
                    draw_content(renderer);
                } else {
                    let cx = bounds.center_x();
                    let cy = bounds.center_y();
                    let transform = Transformation::translate(cx, cy)
                        * Transformation::scale(scale)
                        * Transformation::translate(-cx, -cy);
                    renderer.with_transformation(transform, draw_content);
                }
            }
        }
    }
}

fn draw_press_lucide(
    renderer: &mut Renderer,
    icon: LucideIcon,
    color: Color,
    size: f32,
    bounds: Rectangle,
    viewport: Rectangle,
) {
    let clip = bounds.intersection(&viewport).unwrap_or(bounds);
    renderer.fill_text(
        adv_text::Text {
            content: char::from(icon).to_string(),
            bounds: Size::new(size * 2.0, size * 2.0),
            size: Pixels(size),
            line_height: adv_text::LineHeight::Absolute(Pixels(size)),
            font: Font::with_name("lucide"),
            align_x: adv_text::Alignment::Center,
            align_y: Alignment::Center.into(),
            shaping: adv_text::Shaping::Advanced,
            wrapping: adv_text::Wrapping::None,
        },
        Point::new(bounds.center_x(), bounds.center_y()),
        color,
        clip,
    );
}

/// Layout-stable CSS `scale(s)` of `bounds` about its centre.
fn press_face(bounds: Rectangle, scale: f32) -> Rectangle {
    let width = bounds.width * scale;
    let height = bounds.height * scale;
    Rectangle {
        x: bounds.center_x() - width * 0.5,
        y: bounds.center_y() - height * 0.5,
        width,
        height,
    }
}

impl<'a> From<ScalePressButton<'a>> for Element<'a, Message> {
    fn from(button: ScalePressButton<'a>) -> Self {
        Element::new(button)
    }
}
