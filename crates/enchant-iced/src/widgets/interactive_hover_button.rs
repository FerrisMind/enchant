//! Builder-first interactive hover button (Magic UI `InteractiveHoverButton`).
//!
//! svelte layers:
//! 1. Pill chrome (`bg-background`, border, `rounded-full`, `overflow:hidden`)
//! 2. Expanding primary dot (`scale-[100.8]` from `h-2 w-2`)
//! 3. Idle row: dot + label slides `translate-x-12` / fades out
//! 4. Hover layer: label + optional icon, centered (`justify-center`)

use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas};
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
use iced::advanced::text::{
    self as text_layout, Alignment as TextAlign, LineHeight, Paragraph as _, Shaping, Wrapping,
};
use iced::alignment::Vertical;
use iced::border::Radius;
use iced::font::{Font, Style, Weight};
use iced::mouse;
use iced::widget::button;
use iced::widget::canvas::{self, Fill};
use iced::widget::{container, row, stack, text};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Pixels, Point, Rectangle, Renderer,
    Shadow, Size,
};
use std::fmt;
use std::time::Duration;

const DEFAULT_DURATION_SECS: f32 = 0.3;
const DEFAULT_DOT_SIZE: f32 = 8.0;
const DEFAULT_HOVER_SCALE: f32 = 100.8;
const DEFAULT_GAP: f32 = 8.0;
const DEFAULT_BORDER_RADIUS: f32 = 999.0;
const DEFAULT_BORDER_WIDTH: f32 = 1.0;
const IDLE_LABEL_SLIDE_PX: f32 = 48.0;
/// CSS starts at `translate-x-12`; settled hover centers the row (`justify-center`).
const HOVER_LAYER_FROM_X: f32 = 48.0;
const HOVER_LAYER_TO_X: f32 = 0.0;
const DEFAULT_FONT_SIZE: f32 = 16.0;

/// CSS `ease` control points (`cubic-bezier(0.25, 0.1, 0.25, 1)`).
const CSS_EASE_X1: f32 = 0.25;
const CSS_EASE_Y1: f32 = 0.1;
const CSS_EASE_X2: f32 = 0.25;
const CSS_EASE_Y2: f32 = 1.0;

const DEFAULT_PRIMARY: Color = Color::from_rgb8(0x17, 0x17, 0x17);

/// Optional trailing glyph on the hover layer (any icon font / character).
///
/// The svelte demo uses Lucide `ArrowRight` at **24×24**; pass that from the
/// app if you want the same look — the widget does not hard-code an icon.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveHoverIcon {
    content: String,
    font: Font,
    size: f32,
}

