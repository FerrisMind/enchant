//! Builder-first macOS-style dock (Magic UI `Dock` / `DockIcon`).
//!
//! Pointer-driven spring magnification lives in host-owned [`DockState`]. Time
//! advances via [`DockState::tick`]; layout size (not CSS scale) drives icons.

use crate::primitives::{pointer_distance, pointer_strength};
use iced::time::Instant;
use iced::widget::{Space, container, mouse_area, row, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};
use iced_shadcn::Theme as ShadTheme;

/// Magic UI bar height (`h-14`).
const DOCK_BAR_HEIGHT: f32 = 56.0;
const DOCK_SEPARATOR_WIDTH: f32 = 1.0;
const MAX_SUBSTEP_SECS: f32 = 0.004;

/// Pointer interaction messages for the dock bar.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DockAction {
    /// Pointer X relative to the dock bar's left edge (stable while the bar re-centers).
    Move(f32),
    Exit,
}

/// Vertical alignment of icons inside the fixed-height bar.
///
/// Matches the Magic UI Direction demo visuals: idle placement is identical for all
/// variants (icons fill the `h-14` content band). On magnification:
/// - [`Self::Top`] — pin bottom edge, grow **up**
/// - [`Self::Middle`] — grow both ways
/// - [`Self::Bottom`] — pin top edge, grow **down**
///
/// Note: Magic UI's Svelte source maps `direction="top"` → CSS `items-start` (grow down).
/// This port swaps Top/Bottom to match the documented Direction demo expectation
/// (Top → icons rise, Bottom → icons drop).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum DockDirection {
    Top,
    #[default]
    Middle,
    Bottom,
}

/// Spring-smoothed magnification.
///
/// Svelte `dock-icon.svelte`: `useSpring(sizeTransform, { mass: 0.1, stiffness: 150, damping: 12 })`.
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
        // Svelte dock-icon.svelte spring: mass 0.1 / stiffness 150 / damping 12.
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

/// Host-owned spring + pointer state for a dock instance.
#[derive(Clone, Debug, Default)]
pub struct DockState {
    /// Pointer X relative to dock left edge. `None` when pointer left the dock.
    pointer_x: Option<f32>,
    springs: Vec<DockSpring>,
    last_tick: Option<Instant>,
    /// Reused across substeps — avoid allocating centers/targets every step.
    scratch_sizes: Vec<f32>,
    scratch_centers: Vec<f32>,
    scratch_targets: Vec<f32>,
}

impl DockState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare(&mut self, icon_count: usize, default_size: f32) {
        if self.springs.len() != icon_count {
            self.springs = (0..icon_count)
                .map(|_| DockSpring::new(default_size))
                .collect();
        }

        self.scratch_sizes.resize(icon_count, 0.0);
        self.scratch_centers.resize(icon_count, 0.0);
        self.scratch_targets.resize(icon_count, 0.0);
    }

    pub fn apply_action(&mut self, action: &DockAction, icon_count: usize, default_size: f32) {
        self.prepare(icon_count, default_size);

        match action {
            DockAction::Move(x) => self.pointer_x = Some(*x),
            DockAction::Exit => self.pointer_x = None,
        }
    }

    /// Advance springs — call every frame from `Message::Tick` with a fresh `Instant`.
    ///
    /// Recomputes icon centers from current spring sizes every substep (DOM-live parity).
    pub fn tick(&mut self, now: Instant, dock: &Dock) {
        if self.springs.is_empty() {
            return;
        }

        let dt = self
            .last_tick
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_tick = Some(now);

        let mut remaining = dt;
        while remaining > f32::EPSILON {
            let sub_dt = remaining.min(MAX_SUBSTEP_SECS);

            for (slot, spring) in self.scratch_sizes.iter_mut().zip(self.springs.iter()) {
                *slot = spring.value;
            }
            fill_icon_centers(&self.scratch_sizes, dock, &mut self.scratch_centers);
            fill_icon_targets(
                self.pointer_x,
                &self.scratch_centers,
                dock,
                &mut self.scratch_targets,
            );

            for (spring, &target) in self.springs.iter_mut().zip(self.scratch_targets.iter()) {
                spring.step(target, sub_dt);
            }
            remaining -= sub_dt;
        }
    }

    /// Whether springs still need animation frames (reuses scratch buffers like [`Self::tick`]).
    pub fn needs_redraw(&mut self, dock: &Dock) -> bool {
        if self.pointer_x.is_some() {
            return true;
        }

        if self.springs.is_empty() {
            return false;
        }

        let n = self.springs.len();
        self.scratch_sizes.resize(n, 0.0);
        self.scratch_centers.resize(n, 0.0);
        self.scratch_targets.resize(n, 0.0);

        for (slot, spring) in self.scratch_sizes.iter_mut().zip(self.springs.iter()) {
            *slot = spring.value;
        }
        fill_icon_centers(&self.scratch_sizes, dock, &mut self.scratch_centers);
        fill_icon_targets(
            self.pointer_x,
            &self.scratch_centers,
            dock,
            &mut self.scratch_targets,
        );

        self.springs
            .iter()
            .zip(self.scratch_targets.iter())
            .any(|(spring, &target)| (spring.value - target).abs() > 0.2)
    }

    pub fn icon_size_at(&self, index: usize, dock: &Dock) -> f32 {
        self.springs
            .get(index)
            .map(|spring| spring.value)
            .unwrap_or(dock.icon_size)
    }

    /// Largest live spring size (or base [`Dock::icon_size`] when springs are empty).
    pub fn max_icon_size(&self, dock: &Dock) -> f32 {
        self.springs
            .iter()
            .map(|spring| spring.value)
            .fold(dock.icon_size, f32::max)
    }
}

