mod _shared;

use iced::time::Instant;

use iced::advanced::Clipboard;
use iced::advanced::Renderer as _;
use iced::advanced::Shell;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::gradient;
use iced::widget::{column, container, image, mouse_area, row, text};
use iced::{
    Alignment, Border, Color, Element, Event, Length, Rectangle, Renderer, Size, Subscription, Task,
};
use iced_shadcn::{CardProps, CardSize, DecorativeSurfaceProps, Theme, card, decorative_surface};
use enchant_iced::{MagicTheme, MarqueeItem, MarqueeOrientation, MarqueeProps, marquee_indexed};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;
use twill::backends::iced::styled_container_with_semantic_theme;
use twill::prelude::{BackgroundColor, SemanticColor, SemanticThemeVars, Style as TwillStyle};

static REVIEWS: LazyLock<Vec<MarqueeItem>> = LazyLock::new(reviews);
const PREVIEW_WIDTH: f32 = 780.0;
const PREVIEW_PADDING: f32 = 24.0;
const REVIEW_CARD_WIDTH: f32 = 256.0;
const REVIEW_CARD_GAP: f32 = 16.0;
const REVIEW_ROW_PADDING: f32 = 8.0;
const REVIEW_ROW_CROSS: f32 = 112.0;
const FADE_WIDTH_RATIO: f32 = 0.25;

pub fn main() -> iced::Result {
    iced::application(MarqueeDemo::new, MarqueeDemo::update, MarqueeDemo::view)
        .title("Marquee Example")
        .window(iced::window::Settings {
            size: Size::new(980.0, 620.0),
            ..Default::default()
        })
        .subscription(MarqueeDemo::subscription)
        .run()
}

struct MarqueeDemo {
    theme_shell: _shared::ThemeShell,
    started_at: Instant,
    elapsed: [Duration; 2],
    paused_total: [Duration; 2],
    pause_started_at: [Option<Instant>; 2],
    hover_depth: [usize; 2],
    hovered_card: [Option<(usize, usize)>; 2],
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    Hover(usize, bool),
    CardHover(usize, Option<(usize, usize)>),
}

impl Default for MarqueeDemo {
    fn default() -> Self {
        Self {
            theme_shell: _shared::ThemeShell::new_dark(),
            started_at: Instant::now(),
            elapsed: [Duration::ZERO; 2],
            paused_total: [Duration::ZERO; 2],
            pause_started_at: [None, None],
            hover_depth: [0; 2],
            hovered_card: [None, None],
        }
    }
}

