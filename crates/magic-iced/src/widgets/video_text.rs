use crate::primitives::{EffectCanvasProps, EffectProgram, FrameSequence, effect_canvas};
use iced::time::Instant;
use iced::widget::{canvas, image};
use iced::{Color, Element, Length, Point, Rectangle, Renderer};

#[derive(Clone, Debug)]
pub enum VideoTextSource {
    Image(image::Handle),
    Frames(FrameSequence),
}

impl VideoTextSource {
    fn frame_at(&self, elapsed_ms: u64) -> Option<image::Handle> {
        match self {
            Self::Image(handle) => Some(handle.clone()),
            Self::Frames(sequence) => sequence.frame_at(elapsed_ms).cloned(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VideoTextState {
    started_at: Option<Instant>,
}

impl VideoTextState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, now: Instant) {
        let _ = self.started_at.get_or_insert(now);
    }

    fn elapsed_ms(&self, now: Instant) -> u64 {
        self.started_at
            .map(|started| now.saturating_duration_since(started).as_millis() as u64)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug)]
pub struct VideoTextProps {
    pub width: Length,
    pub height: Length,
    pub source: VideoTextSource,
    pub text: String,
    pub frame_duration_ms: u64,
    pub font_size: f32,
    pub font_weight: f32,
    pub font_family: String,
    pub text_color: Color,
}

impl VideoTextProps {
    pub fn new(source: VideoTextSource, text: impl Into<String>) -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(320.0),
            source,
            text: text.into(),
            frame_duration_ms: 180,
            font_size: 120.0,
            font_weight: 700.0,
            font_family: "Sans".into(),
            text_color: Color::WHITE,
        }
    }
}

pub fn video_text<'a, Message: 'a>(
    state: &VideoTextState,
    now: Instant,
    props: &'a VideoTextProps,
) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        VideoTextProgram {
            elapsed_ms: state.elapsed_ms(now),
            props,
        },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

struct VideoTextProgram<'a> {
    elapsed_ms: u64,
    props: &'a VideoTextProps,
}

impl<Message> EffectProgram<Message> for VideoTextProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let Some(handle) = self
            .props
            .source
            .frame_at(self.elapsed_ms.max(self.props.frame_duration_ms))
        else {
            return vec![frame.into_geometry()];
        };

        frame.draw_image(bounds, &handle);
        frame.fill(
            &canvas::Path::rectangle(Point::ORIGIN, bounds.size()),
            Color::from_rgba8(0, 0, 0, 0.35),
        );

        let graphemes = self.props.text.chars().collect::<Vec<_>>();
        let count = graphemes.len().max(1);
        let box_width = bounds.width / count as f32;
        let box_height = (self.props.font_size * 1.1).min(bounds.height * 0.82);
        let top = bounds.height * 0.5 - box_height * 0.5;

        for (index, grapheme) in graphemes.iter().enumerate() {
            let rect = Rectangle::new(
                Point::new(index as f32 * box_width, top),
                iced::Size::new(box_width, box_height),
            );

            frame.with_clip(rect, |frame| {
                frame.draw_image(bounds, &handle);
            });

            frame.stroke(
                &canvas::Path::rounded_rectangle(
                    rect.position(),
                    rect.size(),
                    iced::border::Radius::from(box_width.min(box_height) * 0.18),
                ),
                canvas::Stroke::default().with_width(1.0).with_color(Color {
                    a: 0.2,
                    ..self.props.text_color
                }),
            );

            frame.fill_text(canvas::Text {
                content: grapheme.to_string(),
                position: Point::new(rect.x + rect.width * 0.5, bounds.height * 0.5),
                color: Color {
                    a: 0.26,
                    ..self.props.text_color
                },
                size: iced::Pixels(self.props.font_size),
                line_height: iced::widget::text::LineHeight::Relative(
                    self.props.font_weight / 700.0,
                ),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }

        frame.fill_text(canvas::Text {
            content: self.props.text.clone(),
            position: Point::new(bounds.width * 0.5, bounds.height * 0.5),
            color: Color {
                a: 0.12,
                ..self.props.text_color
            },
            size: iced::Pixels(self.props.font_size),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(color: [u8; 4]) -> image::Handle {
        image::Handle::from_rgba(1, 1, color.to_vec())
    }

    #[test]
    fn video_text_source_frames_cycle() {
        let source = VideoTextSource::Frames(FrameSequence::new(
            vec![handle([255, 0, 0, 255]), handle([0, 255, 0, 255])],
            100,
        ));

        assert!(source.frame_at(0).is_some());
        assert!(source.frame_at(100).is_some());
    }

    #[test]
    fn video_text_state_starts_clock() {
        let mut state = VideoTextState::new();
        state.tick(Instant::now());

        assert!(state.started_at.is_some());
    }
}
