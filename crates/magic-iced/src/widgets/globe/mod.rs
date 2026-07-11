mod card_layer;
mod pipeline;
mod primitive;
mod program;
mod vignette_layer;

pub use pipeline::lat_lon_to_unit;

use crate::widgets::AnimatedThemeTogglerController;
use card_layer::globe_card_layer;
use iced::advanced::graphics::gradient as canvas_gradient;
use iced::alignment::Vertical;
use iced::mouse;
use iced::time::Instant;
use iced::widget::{Space, canvas, container, shader, stack};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer,
};
use iced_shadcn::Theme;
use program::GlobeShaderProgram;
use vignette_layer::globe_vignette_layer;

/// Cobe `MOVEMENT_DAMPING` from Magic UI `globe.tsx`.
const MOVEMENT_DAMPING: f32 = 1400.0;

/// Magic UI `globe-demo` card (measured on magicui.design preview).
const DEMO_CARD_WIDTH: f32 = 496.0;
const DEMO_CARD_HEIGHT: f32 = 290.0;
/// Square Cobe canvas — full render resolution (CSS size in Magic UI).
pub const GLOBE_CANVAS_SIDE: f32 = 494.0;
/// Visible globe band = top 2/3 of the card; bottom 1/3 of the sphere is clipped.
const GLOBE_VISIBLE_CARD_FRACTION: f32 = 2.0 / 3.0;
const DEMO_CARD_RADIUS: f32 = 12.0;
/// Outer shell uses `padding(1)`; inner layers should match the inset corner radius.
pub(crate) const DEMO_CARD_INNER_RADIUS: f32 = DEMO_CARD_RADIUS - 1.0;

fn globe_top_offset(card_height: f32) -> f32 {
    card_height * (1.0 - GLOBE_VISIBLE_CARD_FRACTION)
}
/// `pt-8` on the demo card.
const DEMO_TITLE_PADDING_TOP: f32 = 32.0;
/// `text-8xl` title size.
const DEMO_TITLE_SIZE: u16 = 96;

/// Fixed Cobe palette from `GLOBE_CONFIG` — does not follow page theme.
pub(crate) const DEFAULT_BASE: Color = Color::WHITE;
pub(crate) const DEFAULT_GLOW: Color = Color::from_rgba(1.0, 1.0, 1.0, 1.0);
pub(crate) const DEFAULT_MARKER: Color = Color::from_rgb8(0xFB, 0x64, 0x15);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobeMarker {
    pub lat: f32,
    pub lng: f32,
    pub size: f32,
}