impl InteractiveHoverIcon {
    /// Creates an icon glyph drawn with `font` at `size` logical pixels.
    pub fn new(content: impl Into<String>, font: Font, size: f32) -> Self {
        Self {
            content: content.into(),
            font,
            size: size.max(1.0),
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn font(&self) -> Font {
        self.font
    }

    pub fn size(&self) -> f32 {
        self.size
    }
}

/// Easing applied to hover progress for visual interpolation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum InteractiveHoverEase {
    Linear,
    /// CSS default `ease` (`cubic-bezier(0.25, 0.1, 0.25, 1)`).
    #[default]
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// Builder for the Magic UI interactive hover button.
#[must_use = "builders do nothing unless turned into an iced Element"]
pub struct InteractiveHoverButton<Message> {
    label: String,
    hover_icon: Option<InteractiveHoverIcon>,
    on_press: Option<Message>,
    background: Color,
    primary: Color,
    idle_foreground: Color,
    hover_foreground: Color,
    border_color: Color,
    border_width: f32,
    border_radius: f32,
    dot_size: f32,
    hover_scale: f32,
    duration_secs: f32,
    padding: Padding,
    gap: f32,
    font_size: f32,
    ease: InteractiveHoverEase,
    width: Length,
    height: Length,
}

impl<Message> fmt::Debug for InteractiveHoverButton<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InteractiveHoverButton")
            .field("label", &self.label)
            .field("hover_icon", &self.hover_icon)
            .field("on_press", &self.on_press.is_some())
            .field("background", &self.background)
            .field("primary", &self.primary)
            .field("idle_foreground", &self.idle_foreground)
            .field("hover_foreground", &self.hover_foreground)
            .field("border_color", &self.border_color)
            .field("border_width", &self.border_width)
            .field("border_radius", &self.border_radius)
            .field("dot_size", &self.dot_size)
            .field("hover_scale", &self.hover_scale)
            .field("duration_secs", &self.duration_secs)
            .field("padding", &self.padding)
            .field("gap", &self.gap)
            .field("font_size", &self.font_size)
            .field("ease", &self.ease)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl<Message> InteractiveHoverButton<Message> {
    /// Creates a button with svelte / Magic UI defaults (`Hover Me` in docs).
    ///
    /// No hover icon by default — add one with [`Self::hover_icon`] (docs demo
    /// uses Lucide `ArrowRight` at 24px).
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            hover_icon: None,
            on_press: None,
            background: Color::WHITE,
            primary: DEFAULT_PRIMARY,
            idle_foreground: DEFAULT_PRIMARY,
            hover_foreground: Color::WHITE,
            border_color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
            border_width: DEFAULT_BORDER_WIDTH,
            border_radius: DEFAULT_BORDER_RADIUS,
            dot_size: DEFAULT_DOT_SIZE,
            hover_scale: DEFAULT_HOVER_SCALE,
            duration_secs: DEFAULT_DURATION_SECS,
            padding: Padding::from([8.0, 24.0]),
            gap: DEFAULT_GAP,
            font_size: DEFAULT_FONT_SIZE,
            ease: InteractiveHoverEase::Ease,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn clear_on_press(mut self) -> Self {
        self.on_press = None;
        self
    }

    /// Optional trailing icon on the hover layer (any glyph/font). Centered with the label.
    pub fn hover_icon(mut self, icon: InteractiveHoverIcon) -> Self {
        self.hover_icon = Some(icon);
        self
    }

    /// Removes a previously set hover icon — hover shows only the centered label.
    pub fn clear_hover_icon(mut self) -> Self {
        self.hover_icon = None;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn primary(mut self, color: Color) -> Self {
        self.primary = color;
        self
    }

    pub fn idle_foreground(mut self, color: Color) -> Self {
        self.idle_foreground = color;
        self
    }

    pub fn hover_foreground(mut self, color: Color) -> Self {
        self.hover_foreground = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width.max(0.0);
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius.max(0.0);
        self
    }

    pub fn dot_size(mut self, size: f32) -> Self {
        self.dot_size = size.max(1.0);
        self
    }

    pub fn hover_scale(mut self, scale: f32) -> Self {
        self.hover_scale = scale.max(1.0);
        self
    }

    pub fn duration_secs(mut self, duration_secs: f32) -> Self {
        self.duration_secs = duration_secs.max(0.01);
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.max(1.0);
        self
    }

    pub fn ease(mut self, ease: InteractiveHoverEase) -> Self {
        self.ease = ease;
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Builds the interactive element for the given animation clock.
    pub fn into_element<'a>(self, elapsed: Duration) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        let radius = self.border_radius;
        let background = self.background;
        let border_color = self.border_color;
        let border_width = self.border_width;
        let padding = self.padding;
        let width = self.width;
        let height = self.height;
        let dot_size = self.dot_size;
        let gap = self.gap;
        let font_size = self.font_size;

        let sizing_label = text(self.label.clone())
            .size(font_size)
            .color(Color::TRANSPARENT)
            .font(semibold_font());
        let dot_spacer = container(iced::widget::Space::new())
            .width(Length::Fixed(dot_size))
            .height(Length::Fixed(dot_size));
        let sizing_row = row![dot_spacer, sizing_label].spacing(gap);

        let mut control = button(sizing_row)
            .padding(padding)
            .style(move |_theme, _status| interactive_hover_button_style(radius));

        if let Some(on_press) = self.on_press {
            control = control.on_press(on_press);
        }

        let overlay = effect_canvas::<_, Message>(
            InteractiveHoverProgram {
                elapsed,
                label: self.label,
                hover_icon: self.hover_icon,
                primary: self.primary,
                idle_foreground: self.idle_foreground,
                hover_foreground: self.hover_foreground,
                border_radius: self.border_radius,
                dot_size: self.dot_size,
                hover_scale: self.hover_scale,
                duration_secs: self.duration_secs,
                padding: self.padding,
                gap: self.gap,
                font_size: self.font_size,
                ease: self.ease,
            },
            EffectCanvasProps::new()
                .width(Length::Fill)
                .height(Length::Fill),
        );

        let overlay_layer: Element<'a, Message> = container(overlay)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let layered = stack(vec![control.into()]).push(overlay_layer).clip(true);

        container(layered)
            .width(width)
            .height(height)
            .clip(true)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(background)),
                text_color: None,
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: Radius::from(radius),
                },
                shadow: Shadow::default(),
                snap: false,
            })
            .into()
    }

    pub fn view<'a>(self, elapsed: Duration) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        self.into_element(elapsed)
    }
}

