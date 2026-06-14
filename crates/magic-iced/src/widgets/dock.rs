use crate::primitives::{pointer_distance, pointer_strength};
use iced::time::Instant;
use iced::widget::{container, mouse_area, row};
use iced::{Background, Color, Element, Length};
use iced_shadcn::Theme as ShadTheme;

/// Magic UI default bar height (`h-[58px]`).
const DOCK_BAR_HEIGHT: f32 = 58.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DockAction {
    /// Pointer X relative to the dock bar's left edge (stable while the bar re-centers).
    Move(f32),
    Exit,
}

/// Spring-smoothed magnification — Magic UI `useSpring({ mass: 0.1, stiffness: 150, damping: 12 })`.
#[derive(Clone, Copy, Debug)]
struct DockSpring {
    value: f32,
    velocity: f32,
}

impl DockSpring {
    fn new(initial: f32) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
        }
    }

    fn step(&mut self, target: f32, dt: f32) {
        const MASS: f32 = 0.1;
        const STIFFNESS: f32 = 150.0;
        const DAMPING: f32 = 12.0;

        let dt = dt.clamp(1.0 / 240.0, 0.05);
        let displacement = self.value - target;
        let acceleration = (-STIFFNESS * displacement - DAMPING * self.velocity) / MASS;
        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;
    }
}

#[derive(Clone, Debug)]
pub struct DockState {
    /// Pointer X relative to dock left edge. `None` when pointer left the dock.
    pointer_x: Option<f32>,
    springs: Vec<DockSpring>,
    last_tick: Option<Instant>,
}

impl Default for DockState {
    fn default() -> Self {
        Self {
            pointer_x: None,
            springs: Vec::new(),
            last_tick: None,
        }
    }
}

impl DockState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare(&mut self, icon_count: usize, default_size: f32) {
        if self.springs.len() == icon_count {
            return;
        }

        self.springs = (0..icon_count)
            .map(|_| DockSpring::new(default_size))
            .collect();
    }

    pub fn apply_action(&mut self, action: &DockAction, icon_count: usize, default_size: f32) {
        self.prepare(icon_count, default_size);

        match action {
            DockAction::Move(x) => self.pointer_x = Some(*x),
            DockAction::Exit => self.pointer_x = None,
        }
    }

    /// Advance springs — call every frame from `Message::Tick` with a fresh `Instant`.
    pub fn tick(&mut self, now: Instant, props: &DockProps) {
        if self.springs.is_empty() {
            return;
        }

        let dt = self
            .last_tick
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_tick = Some(now);

        let sizes: Vec<f32> = self.springs.iter().map(|spring| spring.value).collect();
        let centers = icon_centers_relative(&sizes, props);
        let targets = icon_targets(self.pointer_x, &centers, self.springs.len(), props);

        // Explicit spring integration needs substeps at ~60 Hz (dt≈16 ms is unstable).
        const MAX_SUBSTEP_SECS: f32 = 0.004;
        let mut remaining = dt;
        while remaining > f32::EPSILON {
            let sub_dt = remaining.min(MAX_SUBSTEP_SECS);
            for (spring, target) in self.springs.iter_mut().zip(targets.iter().copied()) {
                spring.step(target, sub_dt);
            }
            remaining -= sub_dt;
        }
    }

    pub fn needs_redraw(&self, props: &DockProps) -> bool {
        if self.pointer_x.is_some() {
            return true;
        }

        let sizes: Vec<f32> = self.springs.iter().map(|spring| spring.value).collect();
        let centers = icon_centers_relative(&sizes, props);
        let targets = icon_targets(self.pointer_x, &centers, self.springs.len(), props);
        self.springs
            .iter()
            .zip(targets)
            .any(|(spring, target)| (spring.value - target).abs() > 0.2)
    }

    fn icon_size_at(&self, index: usize, props: &DockProps) -> f32 {
        self.springs
            .get(index)
            .map(|spring| spring.value)
            .unwrap_or(props.icon_size)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DockDirection {
    Top,
    #[default]
    Middle,
    Bottom,
}

pub struct DockIcon<'a, Message> {
    pub content: Element<'a, Message>,
    pub on_press: Option<Message>,
    pub background: Option<Color>,
}

impl<'a, Message> DockIcon<'a, Message> {
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }
}