impl GlobeMarker {
    pub const fn new(lat: f32, lng: f32, size: f32) -> Self {
        Self { lat, lng, size }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlobeAction {
    Move(Point),
    Press,
    Release,
    Exit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlobeState {
    /// Horizontal rotation (`phi` in COBE).
    pub phi: f32,
    /// Tilt (`theta` in COBE, Magic UI default `0.3`).
    pub theta: f32,
    pointer: Option<Point>,
    dragging: bool,
    last_tick_at: Option<Instant>,
    mount_at: Instant,
    pub opacity: f32,
}

impl GlobeState {
    pub fn new(_props: &GlobeProps) -> Self {
        Self {
            phi: 0.0,
            theta: 0.3,
            pointer: None,
            dragging: false,
            last_tick_at: None,
            mount_at: Instant::now(),
            opacity: 0.0,
        }
    }

    pub fn apply_action(&mut self, action: &GlobeAction, props: &GlobeProps) {
        match action {
            GlobeAction::Move(position) => {
                if let Some(last) = self.pointer
                    && self.dragging
                {
                    let delta_x = position.x - last.x;
                    self.phi += delta_x * props.drag_sensitivity;
                }
                self.pointer = Some(*position);
            }
            GlobeAction::Press => self.dragging = true,
            GlobeAction::Release | GlobeAction::Exit => {
                self.dragging = false;
                if matches!(action, GlobeAction::Exit) {
                    self.pointer = None;
                }
            }
        }
    }

    pub fn tick(&mut self, now: Instant, props: &GlobeProps) {
        let dt = self
            .last_tick_at
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0);
        self.last_tick_at = Some(now);
        self.opacity =
            (now.saturating_duration_since(self.mount_at).as_secs_f32() / 0.5).clamp(0.0, 1.0);

        if !self.dragging {
            // COBE: `phi += 0.005` per frame (~60fps) ≈ 0.3 rad/s.
            self.phi += props.auto_rotate_speed * dt;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlobeProps {
    pub width: Length,
    pub height: Length,
    pub map_samples: usize,
    pub map_brightness: f32,
    pub map_base_brightness: f32,
    pub diffuse: f32,
    pub dark: f32,
    pub scale: f32,
    pub offset: [f32; 2],
    pub marker_elevation: f32,
    pub device_pixel_ratio: f32,
    pub auto_rotate_speed: f32,
    pub drag_sensitivity: f32,
    pub base_color: Option<Color>,
    pub marker_color: Option<Color>,
    pub glow_color: Option<Color>,
    /// COBE outer halo outside the sphere (`glowFactor` in `globe.frag.glslx`).
    pub outer_glow: bool,
    /// Canvas fill behind the globe (Magic UI: transparent canvas over `bg-background`).
    pub canvas_background: Option<Color>,
    pub markers: Vec<GlobeMarker>,
}

impl Default for GlobeProps {
    fn default() -> Self {
        Self {
            width: Length::Fixed(GLOBE_CANVAS_SIDE),
            height: Length::Fixed(GLOBE_CANVAS_SIDE),
            map_samples: 16_000,
            map_brightness: 1.2,
            map_base_brightness: 0.0,
            diffuse: 0.4,
            dark: 0.0,
            scale: 1.0,
            offset: [0.0, 0.0],
            // COBE default is 0.05; Magic UI globe demos read as on-surface at 0.
            marker_elevation: 0.0,
            device_pixel_ratio: 2.0,
            auto_rotate_speed: 0.3,
            drag_sensitivity: 1.0 / MOVEMENT_DAMPING,
            base_color: Some(DEFAULT_BASE),
            marker_color: Some(DEFAULT_MARKER),
            glow_color: Some(DEFAULT_GLOW),
            outer_glow: false,
            canvas_background: None,
            markers: default_markers(),
        }
    }
}

impl GlobeProps {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Shell colors for [`globe_demo`]. These follow the page theme and must be blended
/// during animated theme transitions. The globe canvas keeps fixed Cobe colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobeDemoColors {
    pub background: Color,
    pub border: Color,
    pub title_top: Color,
    pub title_bottom: Color,
    pub dark_mode: bool,
}

pub fn globe_demo_colors_from_theme(theme: &Theme, dark_mode: bool) -> GlobeDemoColors {
    GlobeDemoColors {
        background: theme.palette.background,
        border: apparent_border_on_background(theme.palette.border, theme.palette.background),
        title_top: if dark_mode {
            Color::WHITE
        } else {
            Color::BLACK
        },
        title_bottom: if dark_mode {
            Color::from_rgba(15.0 / 255.0, 23.0 / 255.0, 42.0 / 255.0, 0.10)
        } else {
            Color::from_rgba(209.0 / 255.0, 213.0 / 255.0, 219.0 / 255.0, 0.80)
        },
        dark_mode,
    }
}

pub fn globe_demo_colors_for_mode(dark_mode: bool) -> GlobeDemoColors {
    let theme = if dark_mode {
        crate::theme::MagicTheme::dark()
    } else {
        crate::theme::MagicTheme::light()
    };
    globe_demo_colors_from_theme(&theme, dark_mode)
}

/// Shell colors synced with [`AnimatedThemeTogglerController::display_theme`].
///
/// Use when the rest of the page keeps the previous theme during circle-reveal
/// (for example the wasm demo shell).
pub fn globe_demo_colors_for_controller(
    controller: &AnimatedThemeTogglerController,
    now: Instant,
) -> GlobeDemoColors {
    let theme = controller.display_theme(now);
    let dark_mode = if controller.is_transition_active(now) {
        !controller.dark_mode()
    } else {
        controller.dark_mode()
    };
    globe_demo_colors_from_theme(&theme, dark_mode)
}

/// Shell colors interpolated with circle-reveal progress.
pub fn globe_demo_colors_blended(
    controller: &AnimatedThemeTogglerController,
    now: Instant,
    theme_for_mode: impl Fn(bool) -> Theme,
) -> GlobeDemoColors {
    let target_dark = controller.dark_mode();
    let target = globe_demo_colors_from_theme(&theme_for_mode(target_dark), target_dark);
    if !controller.is_transition_active(now) {
        return target;
    }

    let source_dark = !target_dark;
    let source = globe_demo_colors_from_theme(&theme_for_mode(source_dark), source_dark);
    blend_globe_demo_colors(source, target, controller.reveal_at(now))
}

pub fn blend_globe_demo_colors(
    from: GlobeDemoColors,
    to: GlobeDemoColors,
    progress: f32,
) -> GlobeDemoColors {
    let blend = |left: Color, right: Color| blend_colors(left, right, progress);
    GlobeDemoColors {
        background: blend(from.background, to.background),
        border: blend(from.border, to.border),
        title_top: blend(from.title_top, to.title_top),
        title_bottom: blend(from.title_bottom, to.title_bottom),
        dark_mode: if progress >= 0.5 {
            to.dark_mode
        } else {
            from.dark_mode
        },
    }
}

/// COBE globe rendered via iced `widget::shader` + WGSL (ported from upstream cobe).
pub fn globe<'a, Message: Clone + 'static>(
    state: &'a GlobeState,
    props: &'a GlobeProps,
    on_action: impl Fn(GlobeAction) -> Message + 'static + 'a,
) -> Element<'a, Message> {
    let shader = shader(GlobeShaderProgram {
        state,
        props,
        on_action: Box::new(on_action),
    })
    .width(props.width)
    .height(props.height);

    match props.canvas_background {
        Some(background) => container(shader)
            .width(props.width)
            .height(props.height)
            .style(move |_| container::Style {
                background: Some(Background::Color(background)),
                ..container::Style::default()
            })
            .into(),
        None => shader.into(),
    }
}

/// Magic UI `globe-demo` shell.
pub fn globe_demo<'a, Message: Clone + 'static>(
    state: &'a GlobeState,
    props: &'a GlobeProps,
    colors: GlobeDemoColors,
    on_action: impl Fn(GlobeAction) -> Message + 'static + 'a,
) -> Element<'a, Message> {
    let title = demo_title(colors.title_top, colors.title_bottom);
    let top_offset = globe_top_offset(DEMO_CARD_HEIGHT);

    let title_layer = container(title)
        .width(Length::Fill)
        .padding([DEMO_TITLE_PADDING_TOP, 0.0])
        .center_x(Length::Fill);

    let stage = stack![
        Space::new().width(Length::Fill).height(Length::Fill),
        globe_card_layer(state, props, top_offset, on_action),
        title_layer,
        globe_vignette_layer(),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .clip(true);

    container(stage)
        .width(Length::Fixed(DEMO_CARD_WIDTH))
        .height(Length::Fixed(DEMO_CARD_HEIGHT))
        .padding(1)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(Background::Color(colors.background)),
            border: Border {
                width: 1.0,
                color: colors.border,
                radius: DEMO_CARD_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}

impl shader::Pipeline for pipeline::GlobePipeline {
    fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self {
        Self::new(device, queue, format)
    }
}

fn demo_title<'a, Message: 'a>(top: Color, bottom: Color) -> Element<'a, Message> {
    canvas(GlobeTitleProgram { top, bottom })
        .width(Length::Fill)
        .height(Length::Fixed(DEMO_TITLE_SIZE as f32 * 1.05))
        .into()
}

#[derive(Clone, Copy, Debug)]
struct GlobeTitleProgram {
    top: Color,
    bottom: Color,
}

impl<Message> canvas::Program<Message> for GlobeTitleProgram {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let font = Font {
            weight: iced::font::Weight::Semibold,
            ..Font::DEFAULT
        };
        let font_size = DEMO_TITLE_SIZE as f32;
        let mut paths = Vec::new();
        canvas::Text {
            content: "Globe".into(),
            position: Point::new(bounds.width / 2.0, 0.0),
            color: Color::TRANSPARENT,
            size: Pixels(font_size),
            line_height: iced::widget::text::LineHeight::Relative(1.0),
            font,
            align_x: Alignment::Center.into(),
            align_y: Vertical::Top,
            ..canvas::Text::default()
        }
        .draw_with(|path, _| paths.push(path));

        // Magic UI `bg-linear-to-b`: black/white at top → muted gray at bottom.
        let fill = canvas::Fill::from(
            canvas_gradient::Linear::new(
                Point::new(bounds.width / 2.0, 0.0),
                Point::new(bounds.width / 2.0, bounds.height),
            )
            .add_stop(0.0, self.top)
            .add_stop(1.0, self.bottom),
        );

        for path in &paths {
            frame.fill(path, fill);
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

fn default_markers() -> Vec<GlobeMarker> {
    vec![
        GlobeMarker::new(14.5995, 120.9842, 0.03),
        GlobeMarker::new(19.0760, 72.8777, 0.10),
        GlobeMarker::new(23.8103, 90.4125, 0.05),
        GlobeMarker::new(30.0444, 31.2357, 0.07),
        GlobeMarker::new(39.9042, 116.4074, 0.08),
        GlobeMarker::new(-23.5505, -46.6333, 0.10),
        GlobeMarker::new(19.4326, -99.1332, 0.10),
        GlobeMarker::new(40.7128, -74.0060, 0.10),
        GlobeMarker::new(34.6937, 135.5022, 0.05),
        GlobeMarker::new(41.0082, 28.9784, 0.06),
    ]
}

fn apparent_border_on_background(border: Color, background: Color) -> Color {
    let alpha = border.a.clamp(0.0, 1.0);
    Color {
        r: border.r * alpha + background.r * (1.0 - alpha),
        g: border.g * alpha + background.g * (1.0 - alpha),
        b: border.b * alpha + background.b * (1.0 - alpha),
        a: 1.0,
    }
}

fn blend_colors(from: Color, to: Color, progress: f32) -> Color {
    let t = progress.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globe_defaults_match_magic_ui_config() {
        let props = GlobeProps::new();

        assert_eq!(props.map_samples, 16_000);
        assert!((props.map_brightness - 1.2).abs() < f32::EPSILON);
        assert!((props.diffuse - 0.4).abs() < f32::EPSILON);
        assert!((props.auto_rotate_speed - 0.3).abs() < f32::EPSILON);
        assert!((props.drag_sensitivity - 1.0 / MOVEMENT_DAMPING).abs() < f32::EPSILON);
        assert_eq!(props.markers.len(), 10);
        assert_eq!(props.dark, 0.0);
    }

    #[test]
    fn globe_drag_updates_phi_only() {
        let props = GlobeProps::new();
        let mut state = GlobeState::new(&props);

        state.apply_action(&GlobeAction::Move(Point::new(10.0, 10.0)), &props);
        state.apply_action(&GlobeAction::Press, &props);
        state.apply_action(&GlobeAction::Move(Point::new(30.0, 20.0)), &props);

        assert!(state.phi > 0.0);
        assert_eq!(state.theta, 0.3);
    }

    #[test]
    fn lat_lon_to_unit_matches_cobe_convention() {
        let pos = lat_lon_to_unit(0.0, 180.0);
        assert!(pos[0] < -0.99);
        assert!(pos[1].abs() < 0.01);
    }

    /// COBE `globe.frag.glslx` map UV + unflipped WebGPU texture upload (PNG top = v=0).
    #[test]
    fn cobe_map_samples_land_at_magic_ui_markers() {
        const COBE_MAP: &[u8] = include_bytes!("../../../assets/cobe_map.png");
        let img = image::load_from_memory(COBE_MAP)
            .expect("cobe map")
            .to_luma8();
        let (w, h) = img.dimensions();

        for marker in default_markers() {
            let [x, y, z] = lat_lon_to_unit(marker.lat, marker.lng);
            let g_phi = y.clamp(-1.0, 1.0).asin();
            let mut g_theta = (-x / g_phi.cos()).clamp(-1.0, 1.0).acos();
            if z < 0.0 {
                g_theta = -g_theta;
            }
            let u = (g_theta * 0.5) / std::f32::consts::PI;
            let v = -(g_phi / std::f32::consts::PI + 0.5);
            let px = ((u.rem_euclid(1.0)) * w as f32) as u32 % w;
            let py = ((v.rem_euclid(1.0)) * h as f32) as u32 % h;
            let mut peak = 0u8;
            for dy in -4i32..=4 {
                for dx in -4i32..=4 {
                    let sx = (px as i32 + dx).rem_euclid(w as i32) as u32;
                    let sy = (py as i32 + dy).rem_euclid(h as i32) as u32;
                    peak = peak.max(img.get_pixel(sx, sy).0[0]);
                }
            }
            assert!(
                peak > 128,
                "expected land near lat={} lng={}, uv=({:.3},{:.3}) peak={peak}",
                marker.lat,
                marker.lng,
                u.rem_euclid(1.0),
                v.rem_euclid(1.0),
            );
        }
    }

    #[test]
    fn globe_demo_colors_blend_at_midpoint() {
        let light = globe_demo_colors_for_mode(false);
        let dark = globe_demo_colors_for_mode(true);
        let blended = blend_globe_demo_colors(light, dark, 0.5);

        assert!(blended.background.r > 0.2 && blended.background.r < 0.8);
        assert!(blended.title_top.r > 0.4 && blended.title_top.r < 0.6);
    }
}
