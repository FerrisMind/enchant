//! Bot avatars: all eighteen shapes, every shading mode and the wardrobe.
//!
//! ```text
//! cargo run -p enchant-iced --example bot_avatar --release
//! cargo run -p enchant-iced --example bot_avatar --release -- --sheet
//! ```
//!
//! The window shows a large selected avatar and the full gallery. Everything
//! on the right is live: states, shading, hats, glasses, headphones, the bow
//! tie, the whirl, speed, pause and reduced motion. Clicking a bot pokes it
//! through the widget's own `on_click` hook (it also hops on its own).
//!
//! `--sheet` renders deterministic PNG sheets into
//! `target/bot-avatar-sheets/` instead of opening a window. They are the
//! comparison artifact for the web reference: the same `draw_frame` the
//! widget uses, composited on the dark page colour.

mod _shared;

use std::path::PathBuf;
use std::time::Duration;

use enchant_iced::widgets::bot_avatar::{
    BOT_AVATAR_TYPES, BotAvatar, BotAvatarGlasses, BotAvatarHat, BotAvatarShading, BotAvatarState,
    BotAvatarTheme, BotAvatarType, Pose, bot_avatar_preset, rest_pose,
};
use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length, Subscription, Task};
use iced_shadcn::{ButtonProps, ButtonSize, ButtonVariant, button_content};

