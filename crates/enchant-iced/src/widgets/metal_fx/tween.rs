//! Time-based tweens, from `engine/tween.ts`.

/// Easing curve of a [`Tween`].
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::Ease;
///
/// assert_eq!(Ease::Linear.apply(0.25), 0.25);
/// assert_eq!(Ease::Smoothstep.apply(0.5), 0.5);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Ease {
    /// `t`.
    #[default]
    Linear,
    /// `t² (3 − 2t)`.
    Smoothstep,
}

impl Ease {
    /// Maps `t` in `0..=1` through the curve.
    #[must_use]
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Self::Linear => t,
            Self::Smoothstep => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// A value moving from `from` to `to` over `duration_ms`.
///
/// Mirrors the reference: the clock is supplied by the caller, a tween that was
/// never started stays on `from`, and once it reaches `t = 1` it is done.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{Ease, Tween};
///
/// let mut tween = Tween::new(0.0, 1.0, 100.0, Ease::Linear);
/// tween.start(1000.0);
/// assert_eq!(tween.tick(1050.0), 0.5);
/// assert!(!tween.is_done());
/// assert_eq!(tween.tick(1200.0), 1.0);
/// assert!(tween.is_done());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween {
    from: f64,
    to: f64,
    duration_ms: f64,
    ease: Ease,
    start_ms: Option<f64>,
    value: f64,
    done: bool,
}

impl Tween {
    /// A tween that has not been started yet.
    #[must_use]
    pub fn new(from: f64, to: f64, duration_ms: f64, ease: Ease) -> Self {
        Self {
            from,
            to,
            duration_ms: duration_ms.max(f64::MIN_POSITIVE),
            ease,
            start_ms: None,
            value: from,
            done: false,
        }
    }

    /// Starts (or restarts) the tween at `now_ms`.
    pub fn start(&mut self, now_ms: f64) {
        self.start_ms = Some(now_ms);
        self.value = self.from;
        self.done = false;
    }

    /// Advances to `now_ms` and returns the current value.
    pub fn tick(&mut self, now_ms: f64) -> f64 {
        let Some(start) = self.start_ms else {
            return self.value;
        };
        if self.done {
            return self.value;
        }
        let t = ((now_ms - start) / self.duration_ms).min(1.0);
        self.value = self.from + (self.to - self.from) * self.ease.apply(t);
        if t >= 1.0 {
            self.done = true;
        }
        self.value
    }

    /// The last value returned by [`tick`](Self::tick).
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The target value.
    #[must_use]
    pub fn target(&self) -> f64 {
        self.to
    }

    /// Whether the tween has reached its end.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unstarted_tween_holds_its_start() {
        let mut t = Tween::new(2.0, 8.0, 10.0, Ease::Linear);
        assert_eq!(t.tick(1e9), 2.0);
        assert!(!t.is_done());
    }

    #[test]
    fn smoothstep_tween_hits_both_ends_and_midpoint() {
        let mut t = Tween::new(0.0, 1.0, 300.0, Ease::Smoothstep);
        t.start(0.0);
        assert_eq!(t.tick(0.0), 0.0);
        assert!((t.tick(150.0) - 0.5).abs() < 1e-12);
        assert_eq!(t.tick(300.0), 1.0);
        assert!(t.is_done());
        assert_eq!(t.tick(10_000.0), 1.0);
    }

    #[test]
    fn restart_resets_the_value() {
        let mut t = Tween::new(1.0, 0.0, 100.0, Ease::Linear);
        t.start(0.0);
        t.tick(100.0);
        assert!(t.is_done());
        t.start(500.0);
        assert!(!t.is_done());
        assert_eq!(t.value(), 1.0);
    }
}
