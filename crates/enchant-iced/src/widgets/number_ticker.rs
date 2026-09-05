//! Builder-first animated number ticker (Magic UI `NumberTicker`).
//!
//! Spring physics are integrated deterministically from host-supplied [`Duration`]
//! via [`NumberTicker::value_at`] / [`NumberTicker::into_element`].

use iced::font::Weight;
use iced::widget::{row, text};
use iced::{Color, Element, Font, Length};
use std::fmt;
use std::time::Duration;

const SPRING_STIFFNESS: f64 = 100.0;
const SPRING_DAMPING: f64 = 60.0;
const SPRING_DT: f64 = 1.0 / 60.0;
const MAX_ANIM_SECS: f64 = 4.0;
const SETTLE_VELOCITY: f64 = 0.01;
const SETTLE_POSITION: f64 = 0.01;

/// Count direction for the ticker animation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum NumberTickerDirection {
    /// Animate from [`NumberTicker::start_value`] toward the constructed value.
    #[default]
    Up,
    /// Animate from the constructed value toward [`NumberTicker::start_value`].
    Down,
}

/// Builder for a spring-animated number display.
///
/// # Examples
///
/// ```rust,no_run
/// use enchant_iced::NumberTicker;
/// use std::time::Duration;
///
/// fn ticker<'a, Message: 'a>(elapsed: Duration) -> iced::Element<'a, Message> {
///     NumberTicker::new(100.0)
///         .font_size(96.0)
///         .into_element(elapsed)
/// }
/// ```
#[must_use = "builders do nothing unless turned into an iced Element"]
#[derive(Clone)]
pub struct NumberTicker {
    value: f64,
    start_value: f64,
    direction: NumberTickerDirection,
    delay_secs: f64,
    decimal_places: u8,
    prefix: String,
    suffix: String,
    once: bool,
    font_size: f32,
    font: Font,
    color: Color,
    letter_spacing: f32,
    width: Length,
    height: Length,
}