impl MarqueeDemo {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }

            Message::Tick(now) => self.tick_at(now),
            Message::Hover(row, entered) => self.set_hover(row, entered, Instant::now()),
            Message::CardHover(row, card) => self.set_card_hover(row, card),
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::window::frames().map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let first_row = &REVIEWS[..3];
        let second_row = &REVIEWS[3..];
        let rows = column![
            self.marquee_row(first_row, false, 0),
            self.marquee_row(second_row, true, 1),
        ]
        .spacing(0)
        .width(Length::Fill)
        .align_x(Alignment::Center);

        let preview = decorative_surface(
            HorizontalFadeMask::new(
                rows.into(),
                self.theme_shell.theme().palette.background,
                FADE_WIDTH_RATIO,
            ),
            Vec::new(),
            Vec::new(),
            DecorativeSurfaceProps::new()
                .width(Length::Fixed(PREVIEW_WIDTH))
                .padding(PREVIEW_PADDING)
                .radius(16.0)
                .clip(true)
                .background(self.theme_shell.theme().palette.background)
                .border_color(self.theme_shell.theme().palette.border)
                .border_width(1.0),
            self.theme_shell.theme(),
        );

        let body = column![
            container(preview)
                .width(Length::Fill)
                .center_x(Length::Fill)
        ]
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .padding([40.0, 0.0]);

        _shared::themed_shell("Marquee", body, &self.theme_shell, Message::ToggleTheme)
    }

    fn marquee_row<'a>(
        &'a self,
        items: &'a [MarqueeItem],
        reverse: bool,
        row_index: usize,
    ) -> Element<'a, Message> {
        let title_color = self.theme_shell.theme().palette.foreground;
        let subtitle_color = Color {
            a: 0.40,
            ..self.theme_shell.theme().palette.foreground
        };
        let body_color = Color {
            a: 0.82,
            ..self.theme_shell.theme().palette.foreground
        };
        let card_background = Color {
            a: 0.10,
            ..self.theme_shell.theme().palette.foreground
        };
        let card_border = Color {
            a: 0.14,
            ..self.theme_shell.theme().palette.foreground
        };
        let hovered_background = Color {
            a: 0.15,
            ..self.theme_shell.theme().palette.foreground
        };
        let hovered_border = Color {
            a: 0.20,
            ..self.theme_shell.theme().palette.foreground
        };
        let hovered_card = self.hovered_card[row_index];

        let render_item =
            |item: &MarqueeItem, copy_index: usize, item_index: usize| -> Element<'a, Message> {
                let is_hovered = hovered_card == Some((copy_index, item_index));
                let avatar = container(image(avatar_handle(item.title.as_str())))
                    .width(Length::Fixed(32.0))
                    .height(Length::Fixed(32.0))
                    .clip(true)
                    .style(|_| iced::widget::container::Style {
                        border: Border {
                            color: Color::TRANSPARENT,
                            width: 0.0,
                            radius: 16.0.into(),
                        },
                        ..iced::widget::container::Style::default()
                    });

                container(
                    mouse_area(
                        card(
                            column![
                                row![
                                    avatar,
                                    column![
                                        text(item.title.clone()).size(14).style(move |_| {
                                            iced::widget::text::Style {
                                                color: Some(title_color),
                                            }
                                        }),
                                        text(item.subtitle.clone()).size(12).style(move |_| {
                                            iced::widget::text::Style {
                                                color: Some(subtitle_color),
                                            }
                                        }),
                                    ]
                                    .spacing(2)
                                    .width(Length::Fill),
                                ]
                                .spacing(10)
                                .align_y(Alignment::Center)
                                .width(Length::Fill),
                                text(item.body.clone()).size(14).width(Length::Fill).style(
                                    move |_| iced::widget::text::Style {
                                        color: Some(body_color),
                                    }
                                ),
                            ]
                            .spacing(8)
                            .width(Length::Fill),
                            review_card_props(
                                self.theme_shell.theme(),
                                is_hovered,
                                card_background,
                                card_border,
                                hovered_background,
                                hovered_border,
                            ),
                            self.theme_shell.theme(),
                        )
                        .width(Length::Fill)
                        .height(Length::Fill),
                    )
                    .on_enter(Message::CardHover(
                        row_index,
                        Some((copy_index, item_index)),
                    ))
                    .on_exit(Message::CardHover(row_index, None)),
                )
                .width(Length::Fixed(REVIEW_CARD_WIDTH))
                .height(Length::Fill)
                .into()
            };

        marquee_indexed(
            items,
            render_item,
            self.elapsed[row_index],
            marquee_props(reverse),
            self.theme_shell.theme(),
            move |entered| Message::Hover(row_index, entered),
        )
    }

    fn tick_at(&mut self, now: Instant) {
        self.theme_shell.tick(now);
        for row in 0..self.elapsed.len() {
            self.elapsed[row] = if self.hover_depth[row] > 0 {
                self.pause_started_at[row]
                    .map(|pause_started_at| {
                        pause_started_at
                            .saturating_duration_since(self.started_at)
                            .saturating_sub(self.paused_total[row])
                    })
                    .unwrap_or(self.elapsed[row])
            } else {
                now.saturating_duration_since(self.started_at)
                    .saturating_sub(self.paused_total[row])
            };
        }
    }

    fn set_hover(&mut self, row: usize, entered: bool, now: Instant) {
        if row >= self.elapsed.len() {
            return;
        }

        if entered {
            self.hover_depth[row] = self.hover_depth[row].saturating_add(1);
            if self.hover_depth[row] == 1 {
                self.pause_started_at[row] = Some(now);
                self.elapsed[row] = now
                    .saturating_duration_since(self.started_at)
                    .saturating_sub(self.paused_total[row]);
            }
        } else if self.hover_depth[row] > 0 {
            self.hover_depth[row] -= 1;
            if self.hover_depth[row] == 0
                && let Some(pause_started_at) = self.pause_started_at[row].take()
            {
                self.paused_total[row] += now.saturating_duration_since(pause_started_at);
            }
        }
    }

    fn set_card_hover(&mut self, row: usize, card: Option<(usize, usize)>) {
        if row >= self.hovered_card.len() {
            return;
        }

        self.hovered_card[row] = card;
    }
}