pub fn main() -> iced::Result {
    if std::env::args().any(|arg| arg == "--sheet") {
        match write_sheets() {
            Ok(paths) => {
                for path in paths {
                    println!("{}", path.display());
                }
            }
            Err(error) => {
                eprintln!("could not write the sheets: {error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    iced::application(Demo::new, Demo::update, Demo::view)
        .title("Bot Avatars · Libraries.dev")
        .window(iced::window::Settings {
            size: iced::Size::new(1240.0, 980.0),
            ..Default::default()
        })
        .subscription(Demo::subscription)
        .run()
}

const GRID_SIZE: f32 = 78.0;
const HERO_SIZE: f32 = 176.0;

struct Demo {
    theme_shell: _shared::ThemeShell,
    selected: BotAvatarType,
    state: BotAvatarState,
    shading: BotAvatarShading,
    hat: BotAvatarHat,
    glasses: BotAvatarGlasses,
    headphones: bool,
    bow_tie: bool,
    paused: bool,
    reduced_motion: bool,
    speed: f32,
    theme: BotAvatarTheme,
    whirl: f32,
    pokes: u32,
}

#[derive(Clone, Debug)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(iced::time::Instant),
    Select(BotAvatarType),
    Poke(BotAvatarType),
    SetState(BotAvatarState),
    SetShading(BotAvatarShading),
    SetHat(BotAvatarHat),
    SetGlasses(BotAvatarGlasses),
    ToggleHeadphones,
    ToggleBowTie,
    TogglePaused,
    ToggleReducedMotion,
    SetSpeed(f32),
    SetTheme(BotAvatarTheme),
    SetWhirl(f32),
}

impl Demo {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                selected: BotAvatarType::Clover,
                state: BotAvatarState::Idle,
                shading: BotAvatarShading::Fabric,
                hat: BotAvatarHat::None,
                glasses: BotAvatarGlasses::None,
                headphones: false,
                bow_tie: false,
                paused: false,
                reduced_motion: false,
                speed: 1.0,
                theme: BotAvatarTheme::Auto,
                whirl: 0.0,
                pokes: 0,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => self.theme_shell.tick(now),
            Message::Select(bot_type) => self.selected = bot_type,
            Message::Poke(bot_type) => {
                self.selected = bot_type;
                self.pokes = self.pokes.saturating_add(1);
            }
            Message::SetState(state) => self.state = state,
            Message::SetShading(shading) => self.shading = shading,
            Message::SetHat(hat) => self.hat = hat,
            Message::SetGlasses(glasses) => self.glasses = glasses,
            Message::ToggleHeadphones => self.headphones = !self.headphones,
            Message::ToggleBowTie => self.bow_tie = !self.bow_tie,
            Message::TogglePaused => self.paused = !self.paused,
            Message::ToggleReducedMotion => self.reduced_motion = !self.reduced_motion,
            Message::SetSpeed(speed) => self.speed = speed,
            Message::SetTheme(theme) => self.theme = theme,
            Message::SetWhirl(whirl) => self.whirl = whirl,
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let transition = self
            .theme_shell
            .controller
            .is_transition_active(self.theme_shell.now);
        if transition {
            iced::time::every(Duration::from_millis(16)).map(Message::Tick)
        } else {
            Subscription::none()
        }
    }

    fn avatar_theme(&self) -> BotAvatarTheme {
        match self.theme {
            BotAvatarTheme::Auto => {
                if self.theme_shell.controller.dark_mode() {
                    BotAvatarTheme::Dark
                } else {
                    BotAvatarTheme::Light
                }
            }
            other => other,
        }
    }

    fn avatar(&self, bot_type: BotAvatarType) -> BotAvatar<Message> {
        BotAvatar::new()
            .bot_type(bot_type)
            .state(self.state)
            .shading(self.shading)
            .hat(self.hat)
            .glasses(self.glasses)
            .headphones(self.headphones)
            .bow_tie(self.bow_tie)
            .speed(self.speed)
            .paused(self.paused)
            .reduce_motion(self.reduced_motion)
            .whirl(self.whirl)
            .theme(self.avatar_theme())
            .on_click(move || Message::Poke(bot_type))
    }

    fn state_label(&self) -> &'static str {
        match self.state {
            BotAvatarState::Idle => "Idle",
            BotAvatarState::Working => "Working",
            BotAvatarState::Sleeping => "Sleeping",
            _ => enchant_iced::widgets::bot_avatar::bot_avatar_state_label(self.state),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let dark = self.theme_shell.controller.dark_mode();
        let muted = if dark {
            Color::from_rgb(0.64, 0.66, 0.72)
        } else {
            Color::from_rgb(0.34, 0.36, 0.42)
        };
        let selected_preset = bot_avatar_preset(self.selected);

        let hero = self
            .avatar(self.selected)
            .size(HERO_SIZE)
            .seed(0.37);
        let hero_info = column![
            text(selected_preset.label).size(26),
            text(format!(
                "{} bot · {} · click any bot to poke it",
                selected_preset.label,
                self.state_label()
            ))
            .size(13)
            .style({
                let muted = muted;
                move |_| iced::widget::text::Style { color: Some(muted) }
            }),
            text(format!("Pokes: {}", self.pokes))
                .size(13)
                .style({
                    let muted = muted;
                    move |_| iced::widget::text::Style { color: Some(muted) }
                }),
            Space::new().height(6),
            row![
                self.control(
                    "Idle",
                    Message::SetState(BotAvatarState::Idle),
                    self.state == BotAvatarState::Idle
                ),
                self.control(
                    "Working",
                    Message::SetState(BotAvatarState::Working),
                    self.state == BotAvatarState::Working
                ),
                self.control(
                    "Sleeping",
                    Message::SetState(BotAvatarState::Sleeping),
                    self.state == BotAvatarState::Sleeping
                ),
            ]
            .spacing(6),
            row![
                self.speed_control(0.5),
                self.speed_control(1.0),
                self.speed_control(2.0),
                self.speed_control(4.0),
            ]
            .spacing(6),
        ]
        .spacing(8)
        .align_x(Alignment::Start);

        let hero_row = row![container(hero).padding(4), hero_info]
            .spacing(28)
            .align_y(Alignment::Center);

        let mut grid_rows = Vec::new();
        for chunk in BOT_AVATAR_TYPES.chunks(6) {
            let mut cells = Vec::new();
            for bot_type in chunk {
                cells.push(self.cell(*bot_type));
            }
            grid_rows.push(row(cells).spacing(10).into());
        }
        let grid = column(grid_rows).spacing(10);

        let shading_row = row(BotAvatarShading::ALL
            .iter()
            .map(|shading| {
                self.control(
                    match shading {
                        BotAvatarShading::Crisp => "Crisp",
                        BotAvatarShading::Smooth => "Smooth",
                        BotAvatarShading::Plastic => "Plastic",
                        BotAvatarShading::Flat => "Flat",
                        BotAvatarShading::Fabric => "Fabric",
                        _ => "Custom",
                    },
                    Message::SetShading(*shading),
                    self.shading == *shading,
                )
            })
            .collect::<Vec<_>>())
        .spacing(6);

        let hat_row = row(BotAvatarHat::ALL
            .iter()
            .map(|hat| {
                self.control(
                    match hat {
                        BotAvatarHat::None => "None",
                        BotAvatarHat::Beret => "Beret",
                        BotAvatarHat::Beanie => "Beanie",
                        BotAvatarHat::Party => "Party",
                        BotAvatarHat::Crown => "Crown",
                        _ => "Custom",
                    },
                    Message::SetHat(*hat),
                    self.hat == *hat,
                )
            })
            .collect::<Vec<_>>())
        .spacing(6);

        let glasses_row = row(BotAvatarGlasses::ALL
            .iter()
            .map(|glasses| {
                self.control(
                    match glasses {
                        BotAvatarGlasses::None => "None",
                        BotAvatarGlasses::Round => "Round",
                        BotAvatarGlasses::Square => "Square",
                        BotAvatarGlasses::Shades => "Shades",
                        _ => "Custom",
                    },
                    Message::SetGlasses(*glasses),
                    self.glasses == *glasses,
                )
            })
            .collect::<Vec<_>>())
        .spacing(6);

        let wear_row = row![
            self.control("Headphones", Message::ToggleHeadphones, self.headphones),
            self.control("Bow tie", Message::ToggleBowTie, self.bow_tie),
            self.control("Pause", Message::TogglePaused, self.paused),
            self.control(
                "Reduced motion",
                Message::ToggleReducedMotion,
                self.reduced_motion
            ),
        ]
        .spacing(6);

        let whirl_row = row![
            self.control("Off", Message::SetWhirl(0.0), self.whirl == 0.0),
            self.control("Soft", Message::SetWhirl(0.6), self.whirl == 0.6),
            self.control("Strong", Message::SetWhirl(1.4), self.whirl == 1.4),
        ]
        .spacing(6);

        let theme_row = row![
            self.control(
                "Auto",
                Message::SetTheme(BotAvatarTheme::Auto),
                self.theme == BotAvatarTheme::Auto
            ),
            self.control(
                "Dark",
                Message::SetTheme(BotAvatarTheme::Dark),
                self.theme == BotAvatarTheme::Dark
            ),
            self.control(
                "Light",
                Message::SetTheme(BotAvatarTheme::Light),
                self.theme == BotAvatarTheme::Light
            ),
        ]
        .spacing(6);

        let controls = column![
            labeled("Shading", shading_row.into()),
            labeled("Hat", hat_row.into()),
            labeled("Glasses", glasses_row.into()),
            labeled("Wear & motion", wear_row.into()),
            labeled("Whirl", whirl_row.into()),
            labeled("Surface", theme_row.into()),
        ]
        .spacing(8);

        let body = column![
            hero_row,
            grid,
            controls,
            text(
                "Every avatar owns its clock and asks for a redraw only while visible. \
                 The raster is overscanned 1.5× and pulled back with negative margins, as on the web."
            )
            .size(12)
            .style({
                let muted = muted;
                move |_| iced::widget::text::Style { color: Some(muted) }
            }),
        ]
        .spacing(20)
        .align_x(Alignment::Center);

        let page = scrollable(
            container(body)
                .width(Length::Fill)
                .padding([8, 24])
                .center_x(Length::Fill),
        );
        _shared::themed_shell("Bot Avatars", page, &self.theme_shell, Message::ToggleTheme)
    }

    fn speed_control(&self, speed: f32) -> Element<'_, Message> {
        self.control(
            format!("{speed}×"),
            Message::SetSpeed(speed),
            (self.speed - speed).abs() < f32::EPSILON,
        )
    }

    fn cell(&self, bot_type: BotAvatarType) -> Element<'_, Message> {
        let selected = self.selected == bot_type;
        let dark = self.theme_shell.controller.dark_mode();
        let border_color = if dark {
            Color::from_rgba(1.0, 1.0, 1.0, if selected { 0.55 } else { 0.10 })
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, if selected { 0.45 } else { 0.08 })
        };
        let label_color = if dark {
            Color::from_rgb(0.72, 0.74, 0.80)
        } else {
            Color::from_rgb(0.30, 0.32, 0.38)
        };
        let avatar = self
            .avatar(bot_type)
            .size(GRID_SIZE)
            .seed(seed_for(bot_type));
        container(
            column![
                avatar,
                text(bot_avatar_preset(bot_type).label)
                    .size(11)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(label_color),
                    }),
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .padding([8, 10])
        .style(move |_| iced::widget::container::Style {
            border: Border {
                color: border_color,
                width: if selected { 1.5 } else { 1.0 },
                radius: 18.0.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into()
    }

    fn control(
        &self,
        label: impl Into<String>,
        message: Message,
        selected: bool,
    ) -> Element<'_, Message> {
        button_content(
            text(label.into()).size(12),
            Some(message),
            ButtonProps::new()
                .variant(if selected {
                    ButtonVariant::Default
                } else {
                    ButtonVariant::Outline
                })
                .size(ButtonSize::Size1),
            self.theme_shell.theme(),
        )
        .into()
    }
}

fn labeled<'a>(label: &'static str, controls: Element<'a, Message>) -> Element<'a, Message> {
    row![
        container(text(label).size(12)).width(Length::Fixed(96.0)),
        controls,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn seed_for(bot_type: BotAvatarType) -> f32 {
    let key = bot_type.key().as_bytes();
    let mut hash = 2_166_136_261u32;
    for byte in key {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash % 1000) as f32 / 1000.0
}

fn write_sheets() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/bot-avatar-sheets");
    std::fs::create_dir_all(&dir)?;
    let rest = rest_pose(BotAvatarState::Idle);
    let mut written = Vec::new();

    let mut types: Vec<BotAvatar<()>> = Vec::new();
    for bot_type in BotAvatarType::ALL {
        types.push(
            BotAvatar::new()
                .bot_type(bot_type)
                .shading(BotAvatarShading::Fabric)
                .seed(0.37),
        );
    }
    written.push(write_grid(&dir, "types-fabric.png", &types, rest, 96.0, 6)?);

    let mut shading: Vec<BotAvatar<()>> = Vec::new();
    for mode in BotAvatarShading::ALL {
        shading.push(
            BotAvatar::new()
                .bot_type(BotAvatarType::Clover)
                .shading(mode)
                .seed(0.37),
        );
    }
    written.push(write_grid(&dir, "shading.png", &shading, rest, 112.0, 5)?);

    let mut wear: Vec<BotAvatar<()>> = Vec::new();
    for hat in BotAvatarHat::ALL {
        wear.push(
            BotAvatar::new()
                .bot_type(BotAvatarType::Cat)
                .hat(hat)
                .shading(BotAvatarShading::Fabric)
                .seed(0.37),
        );
    }
    for glasses in BotAvatarGlasses::ALL {
        wear.push(
            BotAvatar::new()
                .bot_type(BotAvatarType::Cat)
                .glasses(glasses)
                .shading(BotAvatarShading::Fabric)
                .seed(0.37),
        );
    }
    wear.push(
        BotAvatar::new()
            .bot_type(BotAvatarType::Cat)
            .headphones(true)
            .bow_tie(true)
            .shading(BotAvatarShading::Fabric)
            .seed(0.37),
    );
    written.push(write_grid(&dir, "wear.png", &wear, rest, 112.0, 5)?);

    let mut states: Vec<(BotAvatar<()>, Pose)> = Vec::new();
    for state in BotAvatarState::ALL {
        for mode in [BotAvatarShading::Plastic, BotAvatarShading::Fabric] {
            states.push((
                BotAvatar::new()
                    .bot_type(BotAvatarType::Cat)
                    .state(state)
                    .shading(mode)
                    .seed(0.37),
                rest_pose(state),
            ));
        }
    }
    written.push(write_posed_grid(&dir, "states.png", &states, 128.0, 3)?);

    let hero: BotAvatar<()> = BotAvatar::new()
        .bot_type(BotAvatarType::Cat)
        .size(HERO_SIZE)
        .shading(BotAvatarShading::Fabric)
        .hat(BotAvatarHat::Beret)
        .glasses(BotAvatarGlasses::Round)
        .bow_tie(true)
        .seed(0.37);
    written.push(write_posed_grid(
        &dir,
        "hero.png",
        &[(hero, rest)],
        HERO_SIZE,
        1,
    )?);

    Ok(written)
}

fn write_grid(
    dir: &std::path::Path,
    name: &str,
    avatars: &[BotAvatar<()>],
    pose: Pose,
    size: f32,
    columns: usize,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let rendered = render_all(avatars.iter().map(|avatar| (avatar, pose)), size);
    compose(dir, name, &rendered, columns)
}

fn write_posed_grid(
    dir: &std::path::Path,
    name: &str,
    avatars: &[(BotAvatar<()>, Pose)],
    size: f32,
    columns: usize,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let rendered = render_all(avatars.iter().map(|(avatar, pose)| (avatar, *pose)), size);
    compose(dir, name, &rendered, columns)
}

fn render_all<'a>(
    avatars: impl Iterator<Item = (&'a BotAvatar<()>, Pose)>,
    size: f32,
) -> Vec<(u32, u32, Vec<u8>)> {
    let density = if size >= 160.0 { 2.0 } else { 1.0 };
    avatars
        .map(|(avatar, pose)| avatar.render_rgba(size, density, &pose))
        .collect()
}

fn compose(
    dir: &std::path::Path,
    name: &str,
    rendered: &[(u32, u32, Vec<u8>)],
    columns: usize,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cell_w = rendered
        .iter()
        .map(|(w, _, _)| *w)
        .max()
        .unwrap_or(1)
        .max(8);
    let cell_h = rendered
        .iter()
        .map(|(_, h, _)| *h)
        .max()
        .unwrap_or(1)
        .max(8);
    let columns = columns.max(1);
    let rows = rendered.len().div_ceil(columns);
    let pad = 12u32;
    let width = columns as u32 * cell_w + (columns as u32 + 1) * pad;
    let height = rows as u32 * cell_h + (rows as u32 + 1) * pad;
    let bg = [11u8, 11, 15, 255];
    let mut canvas = vec![0u8; (width * height * 4) as usize];
    for pixel in canvas.chunks_exact_mut(4) {
        pixel.copy_from_slice(&bg);
    }
    for (index, (w, h, rgba)) in rendered.iter().enumerate() {
        let column = (index % columns) as u32;
        let row = (index / columns) as u32;
        let ox = pad + column * (cell_w + pad) + (cell_w - *w) / 2;
        let oy = pad + row * (cell_h + pad) + (cell_h - *h) / 2;
        for y in 0..*h {
            for x in 0..*w {
                let src = ((y * *w + x) * 4) as usize;
                let alpha = u16::from(rgba[src + 3]);
                if alpha == 0 {
                    continue;
                }
                let dst = (((oy + y) * width + ox + x) * 4) as usize;
                let inv = 255 - alpha;
                for channel in 0..3 {
                    canvas[dst + channel] = ((u16::from(rgba[src + channel]) * alpha
                        + u16::from(canvas[dst + channel]) * inv
                        + 127)
                        / 255) as u8;
                }
                canvas[dst + 3] = 255;
            }
        }
    }
    let path = dir.join(name);
    let image =
        image::RgbaImage::from_raw(width, height, canvas).ok_or("could not build the sheet")?;
    image.save(&path)?;
    Ok(path)
}