/// Builder for a Magic UI dock bar.
///
/// # Examples
///
/// ```rust,no_run
/// use enchant_iced::{Dock, DockState, dock_icon};
/// use iced::Element;
/// use iced_shadcn::Theme as ShadTheme;
///
/// fn bar<'a, Message: Clone + 'static>(
///     state: &'a DockState,
///     theme: &ShadTheme,
///     on_action: impl Fn(enchant_iced::DockAction) -> Message + Clone + 'a,
/// ) -> Element<'a, Message> {
///     Dock::new().into_element(
///         vec![dock_icon(iced::widget::text("Home")).into()],
///         state,
///         on_action,
///         theme,
///     )
/// }
/// ```
#[must_use = "builders do nothing unless turned into an iced Element"]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dock {
    icon_size: f32,
    icon_magnification: f32,
    disable_magnification: bool,
    icon_distance: f32,
    direction: DockDirection,
    gap: f32,
    padding: f32,
    background: Option<Color>,
}

impl Default for Dock {
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

impl Dock {
    /// Creates a dock with Magic UI defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Base icon edge length in logical pixels (default `40`, minimum `1`).
    ///
    /// Also raises [`Self::icon_magnification`] when it would fall below the new size.
    pub fn icon_size(mut self, icon_size: f32) -> Self {
        self.icon_size = icon_size.max(1.0);
        self.icon_magnification = self.icon_magnification.max(self.icon_size);
        self
    }

    /// Peak magnified icon size (default `60`, at least [`Self::icon_size`], minimum `1`).
    pub fn icon_magnification(mut self, icon_magnification: f32) -> Self {
        self.icon_magnification = icon_magnification.max(1.0).max(self.icon_size);
        self
    }

    /// When `true`, icons stay at the base icon size (no spring zoom).
    pub fn disable_magnification(mut self, disable_magnification: bool) -> Self {
        self.disable_magnification = disable_magnification;
        self
    }

    /// Distance falloff radius for magnification (default `140`, minimum `1`).
    pub fn icon_distance(mut self, icon_distance: f32) -> Self {
        self.icon_distance = icon_distance.max(1.0);
        self
    }

    /// Idle is identical for all directions; on magnification Top grows up and
    /// Bottom grows down (Direction demo; opposite of raw CSS `items-start`/`items-end`).
    pub fn direction(mut self, direction: DockDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Gap between icons (default `8`, minimum `0`).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    /// Inner padding of the bar (default `8`, minimum `0`).
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding.max(0.0);
        self
    }

    /// Override frosted surface color.
    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    /// Clears a previously set surface override.
    pub fn clear_background(mut self) -> Self {
        self.background = None;
        self
    }

    /// Optional surface override.
    pub fn background_color(&self) -> Option<Color> {
        self.background
    }

    /// Read-only config snapshot.
    ///
    /// Fluent setters reclaim C-BUILDER short names (`icon_size(…)`, `gap(…)`, …).
    /// Rust forbids a second inherent method with the same name for C-GETTER access, so
    /// hosts that need to read values back (e.g. `DockState::prepare`) use this snapshot.
    pub fn values(&self) -> DockValues {
        DockValues {
            icon_size: self.icon_size,
            icon_magnification: self.icon_magnification,
            disable_magnification: self.disable_magnification,
            icon_distance: self.icon_distance,
            direction: self.direction,
            gap: self.gap,
            padding: self.padding,
            background: self.background,
        }
    }