fn interactive_hover_button_style(radius: f32) -> button::Style {
    button::Style {
        background: None,
        text_color: Color::TRANSPARENT,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: Radius::from(radius),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn semibold_font() -> Font {
    Font {
        weight: Weight::Semibold,
        style: Style::Normal,
        ..Font::DEFAULT
    }
}

#[derive(Clone, Copy, Debug)]
struct HoverState {
    hovered: bool,
    anchor_progress: f32,
    anchor_elapsed_secs: f32,
}

impl Default for HoverState {
    fn default() -> Self {
        Self {
            hovered: false,
            anchor_progress: 0.0,
            anchor_elapsed_secs: 0.0,
        }
    }
}

struct InteractiveHoverProgram {
    elapsed: Duration,
    label: String,
    hover_icon: Option<InteractiveHoverIcon>,
    primary: Color,
    idle_foreground: Color,
    hover_foreground: Color,
    border_radius: f32,
    dot_size: f32,
    hover_scale: f32,
    duration_secs: f32,
    padding: Padding,
    gap: f32,
    font_size: f32,
    ease: InteractiveHoverEase,
}

impl<Message> EffectProgram<Message> for InteractiveHoverProgram {
    type State = HoverState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Option<canvas::Action<Message>> {
        let mut dirty = false;
        let over = cursor.is_over(bounds);
        let elapsed_secs = self.elapsed.as_secs_f32();

        if state.hovered != over {
            let delta = (elapsed_secs - state.anchor_elapsed_secs).max(0.0);
            state.anchor_progress = advance_progress(
                state.anchor_progress,
                state.hovered,
                delta,
                self.duration_secs,
            );
            state.anchor_elapsed_secs = elapsed_secs;
            state.hovered = over;
            dirty = true;
        }

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Mouse(mouse::Event::CursorEntered)
            | Event::Mouse(mouse::Event::CursorLeft) => {
                dirty = true;
            }
            _ => {}
        }

        let progress = current_progress(state, elapsed_secs, self.duration_secs);
        let target = if state.hovered { 1.0 } else { 0.0 };
        if (progress - target).abs() > f32::EPSILON {
            dirty = true;
        }

        dirty.then(canvas::Action::request_redraw)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let elapsed_secs = self.elapsed.as_secs_f32();
        let progress = current_progress(state, elapsed_secs, self.duration_secs);
        let t = apply_ease(progress, self.ease);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let size = bounds.size();

        if size.width < 1.0 || size.height < 1.0 {
            return vec![frame.into_geometry()];
        }

        let center_y = size.height * 0.5;
        let dot_center_x = self.padding.left + self.dot_size * 0.5;
        let dot_center = Point::new(dot_center_x, center_y);

        let scale = lerp_f32(1.0, self.hover_scale, t);
        let fill_radius = self.dot_size * 0.5 * scale;
        let effective_radius = effective_pill_radius(self.border_radius, size);
        paint_primary_fill(
            &mut frame,
            size,
            dot_center,
            fill_radius,
            effective_radius,
            self.primary,
        );

        let idle_label_x = self.padding.left + self.dot_size + self.gap + IDLE_LABEL_SLIDE_PX * t;
        let idle_alpha = 1.0 - t;
        if idle_alpha > 0.01 {
            paint_label(
                &mut frame,
                &self.label,
                Point::new(idle_label_x, center_y),
                self.font_size,
                Color {
                    a: idle_alpha * self.idle_foreground.a,
                    ..self.idle_foreground
                },
            );
        }

        let hover_x = size.width * 0.5 + lerp_f32(HOVER_LAYER_FROM_X, HOVER_LAYER_TO_X, t);
        let hover_alpha = t;
        if hover_alpha > 0.01 {
            paint_hover_row(
                &mut frame,
                &self.label,
                self.hover_icon.as_ref(),
                Point::new(hover_x, center_y),
                self.font_size,
                self.gap,
                Color {
                    a: hover_alpha * self.hover_foreground.a,
                    ..self.hover_foreground
                },
            );
        }

        vec![frame.into_geometry()]
    }
}

fn effective_pill_radius(border_radius: f32, size: Size) -> f32 {
    border_radius.min(size.width * 0.5).min(size.height * 0.5)
}

/// Expanding primary disk (`scale-[100.8]`), clipped to the stadium without
/// exterior masks (EvenOdd AABB∖pill caused vertical seams + rectangular chrome
/// during theme blends).
fn paint_primary_fill(
    frame: &mut canvas::Frame,
    size: Size,
    dot_center: Point,
    fill_radius: f32,
    radius: f32,
    primary: Color,
) {
    let r = radius.max(0.0).min(size.width * 0.5).min(size.height * 0.5);
    let pill = canvas::Path::rounded_rectangle(Point::ORIGIN, size, Radius::from(r));

    // While the disk still fits inside the stadium vertically, draw the circle.
    // As soon as it would spill past the pill, fill the pill path only — no mask.
    if fill_radius <= r - 0.5 {
        frame.fill(
            &canvas::Path::circle(dot_center, fill_radius),
            Fill::from(primary),
        );
        return;
    }

    frame.fill(&pill, Fill::from(primary));
}

fn paint_label(
    frame: &mut canvas::Frame,
    content: &str,
    position: Point,
    font_size: f32,
    color: Color,
) {
    paint_label_with_font(frame, content, position, font_size, color, semibold_font());
}

fn paint_label_with_font(
    frame: &mut canvas::Frame,
    content: &str,
    position: Point,
    font_size: f32,
    color: Color,
    font: Font,
) {
    // Lift/lower so glyph ink — not the line box — hits `position.y`.
    let y = position.y - vertical_ink_bias(content, font_size, font);
    frame.fill_text(canvas::Text {
        content: content.to_owned(),
        position: Point::new(position.x, y),
        color,
        size: Pixels(font_size),
        line_height: LineHeight::Absolute(Pixels(font_size)),
        font,
        align_x: iced::alignment::Horizontal::Left.into(),
        align_y: Vertical::Center,
        shaping: Shaping::Advanced,
        ..canvas::Text::default()
    });
}

fn paint_hover_row(
    frame: &mut canvas::Frame,
    label: &str,
    icon: Option<&InteractiveHoverIcon>,
    center: Point,
    font_size: f32,
    gap: f32,
    color: Color,
) {
    // CSS hover layer: `flex items-center justify-center gap-2`.
    // Label uses ink-biased Center; icon is laid out in its declared square
    // (Lucide advances are often fatter than the SVG box).
    let label_font = semibold_font();
    let label_width = measure_glyph_bounds(label, font_size, label_font).width;
    let icon_box = icon.map(InteractiveHoverIcon::size).unwrap_or(0.0);
    let total_width = if icon.is_some() {
        label_width + gap + icon_box
    } else {
        label_width
    };
    let start_x = center.x - total_width * 0.5;
    let label_y = center.y - vertical_ink_bias(label, font_size, label_font);

    frame.fill_text(canvas::Text {
        content: label.to_owned(),
        position: Point::new(start_x, label_y),
        color,
        size: Pixels(font_size),
        line_height: LineHeight::Absolute(Pixels(font_size)),
        font: label_font,
        align_x: iced::alignment::Horizontal::Left.into(),
        align_y: Vertical::Center,
        shaping: Shaping::Advanced,
        ..canvas::Text::default()
    });

    if let Some(icon) = icon {
        let icon_left = start_x + label_width + gap;
        frame.fill_text(canvas::Text {
            content: icon.content().to_owned(),
            position: Point::new(icon_left + icon_box * 0.5, center.y),
            color,
            size: Pixels(icon.size()),
            line_height: LineHeight::Absolute(Pixels(icon.size())),
            font: icon.font(),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: Vertical::Center,
            shaping: Shaping::Advanced,
            ..canvas::Text::default()
        });
    }
}

fn measure_glyph_bounds(content: &str, font_size: f32, font: Font) -> Size {
    if content.is_empty() {
        return Size::ZERO;
    }

    glyph_paragraph(content, font_size, font).min_bounds()
}

fn glyph_paragraph(content: &str, font_size: f32, font: Font) -> GraphicsParagraph {
    GraphicsParagraph::with_text(text_layout::Text {
        content,
        bounds: Size::new(f32::INFINITY, f32::INFINITY),
        size: Pixels(font_size),
        line_height: LineHeight::Absolute(Pixels(font_size)),
        font,
        align_x: TextAlign::Left,
        align_y: Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
}

/// How far below the line-box mid the Latin ink sits (positive → shift draw up).
///
/// iced `Vertical::Center` centers `min_bounds` (the line box). Cap/x-height ink
/// for UI labels usually sits lower in that box, so plain Center looks dropped.
fn vertical_ink_bias(content: &str, font_size: f32, font: Font) -> f32 {
    if content.is_empty() {
        return 0.0;
    }

    let paragraph = glyph_paragraph(content, font_size, font);
    let box_height = paragraph.min_bounds().height.max(font_size);
    let box_mid = box_height * 0.5;

    let mut ink_top = f32::INFINITY;
    let mut ink_bottom = f32::NEG_INFINITY;
    for run in paragraph.buffer().layout_runs() {
        for glyph in run.glyphs.iter() {
            // Approx ink band: from a bit above the baseline to the baseline.
            // Cap height ≈ 0.7·em; y_offset nudges per-glyph.
            let baseline = run.line_y + glyph.y_offset;
            let top = baseline - glyph.font_size * 0.7;
            let bottom = baseline;
            ink_top = ink_top.min(top);
            ink_bottom = ink_bottom.max(bottom);
        }
    }

    if !ink_top.is_finite() {
        return 0.0;
    }

    let ink_mid = (ink_top + ink_bottom) * 0.5;
    ink_mid - box_mid
}

fn current_progress(state: &HoverState, elapsed_secs: f32, duration_secs: f32) -> f32 {
    let delta = (elapsed_secs - state.anchor_elapsed_secs).max(0.0);
    advance_progress(state.anchor_progress, state.hovered, delta, duration_secs)
}

fn advance_progress(current: f32, hovered: bool, delta_secs: f32, duration_secs: f32) -> f32 {
    let target = if hovered { 1.0 } else { 0.0 };
    if (current - target).abs() < f32::EPSILON {
        return target;
    }

    let step = delta_secs / duration_secs.max(0.01);
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

fn apply_ease(t: f32, ease: InteractiveHoverEase) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        InteractiveHoverEase::Linear => t,
        InteractiveHoverEase::Ease => {
            cubic_bezier_ease(t, CSS_EASE_X1, CSS_EASE_Y1, CSS_EASE_X2, CSS_EASE_Y2)
        }
        InteractiveHoverEase::EaseIn => t * t,
        InteractiveHoverEase::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        InteractiveHoverEase::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
    }
}

fn lerp_f32(from: f32, to: f32, progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    from + (to - from) * progress
}

fn cubic_bezier_ease(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    let sample = |t: f32, p1: f32, p2: f32| {
        let c = 3.0 * p1;
        let b = 3.0 * (p2 - p1) - c;
        let a = 1.0 - c - b;
        ((a * t + b) * t + c) * t
    };
    let derivative = |t: f32, p1: f32, p2: f32| {
        let c = 3.0 * p1;
        let b = 3.0 * (p2 - p1) - c;
        let a = 1.0 - c - b;
        (3.0 * a * t * t) + (2.0 * b * t) + c
    };

    let mut u = t;
    for _ in 0..6 {
        let x = sample(u, x1, x2);
        let dx = derivative(u, x1, x2);
        if dx.abs() < 1e-6 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0.0, 1.0);
    }

    sample(u, y1, y2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_hover_button_defaults_match_svelte() {
        let button = InteractiveHoverButton::<()>::new("Hover Me");

        assert_eq!(button.label, "Hover Me");
        assert_eq!(button.duration_secs, DEFAULT_DURATION_SECS);
        assert_eq!(button.dot_size, DEFAULT_DOT_SIZE);
        assert_eq!(button.hover_scale, DEFAULT_HOVER_SCALE);
        assert_eq!(button.gap, DEFAULT_GAP);
        assert_eq!(button.border_radius, DEFAULT_BORDER_RADIUS);
        assert_eq!(button.border_width, DEFAULT_BORDER_WIDTH);
        assert_eq!(button.font_size, DEFAULT_FONT_SIZE);
        assert!(button.hover_icon.is_none());
        assert_eq!(button.ease, InteractiveHoverEase::Ease);
        assert_eq!(button.background, Color::WHITE);
        assert_eq!(button.primary, DEFAULT_PRIMARY);
        assert_eq!(button.idle_foreground, DEFAULT_PRIMARY);
        assert_eq!(button.hover_foreground, Color::WHITE);
        assert_eq!(button.border_color, Color::from_rgba(0.0, 0.0, 0.0, 0.1));
        assert_eq!(button.padding, Padding::from([8.0, 24.0]));
    }

    #[test]
    fn setters_validate_ranges() {
        let button = InteractiveHoverButton::<()>::new("x")
            .duration_secs(0.0)
            .dot_size(0.0)
            .hover_scale(0.5)
            .border_width(-2.0)
            .border_radius(-4.0)
            .gap(-1.0)
            .font_size(0.0);

        assert_eq!(button.duration_secs, 0.01);
        assert_eq!(button.dot_size, 1.0);
        assert_eq!(button.hover_scale, 1.0);
        assert_eq!(button.border_width, 0.0);
        assert_eq!(button.border_radius, 0.0);
        assert_eq!(button.gap, 0.0);
        assert_eq!(button.font_size, 1.0);
    }

    #[test]
    fn hover_icon_is_optional_and_clearable() {
        let icon = InteractiveHoverIcon::new("→", Font::DEFAULT, 24.0);
        let with_icon = InteractiveHoverButton::<()>::new("Hover Me").hover_icon(icon.clone());
        assert_eq!(with_icon.hover_icon.as_ref().map(|i| i.size()), Some(24.0));

        let cleared = with_icon.clear_hover_icon();
        assert!(cleared.hover_icon.is_none());
    }

    #[test]
    fn vertical_ink_bias_is_finite_for_label() {
        let bias = vertical_ink_bias("Hover Me", DEFAULT_FONT_SIZE, semibold_font());
        assert!(bias.is_finite(), "bias={bias}");
        // Latin UI ink usually sits a little below the line-box mid.
        assert!(bias.abs() < DEFAULT_FONT_SIZE, "bias={bias} out of range");
    }

    #[test]
    fn ease_endpoints_are_zero_and_one() {
        for ease in [
            InteractiveHoverEase::Linear,
            InteractiveHoverEase::Ease,
            InteractiveHoverEase::EaseIn,
            InteractiveHoverEase::EaseOut,
            InteractiveHoverEase::EaseInOut,
        ] {
            assert!((apply_ease(0.0, ease)).abs() < f32::EPSILON);
            assert!((apply_ease(1.0, ease) - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn advance_progress_moves_toward_hover_target() {
        let hovered = advance_progress(0.0, true, DEFAULT_DURATION_SECS, DEFAULT_DURATION_SECS);
        assert!((hovered - 1.0).abs() < f32::EPSILON);

        let idle = advance_progress(1.0, false, DEFAULT_DURATION_SECS, DEFAULT_DURATION_SECS);
        assert!(idle.abs() < f32::EPSILON);

        let mid = advance_progress(
            0.0,
            true,
            DEFAULT_DURATION_SECS * 0.5,
            DEFAULT_DURATION_SECS,
        );
        assert!((mid - 0.5).abs() < 0.001);
    }

    #[test]
    fn ease_is_monotonic_on_unit_interval() {
        let samples = [0.0, 0.25, 0.5, 0.75, 1.0];
        for ease in [
            InteractiveHoverEase::Linear,
            InteractiveHoverEase::Ease,
            InteractiveHoverEase::EaseIn,
            InteractiveHoverEase::EaseOut,
            InteractiveHoverEase::EaseInOut,
        ] {
            let mut previous = apply_ease(samples[0], ease);
            for &sample in &samples[1..] {
                let next = apply_ease(sample, ease);
                assert!(next >= previous, "ease {:?} dipped at {}", ease, sample);
                previous = next;
            }
        }
    }

    #[test]
    fn lerp_f32_clamps_progress() {
        assert!((lerp_f32(0.0, 10.0, 0.5) - 5.0).abs() < f32::EPSILON);
        assert!((lerp_f32(0.0, 10.0, 2.0) - 10.0).abs() < f32::EPSILON);
    }
}