pub fn dock_icon<'a, Message>(content: impl Into<Element<'a, Message>>) -> DockIcon<'a, Message> {
    DockIcon {
        content: content.into(),
        on_press: None,
        background: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockProps {
    pub icon_size: f32,
    pub icon_magnification: f32,
    pub disable_magnification: bool,
    pub icon_distance: f32,
    pub direction: DockDirection,
    pub gap: f32,
    pub padding: f32,
    pub background: Option<Color>,
}

impl Default for DockProps {
    fn default() -> Self {
        Self {
            icon_size: 40.0,
            icon_magnification: 60.0,
            disable_magnification: false,
            icon_distance: 140.0,
            direction: DockDirection::Middle,
            gap: 8.0,
            padding: 8.0,
            background: None,
        }
    }
}

impl DockProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn dock<'a, Message: Clone + 'static>(
    items: Vec<DockIcon<'a, Message>>,
    state: &DockState,
    props: DockProps,
    on_action: impl Fn(DockAction) -> Message + Clone + 'a,
    theme: &ShadTheme,
) -> Element<'a, Message> {
    let surface_background = props
        .background
        .or_else(|| theme.registry.color("magic.dock.surface"))
        .unwrap_or_else(|| dock_surface_color(theme));
    let border_color = dock_border_color(theme);
    let radius = theme.radius.lg;

    let mut content = row![].spacing(props.gap).padding(props.padding);

    for (index, item) in items.into_iter().enumerate() {
        // Magic UI: `style={{ width: scaleSize, height: scaleSize }}` — layout size, not CSS scale.
        let size = state.icon_size_at(index, &props);
        let icon_padding = icon_slot_padding(props.icon_size);

        let mut icon = container(item.content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(icon_padding);

        if let Some(background) = item.background {
            icon = icon.style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(background)),
                border: iced::Border {
                    radius: (size / 2.0).into(),
                    ..Default::default()
                },
                ..iced::widget::container::Style::default()
            });
        }

        let slot: Element<'a, Message> = container(icon)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .center_x(Length::Fill)
            .center_y(match props.direction {
                DockDirection::Top => Length::Shrink,
                DockDirection::Middle => Length::Fill,
                DockDirection::Bottom => Length::Shrink,
            })
            .into();

        content = content.push(slot);
    }

    let body = container(content)
        .height(Length::Fixed(DOCK_BAR_HEIGHT))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(surface_background)),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: radius.into(),
            },
            ..iced::widget::container::Style::default()
        });

    // Magic UI: `onMouseMove` / `onMouseLeave` on the dock container.
    // `mouse_area` `on_move` gives position relative to the bar — stable when `w-max` re-centers.
    mouse_area(body)
        .on_move({
            let on_action = on_action.clone();
            move |point| on_action(DockAction::Move(point.x))
        })
        .on_exit(on_action(DockAction::Exit))
        .into()
}

/// Icon center X relative to the dock bar's left edge.
fn icon_centers_relative(sizes: &[f32], props: &DockProps) -> Vec<f32> {
    let mut x = props.padding;
    sizes
        .iter()
        .map(|&size| {
            let center = x + size / 2.0;
            x += size + props.gap;
            center
        })
        .collect()
}

/// Frosted-glass tint composited over the page background (Magic UI `bg-white/10` / `dark:bg-black/10`).
pub fn dock_surface_color(theme: &ShadTheme) -> Color {
    let background = theme.palette.background;
    let tint = if theme_darkness(background) >= 0.5 {
        Color::from_rgb(0.0, 0.0, 0.0)
    } else {
        Color::from_rgb(1.0, 1.0, 1.0)
    };
    blend_colors(background, tint, 0.1)
}

/// Border composited on the translucent dock surface.
pub fn dock_border_color(theme: &ShadTheme) -> Color {
    let surface = dock_surface_color(theme);
    let border = Color {
        a: 0.2,
        ..theme.palette.border
    };
    blend_colors(surface, border, border.a)
}

fn theme_darkness(background: Color) -> f32 {
    let luminance = background.r * 0.299 + background.g * 0.587 + background.b * 0.114;
    (1.0 - luminance).clamp(0.0, 1.0)
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

fn icon_slot_padding(base_size: f32) -> f32 {
    (base_size * 0.2).max(6.0)
}

fn icon_targets(
    pointer_x: Option<f32>,
    centers: &[f32],
    icon_count: usize,
    props: &DockProps,
) -> Vec<f32> {
    (0..icon_count)
        .map(|index| {
            let center = centers
                .get(index)
                .copied()
                .unwrap_or_else(|| fallback_center_x(index, props));
            dock_icon_target_size(pointer_x, center, props)
        })
        .collect()
}

fn fallback_center_x(index: usize, props: &DockProps) -> f32 {
    let slot = props.icon_size;
    let mut x = props.padding;
    for _ in 0..index {
        x += slot + props.gap;
    }
    x + slot / 2.0
}

fn dock_icon_target_size(pointer_x: Option<f32>, center_x: f32, props: &DockProps) -> f32 {
    if props.disable_magnification {
        return props.icon_size;
    }

    let Some(pointer_x) = pointer_x else {
        return props.icon_size;
    };

    let strength = pointer_strength(
        pointer_distance(pointer_x, center_x),
        props.icon_distance.max(1.0),
    );

    props.icon_size + (props.icon_magnification - props.icon_size) * strength
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn dock_size_peaks_when_pointer_matches_center() {
        let props = DockProps::new();
        let center = 48.0;

        assert_eq!(
            dock_icon_target_size(Some(center), center, &props),
            props.icon_magnification
        );
        assert_eq!(
            dock_icon_target_size(None, center, &props),
            props.icon_size
        );
    }

    #[test]
    fn dock_surface_follows_blended_background() {
        let mut light = crate::theme::MagicTheme::light();
        light.palette.background = Color::WHITE;
        let mut dark = crate::theme::MagicTheme::dark();
        dark.palette.background = Color::from_rgb(0.05, 0.05, 0.08);

        let light_surface = dock_surface_color(&light);
        let dark_surface = dock_surface_color(&dark);
        assert!(light_surface.r > 0.9);
        assert!(dark_surface.r < dark.palette.background.r + 0.15);
    }

    #[test]
    fn dock_spring_converges_under_pointer() {
        let mut state = DockState::new();
        let props = DockProps::new();
        state.prepare(3, props.icon_size);

        let sizes = vec![props.icon_size; 3];
        let centers = icon_centers_relative(&sizes, &props);
        state.apply_action(&DockAction::Move(centers[1]), 3, props.icon_size);

        let start = Instant::now();
        for frame in 1..=60 {
            state.tick(start + Duration::from_millis(frame * 16), &props);
        }

        assert!(state.springs[1].value > props.icon_size + 5.0);
    }
}