    /// Builds the dock element for the given entries and host state.
    pub fn into_element<'a, Message: Clone + 'static>(
        self,
        items: Vec<DockEntry<'a, Message>>,
        state: &DockState,
        on_action: impl Fn(DockAction) -> Message + Clone + 'a,
        theme: &ShadTheme,
    ) -> Element<'a, Message> {
        let surface_background = self
            .background
            .or_else(|| theme.registry.color("magic.dock.surface"))
            .unwrap_or_else(|| dock_surface_color(theme));
        let border_color = dock_border_color(theme);
        let radius = theme.radius.lg;
        // Idle icons fill the padded content band (`h-14 − 2·p-2`), so Top/Middle/Bottom
        // look identical. On hover: Top grows up, Bottom grows down (Direction demo).
        // Never align against the full reserved box — that inverted idle vs hover.
        let reserved_height = dock_reserved_height(&self);
        let content_width = dock_content_width(&items, state, &self);
        let pointer_mapper = dock_pointer_mapper(&items, state, &self);
        // Chrome is centered in `reserved_height` ⇒ content-band top inset equals
        // `(reserved − h-14)/2 + p-2` (and matches the bottom inset).
        let content_inset = (reserved_height - DOCK_BAR_HEIGHT) / 2.0 + self.padding;

        // Horizontal `p-2` as side Spaces only — never `row.padding` vertically (would
        // squash Fixed(spring) slots). Gaps are inserted between entries only; do **not**
        // use `row.spacing` around the padding Spaces (that inflated width past `w-max`
        // and horizontally crushed square slots into ovals).
        let mut icons = row![].align_y(Alignment::Start);
        let mut icon_index = 0;

        for (entry_index, item) in items.into_iter().enumerate() {
            if entry_index > 0 {
                icons = icons.push(Space::new().width(Length::Fixed(self.gap)));
            }

            match item {
                DockEntry::Icon(item) => {
                    let size = state.icon_size_at(icon_index, &self);
                    icon_index += 1;
                    let slot = dock_icon_slot(item, size, &self, theme);
                    icons = icons.push(dock_direction_place(
                        slot,
                        size,
                        reserved_height,
                        content_inset,
                        self.direction,
                    ));
                }
                DockEntry::Separator => {
                    // Separator stays in the h-14 content band (middle), not direction-shifted.
                    icons = icons.push(dock_direction_place(
                        dock_separator_view(theme),
                        (DOCK_BAR_HEIGHT - 2.0 * self.padding).max(1.0),
                        reserved_height,
                        content_inset,
                        DockDirection::Middle,
                    ));
                }
            }
        }

        let content = row![
            Space::new().width(Length::Fixed(self.padding)),
            icons,
            Space::new().width(Length::Fixed(self.padding)),
        ]
        .align_y(Alignment::Start);

        let chrome = container(Space::new())
            .width(Length::Fixed(content_width))
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

        let chrome_layer = container(chrome)
            .width(Length::Fixed(content_width))
            .height(Length::Fixed(reserved_height))
            .center_y(Length::Fixed(reserved_height));

        let icons_layer = container(content)
            .width(Length::Fixed(content_width))
            .height(Length::Fixed(reserved_height))
            .clip(false);

        let body = container(stack![chrome_layer, icons_layer])
            .width(Length::Fixed(content_width))
            .height(Length::Fixed(reserved_height))
            .clip(false);

        mouse_area(body)
            .on_move({
                let on_action = on_action.clone();
                move |point| on_action(DockAction::Move(pointer_mapper.map(point.x)))
            })
            .on_exit(on_action(DockAction::Exit))
            .into()
    }

    /// Alias for [`Self::into_element`].
    pub fn view<'a, Message: Clone + 'static>(
        self,
        items: Vec<DockEntry<'a, Message>>,
        state: &DockState,
        on_action: impl Fn(DockAction) -> Message + Clone + 'a,
        theme: &ShadTheme,
    ) -> Element<'a, Message> {
        self.into_element(items, state, on_action, theme)
    }
}

/// Read-only copy of [`Dock`] configuration (see [`Dock::values`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockValues {
    pub icon_size: f32,
    pub icon_magnification: f32,
    pub disable_magnification: bool,
    pub icon_distance: f32,
    pub direction: DockDirection,
    pub gap: f32,
    pub padding: f32,
    /// Optional surface override (`None` = theme / frosted default).
    pub background: Option<Color>,
}

/// A pressable / decorative icon slot inside the dock.
pub struct DockIcon<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Message>,
    background: Option<Color>,
    tooltip: Option<String>,
}

impl<'a, Message> DockIcon<'a, Message> {
    pub fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            background: None,
            tooltip: None,
        }
    }

    #[must_use]
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    #[must_use]
    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Convenience constructor for [`DockIcon::new`].