fn marquee_props(reverse: bool) -> MarqueeProps {
    MarqueeProps {
        orientation: MarqueeOrientation::Horizontal,
        reverse,
        pause_on_hover: true,
        repeat: 4,
        gap: REVIEW_CARD_GAP,
        speed_px_per_sec: 40.8,
        item_extent: REVIEW_CARD_WIDTH,
        item_cross: REVIEW_ROW_CROSS,
        padding: REVIEW_ROW_PADDING,
        edge_fade: false,
    }
}

fn page_shell_style() -> TwillStyle {
    TwillStyle::new().background_token(BackgroundColor::semantic(SemanticColor::Background))
}

fn review_card_props(
    theme: &Theme,
    is_hovered: bool,
    card_background: Color,
    card_border: Color,
    hovered_background: Color,
    hovered_border: Color,
) -> CardProps {
    CardProps::new()
        .size(CardSize::Size2)
        .show_shadow(false)
        .padding(16.0)
        .radius(12.0)
        .background(if is_hovered {
            hovered_background
        } else {
            card_background
        })
        .border_color(if is_hovered {
            hovered_border
        } else {
            card_border
        })
        .text_color(theme.palette.foreground)
}

struct HorizontalFadeMask<'a, Message> {
    content: Element<'a, Message>,
    background: Color,
    fade_width_ratio: f32,
}

impl<'a, Message> HorizontalFadeMask<'a, Message> {
    fn new(content: Element<'a, Message>, background: Color, fade_width_ratio: f32) -> Self {
        Self {
            content,
            background,
            fade_width_ratio,
        }
    }
}

impl<Message> Widget<Message, iced::Theme, Renderer> for HorizontalFadeMask<'_, Message>
where
    Message: Clone,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.content.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let Some(clipped_viewport) = bounds.intersection(viewport) else {
            return;
        };

        let fade_width = (bounds.width * self.fade_width_ratio).max(1.0);

        renderer.with_layer(clipped_viewport, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                &clipped_viewport,
            );
        });

        let quad = |bounds| renderer::Quad {
            bounds,
            border: Border::default(),
            shadow: iced::Shadow::default(),
            snap: true,
        };

        renderer.fill_quad(
            quad(Rectangle {
                x: bounds.x,
                y: bounds.y,
                width: fade_width,
                height: bounds.height,
            }),
            gradient::Linear::new(iced::Radians(0.0))
                .add_stop(0.0, self.background)
                .add_stop(
                    0.18,
                    Color {
                        a: 0.95,
                        ..self.background
                    },
                )
                .add_stop(
                    0.55,
                    Color {
                        a: 0.55,
                        ..self.background
                    },
                )
                .add_stop(
                    1.0,
                    Color {
                        a: 0.0,
                        ..self.background
                    },
                ),
        );

        renderer.fill_quad(
            quad(Rectangle {
                x: bounds.x + bounds.width - fade_width,
                y: bounds.y,
                width: fade_width,
                height: bounds.height,
            }),
            gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                .add_stop(0.0, self.background)
                .add_stop(
                    0.18,
                    Color {
                        a: 0.95,
                        ..self.background
                    },
                )
                .add_stop(
                    0.55,
                    Color {
                        a: 0.55,
                        ..self.background
                    },
                )
                .add_stop(
                    1.0,
                    Color {
                        a: 0.0,
                        ..self.background
                    },
                ),
        );
    }
}

