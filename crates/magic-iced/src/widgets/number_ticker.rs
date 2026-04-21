use crate::primitives::{AnimatedValue, MotionSpec};
use iced::Element;
use iced::time::Instant;
use iced::widget::text;
use iced_shadcn::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NumberTickerDirection {
    #[default]
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NumberTickerProps {
    pub value: f64,
    pub start_value: f64,
    pub direction: NumberTickerDirection,
    pub decimal_places: u8,
    pub motion: MotionSpec,
    pub prefix: String,
    pub suffix: String,
    pub font_size: u16,
}

impl Default for NumberTickerProps {
    fn default() -> Self {
        Self {
            value: 100.0,
            start_value: 0.0,
            direction: NumberTickerDirection::Up,
            decimal_places: 0,
            motion: MotionSpec::default(),
            prefix: String::new(),
            suffix: String::new(),
            font_size: 36,
        }
    }
}

impl NumberTickerProps {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            ..Self::default()
        }
    }

    pub fn start_value(mut self, start_value: f64) -> Self {
        self.start_value = start_value;
        self
    }

    pub fn direction(mut self, direction: NumberTickerDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn decimal_places(mut self, decimal_places: u8) -> Self {
        self.decimal_places = decimal_places;
        self
    }

    pub fn motion(mut self, motion: MotionSpec) -> Self {
        self.motion = motion;
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn font_size(mut self, font_size: u16) -> Self {
        self.font_size = font_size.max(1);
        self
    }
}

#[derive(Clone, Debug)]
pub struct NumberTickerState {
    animation: AnimatedValue,
    started: bool,
    fingerprint: NumberTickerFingerprint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NumberTickerFingerprint {
    value: f64,
    start_value: f64,
    direction: NumberTickerDirection,
}

impl NumberTickerState {
    pub fn new(props: &NumberTickerProps) -> Self {
        Self {
            animation: AnimatedValue::with_spec(initial_value(props) as f32, props.motion),
            started: false,
            fingerprint: NumberTickerFingerprint {
                value: props.value,
                start_value: props.start_value,
                direction: props.direction,
            },
        }
    }

    pub fn tick(&mut self, now: Instant, props: &NumberTickerProps) {
        let fingerprint = NumberTickerFingerprint {
            value: props.value,
            start_value: props.start_value,
            direction: props.direction,
        };

        if !self.started || fingerprint != self.fingerprint {
            self.animation = AnimatedValue::with_spec(initial_value(props) as f32, props.motion);
            self.animation.transition(target_value(props) as f32, now);
            self.started = true;
            self.fingerprint = fingerprint;
            return;
        }

        self.animation.set_spec(props.motion);
    }

    pub fn value_at(&self, now: Instant) -> f64 {
        self.animation.value_at(now) as f64
    }

    pub fn display_text(&self, now: Instant, props: &NumberTickerProps) -> String {
        let value = format_number(self.value_at(now), props.decimal_places);
        format!("{}{}{}", props.prefix, value, props.suffix)
    }
}

pub fn number_ticker<'a, Message: 'a>(
    state: &NumberTickerState,
    now: Instant,
    props: &NumberTickerProps,
    theme: &Theme,
) -> Element<'a, Message> {
    text(state.display_text(now, props))
        .size(u32::from(props.font_size))
        .style({
            let color = theme.palette.foreground;
            move |_| iced::widget::text::Style { color: Some(color) }
        })
        .into()
}

fn initial_value(props: &NumberTickerProps) -> f64 {
    match props.direction {
        NumberTickerDirection::Up => props.start_value,
        NumberTickerDirection::Down => props.value,
    }
}

fn target_value(props: &NumberTickerProps) -> f64 {
    match props.direction {
        NumberTickerDirection::Up => props.value,
        NumberTickerDirection::Down => props.start_value,
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
    use iced::time::Duration;

    #[test]
    fn format_number_adds_grouping_and_decimals() {
        assert_eq!(format_number(12345.0, 0), "12,345");
        assert_eq!(format_number(12345.678, 2), "12,345.68");
        assert_eq!(format_number(-1000.0, 0), "-1,000");
    }

    #[test]
    fn ticker_starts_from_expected_direction() {
        let up = NumberTickerProps::new(4200.0).start_value(0.0);
        let down = NumberTickerProps::new(4200.0)
            .start_value(0.0)
            .direction(NumberTickerDirection::Down);

        assert_eq!(initial_value(&up), 0.0);
        assert_eq!(target_value(&up), 4200.0);
        assert_eq!(initial_value(&down), 4200.0);
        assert_eq!(target_value(&down), 0.0);
    }

    #[test]
    fn ticker_transitions_after_tick() {
        let now = Instant::now();
        let props = NumberTickerProps::new(100.0).motion(
            MotionSpec::new()
                .duration(twill::prelude::TransitionDuration::Ms100)
                .easing(twill::prelude::Easing::Linear),
        );
        let mut state = NumberTickerState::new(&props);

        state.tick(now, &props);

        let halfway = state.value_at(now + Duration::from_millis(50));
        assert!(
            halfway > 45.0 && halfway < 55.0,
            "expected halfway value, got {halfway}"
        );
    }
}