pub fn dock_icon<'a, Message>(content: impl Into<Element<'a, Message>>) -> DockIcon<'a, Message> {
    DockIcon::new(content)
}

/// Dock children: interactive icons or vertical separators between groups.
#[non_exhaustive]
pub enum DockEntry<'a, Message> {
    Icon(DockIcon<'a, Message>),
    Separator,
}

impl<'a, Message> From<DockIcon<'a, Message>> for DockEntry<'a, Message> {
    fn from(icon: DockIcon<'a, Message>) -> Self {
        Self::Icon(icon)
    }
}

/// Vertical separator entry between icon groups.
pub fn dock_separator<'a, Message>() -> DockEntry<'a, Message> {
    DockEntry::Separator
}

/// Counts [`DockEntry::Icon`] slots (separators ignored).
pub fn dock_entry_icon_count<Message>(items: &[DockEntry<'_, Message>]) -> usize {
    items
        .iter()
        .filter(|entry| matches!(entry, DockEntry::Icon(_)))
        .count()
}

/// Place a spring slot (or separator) inside the reserved box against the h-14
/// content band so idle Top/Middle/Bottom share the same Y.
///
/// On magnification: Top grows **up**, Bottom grows **down**, Middle both ways.
fn dock_direction_place<'a, Message: 'a>(
    slot: Element<'a, Message>,
    slot_height: f32,
    reserved_height: f32,
    content_inset: f32,
    direction: DockDirection,
) -> Element<'a, Message> {
    let inset = content_inset.max(0.0);
    // `slot_height` documents the idle/hover relationship for readers/tests; layout
    // uses direction + inset against the reserved box (springs keep their Fixed size).
    let _ = slot_height;

    match direction {
        // Top → rise: pin bottom of content band (CSS items-end).
        DockDirection::Top => container(slot)
            .height(Length::Fixed(reserved_height))
            .align_y(Alignment::End)
            .padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: inset,
                left: 0.0,
            })
            .clip(false)
            .into(),
        // Bottom → drop: pin top of content band (CSS items-start).
        DockDirection::Bottom => container(slot)
            .height(Length::Fixed(reserved_height))
            .align_y(Alignment::Start)
            .padding(Padding {
                top: inset,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            })
            .clip(false)
            .into(),
        DockDirection::Middle => container(slot)
            .height(Length::Fixed(reserved_height))
            .center_y(Length::Fixed(reserved_height))
            .clip(false)
            .into(),
    }
}

/// Magic UI preview `Button` `size-12` (3rem) — stays fixed while `DockIcon` springs.
///
/// Svelte preview: `class={cn("size-12 rounded-full")}` inside `DockIcon`.
const DOCK_ICON_BUTTON_SIZE: f32 = 48.0;

fn dock_icon_slot<'a, Message: Clone + 'static>(
    item: DockIcon<'a, Message>,
    spring_size: f32,
    dock: &Dock,
    theme: &ShadTheme,
) -> Element<'a, Message> {
    // Svelte dock-icon.svelte: `width: scaleSize, height: scaleSize` + `aspect-square`
    // — slot is always a square equal to the spring value.
    let (slot_w, slot_h) = dock_icon_slot_size(spring_size);

    let glyph = container(item.content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let body: Element<'a, Message> = match item.on_press {
        Some(message) => {
            // Svelte preview: `<Button size="icon" class="size-12 rounded-full">` —
            // fixed 48×48 circle; does **not** grow with magnification (only the
            // DockIcon wrapper springs and pushes neighbors).
            let foreground = theme.palette.foreground;
            let hover_background = if dock.disable_magnification {
                with_alpha(theme.palette.muted_foreground, 0.18)
            } else {
                with_alpha(theme.palette.muted_foreground, 0.12)
            };
            let pressed_background = blend_colors(hover_background, theme.palette.foreground, 0.08);
            let radius = DOCK_ICON_BUTTON_SIZE / 2.0;

            iced::widget::button(glyph)
                .width(Length::Fixed(DOCK_ICON_BUTTON_SIZE))
                .height(Length::Fixed(DOCK_ICON_BUTTON_SIZE))
                .padding(0)
                .on_press(message)
                .style(move |_theme, status| {
                    dock_icon_button_style(
                        status,
                        radius,
                        foreground,
                        hover_background,
                        pressed_background,
                    )
                })
                .into()
        }
        None => {
            // Svelte dock-icon.svelte: outer `motion.div` gets `width/height: scaleSize`,
            // `padding: max(6, iconSize * 0.2)`, `rounded-full`, and `class` (incl. `bg-*`).
            // Glyph fills the padded content box (`size-full`).
            let pad = icon_slot_padding(dock.icon_size);
            let radius = decorative_icon_surface_radius(slot_w);
            let background = item.background;

            container(glyph)
                .width(Length::Fixed(slot_w))
                .height(Length::Fixed(slot_h))
                .padding(pad)
                .style(move |_| iced::widget::container::Style {
                    background: background.map(Background::Color),
                    border: Border {
                        radius: radius.into(),
                        ..Default::default()
                    },
                    ..iced::widget::container::Style::default()
                })
                .into()
        }
    };

    // Outer DockIcon slot: square spring size, content centered; clip(false) so a
    // size-12 button can overflow when spring < 48 (CSS overflow:visible).
    let slot = container(body)
        .width(Length::Fixed(slot_w))
        .height(Length::Fixed(slot_h))
        .center_x(Length::Fixed(slot_w))
        .center_y(Length::Fixed(slot_h))
        .clip(false);

    with_dock_tooltip(slot.into(), item.tooltip, theme)
}