impl<'a, Message> From<HorizontalFadeMask<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(widget: HorizontalFadeMask<'a, Message>) -> Self {
        Element::new(widget)
    }
}

fn reviews() -> Vec<MarqueeItem> {
    vec![
        MarqueeItem::new("Jack")
            .subtitle("@jack")
            .body("I've never seen anything like this before. It's amazing. I love it."),
        MarqueeItem::new("Jill")
            .subtitle("@jill")
            .body("I don't know what to say. I'm speechless. This is amazing."),
        MarqueeItem::new("John")
            .subtitle("@john")
            .body("I'm at a loss for words. This is amazing. I love it."),
        MarqueeItem::new("Jane")
            .subtitle("@jane")
            .body("I'm at a loss for words. This is amazing. I love it."),
        MarqueeItem::new("Jenny")
            .subtitle("@jenny")
            .body("I'm at a loss for words. This is amazing. I love it."),
        MarqueeItem::new("James")
            .subtitle("@james")
            .body("I'm at a loss for words. This is amazing. I love it."),
    ]
}

fn avatar_handle(name: &str) -> iced::widget::image::Handle {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path.push("assets");
    path.push("marquee");
    path.push(format!("{}.png", name.to_lowercase()));
    iced::widget::image::Handle::from_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marquee_demo_uses_reference_reviews() {
        let reviews = reviews();
        let expected = [
            (
                "Jack",
                "@jack",
                "I've never seen anything like this before. It's amazing. I love it.",
            ),
            (
                "Jill",
                "@jill",
                "I don't know what to say. I'm speechless. This is amazing.",
            ),
            (
                "John",
                "@john",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
            (
                "Jane",
                "@jane",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
            (
                "Jenny",
                "@jenny",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
            (
                "James",
                "@james",
                "I'm at a loss for words. This is amazing. I love it.",
            ),
        ];

        assert_eq!(reviews.len(), expected.len());
        for (review, (name, username, body)) in reviews.iter().zip(expected.iter()) {
            assert_eq!(review.title, *name);
            assert_eq!(review.subtitle, *username);
            assert_eq!(review.body, *body);
        }
    }

    #[test]
    fn marquee_demo_uses_two_rows_like_reference() {
        let reviews = reviews();
        assert_eq!(reviews[..3].len(), 3);
        assert_eq!(reviews[3..].len(), 3);
    }

    #[test]
    fn hover_pauses_only_target_row_clock() {
        let mut demo = MarqueeDemo::default();
        let start = Instant::now();
        demo.started_at = start;

        demo.tick_at(start + Duration::from_secs(5));
        assert_eq!(demo.elapsed[0], Duration::from_secs(5));
        assert_eq!(demo.elapsed[1], Duration::from_secs(5));

        demo.set_hover(0, true, start + Duration::from_secs(5));
        demo.tick_at(start + Duration::from_secs(7));
        assert_eq!(demo.elapsed[0], Duration::from_secs(5));
        assert_eq!(demo.elapsed[1], Duration::from_secs(7));

        demo.set_hover(0, false, start + Duration::from_secs(7));
        demo.tick_at(start + Duration::from_secs(9));
        assert_eq!(demo.elapsed[0], Duration::from_secs(7));
        assert_eq!(demo.elapsed[1], Duration::from_secs(9));
    }

    #[test]
    fn card_hover_tracks_per_row() {
        let mut demo = MarqueeDemo::default();

        demo.set_card_hover(1, Some((3, 2)));
        assert_eq!(demo.hovered_card[0], None);
        assert_eq!(demo.hovered_card[1], Some((3, 2)));

        demo.set_card_hover(1, None);
        assert_eq!(demo.hovered_card[1], None);
    }

    #[test]
    fn page_shell_uses_twill_semantic_background() {
        assert_eq!(
            page_shell_style().background_color_value(),
            Some(BackgroundColor::semantic(SemanticColor::Background))
        );
    }
}