impl fmt::Debug for NumberTicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NumberTicker")
            .field("value", &self.value)
            .field("start_value", &self.start_value)
            .field("direction", &self.direction)
            .field("delay_secs", &self.delay_secs)
            .field("decimal_places", &self.decimal_places)
            .field("prefix", &self.prefix)
            .field("suffix", &self.suffix)
            .field("once", &self.once)
            .field("font_size", &self.font_size)
            .field("font", &self.font)
            .field("color", &self.color)
            .field("letter_spacing", &self.letter_spacing)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl NumberTicker {
    /// Creates a ticker that animates toward `value` (Magic UI required prop).
    pub fn new(value: f64) -> Self {
        let font_size = 96.0;
        Self {
            value,
            start_value: 0.0,
            direction: NumberTickerDirection::Up,
            delay_secs: 0.0,
            decimal_places: 0,
            prefix: String::new(),
            suffix: String::new(),
            once: true,
            font_size,
            font: Font {
                weight: Weight::Medium,
                ..Font::DEFAULT
            },
            color: Color::BLACK,
            letter_spacing: 0.05 * font_size,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    /// Starting numeric value when [`NumberTickerDirection::Up`] (default `0`).
    pub fn start_value(mut self, start_value: f64) -> Self {
        self.start_value = start_value;
        self
    }

    /// Animation direction (default [`NumberTickerDirection::Up`]).
    pub fn direction(mut self, direction: NumberTickerDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Delay before the spring starts, in seconds (default `0`).
    pub fn delay_secs(mut self, delay_secs: f64) -> Self {
        self.delay_secs = delay_secs.max(0.0);
        self
    }

    /// Fixed fraction digits for en-US formatting (clamped to `0..=12`).
    pub fn decimal_places(mut self, decimal_places: u8) -> Self {
        self.decimal_places = decimal_places.min(12);
        self
    }

    /// Literal prefix concatenated before the formatted number.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Literal suffix concatenated after the formatted number.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// When `true` (default), host should treat one contiguous play as the animation.
    pub fn once(mut self, once: bool) -> Self {
        self.once = once;
        self
    }

    /// Text size in logical pixels (default `96`, minimum `1`).
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size.max(1.0);
        self.letter_spacing = 0.05 * self.font_size;
        self
    }

    /// Font face and weight (default medium weight).
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Text color (default [`Color::BLACK`]).
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Extra letter spacing in logical pixels (`tracking-wider` ≈ `0.05em`).
    pub fn letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }

    /// Widget width (default [`Length::Shrink`]).
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Widget height (default [`Length::Shrink`]).
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Spring start value for the current direction.
    pub fn initial_value(&self) -> f64 {
        match self.direction {
            NumberTickerDirection::Up => self.start_value,
            NumberTickerDirection::Down => self.value,
        }
    }

    /// Spring target value for the current direction.
    pub fn target_value(&self) -> f64 {
        match self.direction {
            NumberTickerDirection::Up => self.value,
            NumberTickerDirection::Down => self.start_value,
        }
    }

    /// Numeric value at `elapsed` after delay and spring integration.
    pub fn value_at(&self, elapsed: Duration) -> f64 {
        let elapsed_secs = elapsed.as_secs_f64();
        let anim_secs = (elapsed_secs - self.delay_secs).max(0.0);
        spring_position(self.initial_value(), self.target_value(), anim_secs)
    }

    /// Formatted display string: `prefix` + en-US number + `suffix` (no extra spaces).
    pub fn display_text(&self, elapsed: Duration) -> String {
        let value = self.value_at(elapsed);
        let number = format_number(value, self.decimal_places);
        format!("{}{}{}", self.prefix, number, self.suffix)
    }

    /// Builds the text element for the given animation clock.
    pub fn into_element<'a, Message: 'a>(self, elapsed: Duration) -> Element<'a, Message> {
        let display = self.display_text(elapsed);

        if display.is_empty() {
            return row::<Message, _, _>(std::iter::empty())
                .width(self.width)
                .height(self.height)
                .into();
        }

        if self.letter_spacing <= 0.0 {
            return text(display)
                .size(self.font_size)
                .font(self.font)
                .color(self.color)
                .width(self.width)
                .height(self.height)
                .into();
        }

        let glyphs: Vec<Element<'a, Message>> = display
            .chars()
            .map(|ch| {
                text(ch.to_string())
                    .size(self.font_size)
                    .font(self.font)
                    .color(self.color)
                    .into()
            })
            .collect();

        row(glyphs)
            .spacing(self.letter_spacing)
            .width(self.width)
            .height(self.height)
            .into()
    }

    /// Alias for [`Self::into_element`].
    pub fn view<'a, Message: 'a>(self, elapsed: Duration) -> Element<'a, Message> {
        self.into_element(elapsed)
    }
}

fn spring_position(from: f64, to: f64, elapsed_anim_secs: f64) -> f64 {
    if elapsed_anim_secs <= 0.0 {
        return from;
    }

    if (from - to).abs() < SETTLE_POSITION {
        return to;
    }

    if elapsed_anim_secs >= MAX_ANIM_SECS {
        return to;
    }

    let mut position = from;
    let mut velocity = 0.0;
    let mut elapsed = 0.0;

    while elapsed < elapsed_anim_secs {
        let spring_force = -SPRING_STIFFNESS * (position - to);
        let damping_force = -SPRING_DAMPING * velocity;
        let acceleration = spring_force + damping_force;

        velocity += acceleration * SPRING_DT;
        position += velocity * SPRING_DT;
        elapsed += SPRING_DT;

        if velocity.abs() < SETTLE_VELOCITY && (position - to).abs() < SETTLE_POSITION {
            return to;
        }

        if elapsed >= MAX_ANIM_SECS {
            return to;
        }
    }

    if velocity.abs() < SETTLE_VELOCITY && (position - to).abs() < SETTLE_POSITION {
        to
    } else {
        position
    }
}