fn dock_separator_view<'a, Message: 'a>(theme: &ShadTheme) -> Element<'a, Message> {
    let color = with_alpha(theme.palette.border, 0.8);
    // Preview separator sits in the h-14 content band (~40px), not the reserved overflow height.
    let height = (DOCK_BAR_HEIGHT - 16.0).max(1.0);

    container(Space::new())
        .width(Length::Fixed(DOCK_SEPARATOR_WIDTH))
        .height(Length::Fixed(height))
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(color)),
            ..iced::widget::container::Style::default()
        })
        .into()
}

#[derive(Clone, Debug)]
struct DockPointerMapper {
    breakpoints: Vec<(f32, f32)>,
}

impl DockPointerMapper {
    fn map(&self, actual_x: f32) -> f32 {
        let mut offset = 0.0;
        for (breakpoint, breakpoint_offset) in &self.breakpoints {
            if actual_x < *breakpoint {
                break;
            }
            offset = *breakpoint_offset;
        }
        actual_x - offset
    }
}

fn dock_pointer_mapper<Message>(
    items: &[DockEntry<'_, Message>],
    state: &DockState,
    dock: &Dock,
) -> DockPointerMapper {
    let mut actual_x = dock.padding;
    let mut virtual_x = dock.padding;
    let mut icon_index = 0;
    let mut has_previous_icon = false;
    let mut breakpoints = vec![(0.0, 0.0)];

    for (entry_index, item) in items.iter().enumerate() {
        if entry_index > 0 {
            actual_x += dock.gap;
        }

        match item {
            DockEntry::Icon(_) => {
                if has_previous_icon {
                    virtual_x += dock.gap;
                }

                breakpoints.push((actual_x, actual_x - virtual_x));

                let size = state.icon_size_at(icon_index, dock);
                actual_x += size;
                virtual_x += size;
                icon_index += 1;
                has_previous_icon = true;
            }
            DockEntry::Separator => {
                breakpoints.push((actual_x, actual_x - virtual_x));
                actual_x += DOCK_SEPARATOR_WIDTH;
            }
        }
    }

    DockPointerMapper { breakpoints }
}

fn with_dock_tooltip<'a, Message: 'static>(
    content: Element<'a, Message>,
    tooltip: Option<String>,
    theme: &ShadTheme,
) -> Element<'a, Message> {
    let Some(tooltip) = tooltip else {
        return content;
    };

    let text_color = theme.palette.popover_foreground;
    let background = theme.palette.popover;
    let border_color = theme.palette.border;
    let radius = theme.radius.md;

    iced::widget::tooltip(
        content,
        text(tooltip).size(12),
        iced::widget::tooltip::Position::Top,
    )
    .gap(8)
    .padding(8)
    .style(move |_| iced::widget::container::Style {
        text_color: Some(text_color),
        background: Some(Background::Color(background)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius.into(),
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}

fn dock_icon_button_style(
    status: iced::widget::button::Status,
    radius: f32,
    text_color: Color,
    hover_background: Color,
    pressed_background: Color,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(hover_background)),
        iced::widget::button::Status::Pressed => Some(Background::Color(pressed_background)),
        iced::widget::button::Status::Active | iced::widget::button::Status::Disabled => None,
    };

    iced::widget::button::Style {
        background,
        text_color,
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..iced::widget::button::Style::default()
    }
}

/// Icon center X relative to the dock bar's left edge (writes into `out`).
fn fill_icon_centers(sizes: &[f32], dock: &Dock, out: &mut [f32]) {
    debug_assert_eq!(sizes.len(), out.len());
    let mut x = dock.padding;
    for (index, &size) in sizes.iter().enumerate() {
        out[index] = x + size / 2.0;
        x += size + dock.gap;
    }
}

/// Icon centers as a new `Vec` (tests / one-shot helpers).
#[cfg(test)]
fn icon_centers_relative(sizes: &[f32], dock: &Dock) -> Vec<f32> {
    let mut out = vec![0.0; sizes.len()];
    fill_icon_centers(sizes, dock, &mut out);
    out
}

/// Frosted-glass tint composited over the page background (Magic UI `bg-white/10` / `dark:bg-black/10`).
pub fn dock_surface_color(theme: &ShadTheme) -> Color {
    let background = theme.palette.background;
    let tint = blend_colors(
        Color::from_rgb(1.0, 1.0, 1.0),
        Color::from_rgb(0.0, 0.0, 0.0),
        theme_darkness(background),
    );
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

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        a: (color.a * alpha).clamp(0.0, 1.0),
        ..color
    }
}

