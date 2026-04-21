use iced::Subscription;
use iced::time::Duration;
use iced::widget::scrollable::{AbsoluteOffset, RelativeOffset, Viewport};
use iced::window;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeDriver {
    pub tick_ms: u64,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameDriver {
    pub active: bool,
}

impl Default for FrameDriver {
    fn default() -> Self {
        Self { active: true }
    }
}

impl Default for TimeDriver {
    fn default() -> Self {
        Self {
            tick_ms: 16,
            active: true,
        }
    }
}

impl TimeDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick_ms(mut self, tick_ms: u64) -> Self {
        self.tick_ms = tick_ms.max(1);
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn interval(&self) -> Duration {
        Duration::from_millis(self.tick_ms.max(1))
    }

    pub fn subscription<Message: 'static, F>(&self, map: F) -> Subscription<Message>
    where
        F: Fn(iced::time::Instant) -> Message + Copy + Send + 'static,
    {
        if self.active {
            iced::time::every(self.interval()).map(move |_| map(iced::time::Instant::now()))
        } else {
            Subscription::none()
        }
    }
}

impl FrameDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn subscription<Message: 'static, F>(&self, map: F) -> Subscription<Message>
    where
        F: Fn(iced::time::Instant) -> Message + Copy + Send + 'static,
    {
        if self.active {
            window::frames().map(map)
        } else {
            Subscription::none()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollSample {
    pub absolute: AbsoluteOffset,
    pub relative: RelativeOffset,
    pub bounds: iced::Rectangle,
    pub content_bounds: iced::Rectangle,
}

impl ScrollSample {
    pub fn new(
        bounds: iced::Rectangle,
        content_bounds: iced::Rectangle,
        absolute: AbsoluteOffset,
        relative: RelativeOffset,
    ) -> Self {
        Self {
            absolute,
            relative,
            bounds,
            content_bounds,
        }
    }

    pub fn segment_progress(&self, start: f32, end: f32) -> f32 {
        segment_progress(self.relative.y, start, end)
    }
}

impl From<Viewport> for ScrollSample {
    fn from(viewport: Viewport) -> Self {
        Self {
            absolute: viewport.absolute_offset(),
            relative: viewport.relative_offset(),
            bounds: viewport.bounds(),
            content_bounds: viewport.content_bounds(),
        }
    }
}

pub fn segment_progress(progress: f32, start: f32, end: f32) -> f32 {
    if !progress.is_finite() || !start.is_finite() || !end.is_finite() {
        return 0.0;
    }

    let span = (end - start).max(f32::EPSILON);
    ((progress - start) / span).clamp(0.0, 1.0)
}

pub fn marquee_offset(elapsed: Duration, speed_px_per_sec: f32, extent: f32) -> f32 {
    if extent <= f32::EPSILON {
        return 0.0;
    }

    let delta = elapsed.as_secs_f32() * speed_px_per_sec;
    delta.rem_euclid(extent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_driver_clamps_interval_to_positive_values() {
        let driver = TimeDriver::new().tick_ms(0);
        assert_eq!(driver.interval(), Duration::from_millis(1));
    }

    #[test]
    fn frame_driver_defaults_to_active() {
        let driver = FrameDriver::new();

        assert!(driver.active);
    }

    #[test]
    fn frame_driver_can_be_disabled() {
        let driver = FrameDriver::new().active(false);

        assert!(!driver.active);
    }

    #[test]
    fn drivers_accept_timestamped_message_mapping() {
        let _frame: Subscription<()> = FrameDriver::new().subscription(|_| ());
        let _time: Subscription<()> = TimeDriver::new().subscription(|_| ());
    }

    #[test]
    fn segment_progress_normalizes_and_clamps() {
        assert_eq!(segment_progress(0.25, 0.0, 1.0), 0.25);
        assert_eq!(segment_progress(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(segment_progress(2.0, 0.0, 1.0), 1.0);
        assert_eq!(segment_progress(0.5, 0.5, 0.5), 0.0);
    }

    #[test]
    fn marquee_offset_wraps_without_drift() {
        let extent = 320.0;
        let speed = 120.0;

        let start = marquee_offset(Duration::from_secs(0), speed, extent);
        let one_cycle = marquee_offset(Duration::from_secs_f32(extent / speed), speed, extent);
        let long_run = marquee_offset(Duration::from_secs(3600), speed, extent);

        assert_eq!(start, 0.0);
        assert!((one_cycle - 0.0).abs() < 0.001);
        assert!(long_run >= 0.0 && long_run < extent);
    }

    #[test]
    fn scroll_sample_can_be_built_from_explicit_parts() {
        let sample = ScrollSample::new(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 80.0)),
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(300.0, 280.0)),
            AbsoluteOffset { x: 12.0, y: 42.0 },
            RelativeOffset { x: 0.1, y: 0.2 },
        );

        assert_eq!(sample.absolute.y, 42.0);
        assert_eq!(sample.relative.y, 0.2);
    }

    #[test]
    fn scroll_sample_segment_progress_uses_relative_scroll() {
        let sample = ScrollSample::new(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 80.0)),
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(300.0, 280.0)),
            AbsoluteOffset { x: 0.0, y: 42.0 },
            RelativeOffset { x: 0.0, y: 0.5 },
        );

        assert_eq!(sample.segment_progress(0.0, 1.0), 0.5);
        assert_eq!(sample.segment_progress(0.5, 0.5), 0.0);
    }
}
