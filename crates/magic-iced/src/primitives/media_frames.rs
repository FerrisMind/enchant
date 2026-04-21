use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Element, Length, Padding};
use iced_shadcn::tokens::{AccentColor, accent_color, accent_low, mix};
use iced_shadcn::{
    AspectRatioProps, DecorativeSurfaceProps, Theme, aspect_ratio, decorative_surface,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MockupFrameKind {
    Safari,
    Iphone,
    Android,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MockupFrameMetrics {
    pub width: f32,
    pub height: f32,
    pub inset_left: f32,
    pub inset_top: f32,
    pub inset_right: f32,
    pub inset_bottom: f32,
    pub screen_radius: f32,
    pub chrome_height: f32,
}

impl MockupFrameMetrics {
    pub fn aspect_ratio(self) -> f32 {
        self.width / self.height
    }

    pub fn slot_padding(self) -> Padding {
        Padding {
            left: self.inset_left,
            top: self.inset_top,
            right: self.inset_right,
            bottom: self.inset_bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridCell {
    pub row: u16,
    pub col: u16,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MockupFrameProps {
    pub chrome_label: Option<String>,
}

impl MockupFrameProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chrome_label(mut self, chrome_label: impl Into<String>) -> Self {
        self.chrome_label = Some(chrome_label.into());
        self
    }
}

pub fn mockup_metrics(kind: MockupFrameKind) -> MockupFrameMetrics {
    match kind {
        MockupFrameKind::Safari => MockupFrameMetrics {
            width: 1203.0,
            height: 753.0,
            inset_left: 1.0,
            inset_top: 52.0,
            inset_right: 2.0,
            inset_bottom: 1.0,
            screen_radius: 11.0,
            chrome_height: 52.0,
        },
        MockupFrameKind::Iphone => MockupFrameMetrics {
            width: 433.0,
            height: 882.0,
            inset_left: 21.25,
            inset_top: 19.25,
            inset_right: 22.25,
            inset_bottom: 19.25,
            screen_radius: 55.75,
            chrome_height: 74.0,
        },
        MockupFrameKind::Android => MockupFrameMetrics {
            width: 433.0,
            height: 882.0,
            inset_left: 9.0,
            inset_top: 14.0,
            inset_right: 64.0,
            inset_bottom: 68.0,
            screen_radius: 33.0,
            chrome_height: 48.0,
        },
    }
}

pub fn mockup_frame<'a, Message: Clone + 'a + 'static>(
    kind: MockupFrameKind,
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Element<'a, Message> {
    mockup_frame_with_props(kind, content, &MockupFrameProps::new(), theme)
}

pub fn mockup_frame_with_props<'a, Message: Clone + 'a + 'static>(
    kind: MockupFrameKind,
    content: impl Into<Element<'a, Message>>,
    props: &MockupFrameProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let metrics = mockup_metrics(kind);
    let content = content.into();
    let chrome_label = props
        .chrome_label
        .clone()
        .unwrap_or_else(|| "Preview".to_string());
    let slot = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true);
    let slot = container(slot)
        .padding(metrics.slot_padding())
        .width(Length::Fill)
        .height(Length::Fill);
    let chrome_text_color = theme.palette.muted_foreground;

    let frame_body: Element<'a, Message> = match kind {
        MockupFrameKind::Safari => {
            let chrome = row![
                traffic_light(light_color(theme, 0.0)),
                traffic_light(light_color(theme, 0.2)),
                traffic_light(light_color(theme, 0.4)),
                space().width(Length::Fill),
                text(chrome_label)
                    .size(12)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(chrome_text_color),
                    })
            ]
            .align_y(Alignment::Center)
            .spacing(8)
            .padding(Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: 16.0,
            });

            column![container(chrome).height(metrics.chrome_height), slot]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        MockupFrameKind::Iphone => {
            let notch = container(
                space()
                    .width(Length::Fixed(110.0))
                    .height(Length::Fixed(18.0)),
            )
            .center_x(Length::Fill)
            .padding([16.0, 0.0]);
            column![notch, slot]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        MockupFrameKind::Android => {
            let camera = container(
                space()
                    .width(Length::Fixed(18.0))
                    .height(Length::Fixed(18.0)),
            )
            .center_x(Length::Fill)
            .padding([10.0, 0.0]);
            column![camera, slot]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    };

    let props = DecorativeSurfaceProps::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .radius(metrics.screen_radius)
        .background(theme.palette.card)
        .border_color(theme.palette.border)
        .border_width(1.0);

    aspect_ratio(
        decorative_surface(frame_body, Vec::new(), Vec::new(), props, theme),
        AspectRatioProps::new().ratio(metrics.aspect_ratio()),
    )
    .into()
}

pub fn image_grid_cells(rows: u16, cols: u16) -> Box<[GridCell]> {
    if rows == 0 || cols == 0 {
        return Box::new([]);
    }

    let width = 1.0 / cols as f32;
    let height = 1.0 / rows as f32;
    let mut cells = Vec::with_capacity(rows as usize * cols as usize);

    for row in 0..rows {
        for col in 0..cols {
            cells.push(GridCell {
                row,
                col,
                left: col as f32 * width,
                top: row as f32 * height,
                width,
                height,
            });
        }
    }

    cells.into_boxed_slice()
}

fn light_color(theme: &Theme, blend: f32) -> iced::Color {
    if blend == 0.0 {
        accent_color(&theme.palette, AccentColor::Red)
    } else {
        let base = accent_low(&theme.palette, AccentColor::Gray);
        mix(
            base,
            accent_color(&theme.palette, AccentColor::Amber),
            blend,
        )
    }
}

fn traffic_light<Message: 'static>(color: iced::Color) -> Element<'static, Message> {
    container(
        space()
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0)),
    )
    .style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(color)),
        border: iced::Border {
            radius: 9999.0.into(),
            width: 0.0,
            color: iced::Color::TRANSPARENT,
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mockup_metrics_match_phase0_reference_geometry() {
        let safari = mockup_metrics(MockupFrameKind::Safari);
        let iphone = mockup_metrics(MockupFrameKind::Iphone);
        let android = mockup_metrics(MockupFrameKind::Android);

        assert_eq!(safari.inset_top, 52.0);
        assert_eq!(safari.screen_radius, 11.0);
        assert_eq!(iphone.screen_radius, 55.75);
        assert_eq!(android.inset_left, 9.0);
        assert_eq!(android.screen_radius, 33.0);
    }

    #[test]
    fn image_grid_cells_are_deterministic_and_cover_grid() {
        let cells = image_grid_cells(2, 3);

        assert_eq!(cells.len(), 6);
        assert_eq!(
            cells[0],
            GridCell {
                row: 0,
                col: 0,
                left: 0.0,
                top: 0.0,
                width: 1.0 / 3.0,
                height: 0.5,
            }
        );
        assert_eq!(cells[5].row, 1);
        assert_eq!(cells[5].col, 2);
        assert!((cells[5].left - (2.0 / 3.0)).abs() < f32::EPSILON);
        assert!((cells[5].top - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn mockup_frame_props_are_generic_and_configurable() {
        let props = MockupFrameProps::new();
        let configured = MockupFrameProps::new().chrome_label("workspace.local");

        assert_eq!(props.chrome_label, None);
        assert_eq!(configured.chrome_label.as_deref(), Some("workspace.local"));
    }
}