fn icon_slot_padding(base_size: f32) -> f32 {
    (base_size * 0.2).max(6.0)
}

/// `rounded-full` radius for decorative DockIcon chrome (Svelte outer `motion.div`).
///
/// Background paints this spring diameter — not the inner padded content box.
fn decorative_icon_surface_radius(spring_size: f32) -> f32 {
    dock_icon_slot_size(spring_size).0 / 2.0
}

/// Stable outer layout height for the iced stand-in of CSS `overflow:visible` on `h-14`.
///
/// Must fit a peak spring pinned to the content band (Top grows down, Bottom grows up):
/// with the chrome centered, that needs `reserved >= 2 * (icon_magnification - padding)`
/// as well as `max(h-14, icon_magnification)`.
fn dock_reserved_height(dock: &Dock) -> f32 {
    let peak = dock.icon_magnification;
    let for_directional_overflow = (2.0 * (peak - dock.padding)).max(0.0);
    DOCK_BAR_HEIGHT.max(peak).max(for_directional_overflow)
}

/// Intrinsic dock width (`w-max`): padding + gaps + live icon/separator sizes.
fn dock_content_width<Message>(
    items: &[DockEntry<'_, Message>],
    state: &DockState,
    dock: &Dock,
) -> f32 {
    if items.is_empty() {
        return (dock.padding * 2.0).max(1.0);
    }

    let mut width = dock.padding * 2.0;
    let mut icon_index = 0;
    for (entry_index, item) in items.iter().enumerate() {
        if entry_index > 0 {
            width += dock.gap;
        }
        match item {
            DockEntry::Icon(_) => {
                width += state.icon_size_at(icon_index, dock);
                icon_index += 1;
            }
            DockEntry::Separator => {
                width += DOCK_SEPARATOR_WIDTH;
            }
        }
    }
    width.max(1.0)
}

/// Target DockIcon edge length for a pointer sample — always used as both width and height.
fn dock_icon_slot_size(spring_or_target: f32) -> (f32, f32) {
    let edge = spring_or_target.max(1.0);
    (edge, edge)
}

fn fill_icon_targets(pointer_x: Option<f32>, centers: &[f32], dock: &Dock, out: &mut [f32]) {
    debug_assert_eq!(centers.len(), out.len());
    for (index, slot) in out.iter_mut().enumerate() {
        let center = centers
            .get(index)
            .copied()
            .unwrap_or_else(|| fallback_center_x(index, dock));
        *slot = dock_icon_target_size(pointer_x, center, dock);
    }
}

fn fallback_center_x(index: usize, dock: &Dock) -> f32 {
    let slot = dock.icon_size;
    let mut x = dock.padding;
    for _ in 0..index {
        x += slot + dock.gap;
    }
    x + slot / 2.0
}

fn dock_icon_target_size(pointer_x: Option<f32>, center_x: f32, dock: &Dock) -> f32 {
    if dock.disable_magnification {
        return dock.icon_size;
    }

    let Some(pointer_x) = pointer_x else {
        return dock.icon_size;
    };

    let strength = pointer_strength(
        pointer_distance(pointer_x, center_x),
        dock.icon_distance.max(1.0),
    );

    dock.icon_size + (dock.icon_magnification - dock.icon_size) * strength
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::time::Duration;

    #[test]
    fn dock_defaults_match_magic_ui() {
        let d = Dock::new();
        let v = d.values();
        assert!((v.icon_size - 40.0).abs() < f32::EPSILON);
        assert!((v.icon_magnification - 60.0).abs() < f32::EPSILON);
        assert!(!v.disable_magnification);
        assert!((v.icon_distance - 140.0).abs() < f32::EPSILON);
        assert_eq!(v.direction, DockDirection::Middle);
        assert!((v.gap - 8.0).abs() < f32::EPSILON);
        assert!((v.padding - 8.0).abs() < f32::EPSILON);
        assert!(v.background.is_none());
    }

    #[test]
    fn dock_size_peaks_when_pointer_matches_center() {
        let d = Dock::new();
        let center = 48.0;
        assert!(
            (dock_icon_target_size(Some(center), center, &d) - d.icon_magnification).abs() < 0.01
        );
        assert!((dock_icon_target_size(None, center, &d) - d.icon_size).abs() < 0.01);
    }

    #[test]
    fn disable_magnification_keeps_base_size() {
        let d = Dock::new().disable_magnification(true);
        assert!((dock_icon_target_size(Some(48.0), 48.0, &d) - 40.0).abs() < 0.01);
    }

    #[test]
    fn dock_spring_converges_under_pointer() {
        let mut state = DockState::new();
        let dock = Dock::new();
        state.prepare(3, dock.icon_size);
        let rest = [40.0_f32; 3];
        let centers = icon_centers_relative(&rest, &dock);
        state.apply_action(&DockAction::Move(centers[1]), 3, dock.icon_size);
        let start = Instant::now();
        for frame in 1..=60 {
            state.tick(start + Duration::from_millis(frame * 16), &dock);
        }
        assert!(state.icon_size_at(1, &dock) > dock.icon_size + 5.0);
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
    fn dock_surface_does_not_jump_at_theme_midpoint() {
        let mut before = crate::theme::MagicTheme::light();
        before.palette.background = Color::from_rgb(0.51, 0.51, 0.51);
        let mut after = crate::theme::MagicTheme::light();
        after.palette.background = Color::from_rgb(0.49, 0.49, 0.49);

        let before_surface = dock_surface_color(&before);
        let after_surface = dock_surface_color(&after);

        assert!((before_surface.r - after_surface.r).abs() < 0.04);
        assert!((before_surface.g - after_surface.g).abs() < 0.04);
        assert!((before_surface.b - after_surface.b).abs() < 0.04);
    }

    #[test]
    fn dock_entry_icon_count_ignores_separators() {
        let entries: Vec<DockEntry<'static, ()>> = vec![
            dock_icon(text("Home")).into(),
            dock_separator(),
            dock_icon(text("Mail")).into(),
        ];

        assert_eq!(dock_entry_icon_count(&entries), 2);
    }

    #[test]
    fn setters_validate_ranges() {
        let d = Dock::new().icon_size(0.0).icon_distance(-1.0).gap(-2.0);
        let v = d.values();
        assert!(v.icon_size >= 1.0);
        assert!(v.icon_distance >= 1.0);
        assert!(v.gap >= 0.0);
        assert!(v.icon_magnification >= v.icon_size);

        let d = Dock::new().icon_size(80.0).icon_magnification(50.0);
        let v = d.values();
        assert!((v.icon_size - 80.0).abs() < f32::EPSILON);
        assert!(v.icon_magnification >= v.icon_size);
    }

    #[test]
    fn reserved_height_is_stable_across_pointer_motion() {
        let dock = Dock::new();
        let expected = {
            let peak = dock.icon_magnification;
            DOCK_BAR_HEIGHT
                .max(peak)
                .max((2.0 * (peak - dock.padding)).max(0.0))
        };
        assert!((dock_reserved_height(&dock) - expected).abs() < f32::EPSILON);

        let mut state = DockState::new();
        state.prepare(3, dock.icon_size);
        assert!((dock_reserved_height(&dock) - expected).abs() < f32::EPSILON);

        let centers = icon_centers_relative(&[40.0, 40.0, 40.0], &dock);
        state.apply_action(&DockAction::Move(centers[1]), 3, dock.icon_size);
        let start = Instant::now();
        for frame in 1..=90 {
            state.tick(start + Duration::from_millis(frame * 16), &dock);
        }
        assert!(state.max_icon_size(&dock) > dock.icon_size + 10.0);
        // Layout reservation must not track live springs — no page shift / icon jerk.
        assert!((dock_reserved_height(&dock) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn direction_idle_offsets_match_across_top_middle_bottom() {
        // Idle spring == content band ⇒ same content-band placement for every direction.
        let dock = Dock::new();
        let reserved = dock_reserved_height(&dock);
        let inset = (reserved - DOCK_BAR_HEIGHT) / 2.0 + dock.padding;
        let band = (DOCK_BAR_HEIGHT - 2.0 * dock.padding).max(1.0);
        let spring = dock.icon_size;
        assert!(
            (spring - band).abs() < f32::EPSILON,
            "defaults: icon_size fills h-14 content band"
        );

        let top_y = reserved - inset - spring; // Top pins spring bottom to content bottom
        let middle_y = (reserved - spring) / 2.0;
        let bottom_y = inset; // Bottom pins spring top to content top
        assert!(
            (top_y - middle_y).abs() < 0.01,
            "idle Top Y ({top_y}) must match Middle ({middle_y})"
        );
        assert!(
            (bottom_y - middle_y).abs() < 0.01,
            "idle Bottom Y ({bottom_y}) must match Middle ({middle_y})"
        );
    }

    #[test]
    fn direction_hover_peak_moves_up_for_top_and_down_for_bottom() {
        let dock = Dock::new();
        let reserved = dock_reserved_height(&dock);
        let inset = (reserved - DOCK_BAR_HEIGHT) / 2.0 + dock.padding;
        let peak = dock.icon_magnification;
        let idle = dock.icon_size;
        let idle_mid = inset + idle / 2.0;
        // Top pins bottom → center moves up; Bottom pins top → center moves down.
        let top_mid = reserved - inset - peak / 2.0;
        let bottom_mid = inset + peak / 2.0;
        assert!(top_mid < idle_mid - 1.0, "Top peak center must move up");
        assert!(
            bottom_mid > idle_mid + 1.0,
            "Bottom peak center must move down"
        );
    }

    #[test]
    fn content_width_matches_icons_and_gaps() {
        let dock = Dock::new();
        let mut state = DockState::new();
        state.prepare(2, dock.icon_size);
        let entries: Vec<DockEntry<'static, ()>> = vec![
            dock_icon(text("A")).into(),
            dock_separator(),
            dock_icon(text("B")).into(),
        ];
        // padding 8*2 + 40 + gap 8 + sep 1 + gap 8 + 40 = 113
        let expected = 8.0 * 2.0 + 40.0 + 8.0 + 1.0 + 8.0 + 40.0;
        assert!((dock_content_width(&entries, &state, &dock) - expected).abs() < 0.01);
    }

    #[test]
    fn dock_icon_slot_targets_are_always_square() {
        let dock = Dock::new();
        for &edge in &[1.0_f32, 40.0, 48.0, 60.0, 90.0] {
            let (w, h) = dock_icon_slot_size(edge);
            assert!((w - h).abs() < f32::EPSILON, "slot {w}×{h} must be square");
            assert!((w - edge).abs() < f32::EPSILON);
        }

        let peak = dock_icon_target_size(Some(48.0), 48.0, &dock);
        let (w, h) = dock_icon_slot_size(peak);
        assert!((w - h).abs() < f32::EPSILON);
        assert!((w - dock.icon_magnification).abs() < 0.01);
    }

    #[test]
    fn preview_pressable_button_size_is_size_12() {
        // Svelte preview: `size-12 rounded-full` → 48×48; never tracks the spring.
        assert!((DOCK_ICON_BUTTON_SIZE - 48.0).abs() < f32::EPSILON);
        let radius = DOCK_ICON_BUTTON_SIZE / 2.0;
        assert!((radius - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn decorative_background_uses_outer_spring_radius_not_inner() {
        // Svelte dock-icon.svelte: `class` / `bg-*` + `rounded-full` on the outer
        // spring square (`width/height: scaleSize`), not the padded content box.
        let spring = 60.0;
        let icon_size = 40.0;
        let pad = icon_slot_padding(icon_size);
        let radius = decorative_icon_surface_radius(spring);
        assert!(
            (radius - spring / 2.0).abs() < f32::EPSILON,
            "expected outer spring radius {}, got {radius}",
            spring / 2.0
        );
        let inner_radius = (spring - pad * 2.0).max(1.0) / 2.0;
        assert!(
            (radius - inner_radius).abs() > 1.0,
            "radius must not collapse to inner padded box ({inner_radius})"
        );
        assert!((pad - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn content_width_grows_with_live_springs_not_fill() {
        let dock = Dock::new();
        let mut state = DockState::new();
        state.prepare(2, dock.icon_size);
        let entries: Vec<DockEntry<'static, ()>> =
            vec![dock_icon(text("A")).into(), dock_icon(text("B")).into()];
        let idle = dock_content_width(&entries, &state, &dock);

        let centers = icon_centers_relative(&[40.0, 40.0], &dock);
        state.apply_action(&DockAction::Move(centers[0]), 2, dock.icon_size);
        let start = Instant::now();
        for frame in 1..=90 {
            state.tick(start + Duration::from_millis(frame * 16), &dock);
        }
        let bloomed = dock_content_width(&entries, &state, &dock);
        assert!(bloomed > idle + 5.0);
        // Still an intrinsic content width — never a Fill/window fraction.
        assert!(bloomed < 400.0);
    }
}