fn format_number(value: f64, decimal_places: u8) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let factor_u64 = 10_u64.pow(decimal_places as u32);
    let factor = factor_u64 as f64;
    let scaled = (value.abs() * factor).round() as u64;
    let integer = scaled / factor_u64.max(1);
    let fraction = scaled % factor_u64.max(1);
    let grouped = group_digits(integer);

    if decimal_places == 0 {
        format!("{sign}{grouped}")
    } else {
        format!(
            "{sign}{grouped}.{:0width$}",
            fraction,
            width = decimal_places as usize
        )
    }
}

fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, ch) in digits.chars().rev().enumerate() {
        if index != 0 && index % 3 == 0 {
            output.push(',');
        }
        output.push(ch);
    }

    output.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn number_ticker_defaults_match_magic_ui() {
        let t = NumberTicker::new(100.0);
        assert_eq!(t.start_value, 0.0);
        assert_eq!(t.direction, NumberTickerDirection::Up);
        assert_eq!(t.delay_secs, 0.0);
        assert_eq!(t.decimal_places, 0);
        assert!(t.once);
        assert!((t.font_size - 96.0).abs() < f32::EPSILON);
    }

    #[test]
    fn direction_sets_initial_and_target() {
        let up = NumberTicker::new(4200.0).start_value(0.0);
        let down = NumberTicker::new(4200.0)
            .start_value(0.0)
            .direction(NumberTickerDirection::Down);
        assert_eq!(up.initial_value(), 0.0);
        assert_eq!(up.target_value(), 4200.0);
        assert_eq!(down.initial_value(), 4200.0);
        assert_eq!(down.target_value(), 0.0);
    }

    #[test]
    fn delay_holds_at_initial_then_moves() {
        let t = NumberTicker::new(100.0).delay_secs(0.5);
        let at_250 = t.value_at(Duration::from_millis(250));
        assert!((at_250 - 0.0).abs() < 0.01);
        let at_800 = t.value_at(Duration::from_millis(800));
        assert!(at_800 > 1.0, "expected spring progress after delay, got {at_800}");
    }

    #[test]
    fn spring_settles_near_target() {
        let t = NumberTicker::new(100.0);
        let settled = t.value_at(Duration::from_secs(5));
        assert!((settled - 100.0).abs() < 0.01);
    }

    #[test]
    fn format_number_en_us() {
        assert_eq!(format_number(12345.0, 0), "12,345");
        assert_eq!(format_number(12345.678, 2), "12,345.68");
        assert_eq!(format_number(-1000.0, 0), "-1,000");
    }

    #[test]
    fn display_text_prefix_suffix_tight() {
        let t = NumberTicker::new(100.5)
            .decimal_places(2)
            .prefix("$")
            .suffix("USD");
        let s = t.display_text(Duration::from_secs(5));
        assert_eq!(s, "$100.50USD");
    }

    #[test]
    fn setters_validate_ranges() {
        let t = NumberTicker::new(1.0)
            .delay_secs(-1.0)
            .decimal_places(20)
            .font_size(0.0);
        assert_eq!(t.delay_secs, 0.0);
        assert!(t.font_size >= 1.0);
        assert_eq!(t.decimal_places, 12);
    }

    #[test]
    fn letter_spacing_defaults_and_resets_with_font_size() {
        let t = NumberTicker::new(100.0);
        assert!((t.letter_spacing - 0.05 * t.font_size).abs() < f32::EPSILON);

        let t = NumberTicker::new(100.0).letter_spacing(10.0);
        assert!((t.letter_spacing - 10.0).abs() < f32::EPSILON);

        let t = NumberTicker::new(100.0)
            .letter_spacing(10.0)
            .font_size(48.0);
        assert!((t.letter_spacing - 0.05 * 48.0).abs() < f32::EPSILON);
    }

    #[test]
    fn font_defaults_to_medium_weight() {
        let t = NumberTicker::new(100.0);
        assert_eq!(t.font.weight, Weight::Medium);
    }

    #[test]
    fn non_positive_letter_spacing_uses_single_text_path() {
        let t = NumberTicker::new(100.0).letter_spacing(0.0);
        assert!(t.letter_spacing <= 0.0);

        let t = NumberTicker::new(100.0).letter_spacing(-4.8);
        assert!(t.letter_spacing <= 0.0);
    }
}
