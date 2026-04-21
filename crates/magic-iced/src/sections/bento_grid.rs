use iced::Alignment;
use iced::Background;
use iced::Color;
use iced::Element;
use iced::Length;
use iced::widget::{column, container, row, text};
use iced_shadcn::{
    ButtonProps, ButtonSize, ButtonVariant, DecorativeSurfaceProps, Theme, button,
    decorative_surface,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BentoGridProps {
    pub columns: u16,
    pub gap: f32,
    pub card_height: f32,
}

impl Default for BentoGridProps {
    fn default() -> Self {
        Self {
            columns: 3,
            gap: 16.0,
            card_height: 240.0,
        }
    }
}

impl BentoGridProps {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct BentoCardProps<'a, Message> {
    pub name: String,
    pub description: String,
    pub cta: String,
    pub icon: String,
    pub col_span: u16,
    pub background: Element<'a, Message>,
    pub on_press: Option<Message>,
    pub accent: Option<Color>,
}

impl<'a, Message> BentoCardProps<'a, Message> {
    pub fn new(name: impl Into<String>, background: impl Into<Element<'a, Message>>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            cta: String::new(),
            icon: String::new(),
            col_span: 1,
            background: background.into(),
            on_press: None,
            accent: None,
        }
    }
}

pub fn bento_grid<'a, Message: Clone + 'a>(
    cards: Vec<BentoCardProps<'a, Message>>,
    props: BentoGridProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let columns = props.columns.max(1);
    let mut rows = Vec::new();
    let mut current = Vec::new();
    let mut used = 0u16;

    for card in cards {
        let span = card.col_span.clamp(1, columns);
        if used > 0 && used + span > columns {
            rows.push(render_row(std::mem::take(&mut current), props, theme));
            used = 0;
        }
        used += span;
        current.push((span, render_card(card, props, theme)));
    }

    if !current.is_empty() {
        rows.push(render_row(current, props, theme));
    }

    column(rows).spacing(props.gap).into()
}

fn render_row<'a, Message: Clone + 'a>(
    cards: Vec<(u16, Element<'a, Message>)>,
    props: BentoGridProps,
    _theme: &Theme,
) -> Element<'a, Message> {
    let children = cards
        .into_iter()
        .map(|(span, card)| container(card).width(Length::FillPortion(span)).into())
        .collect::<Vec<Element<'a, Message>>>();

    row(children)
        .spacing(props.gap)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

fn render_card<'a, Message: Clone + 'a>(
    card: BentoCardProps<'a, Message>,
    props: BentoGridProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let accent = card.accent.unwrap_or(theme.palette.primary);
    let cta: Element<'a, Message> = if card.cta.is_empty() {
        iced::widget::Space::new().height(Length::Fixed(0.0)).into()
    } else {
        button(
            card.cta.clone(),
            card.on_press.clone(),
            ButtonProps::new()
                .variant(ButtonVariant::Link)
                .size(ButtonSize::Size1),
            theme,
        )
        .into()
    };

    let preview_height = (props.card_height * 0.5).max(96.0);
    let preview = container(card.background)
        .width(Length::Fill)
        .height(Length::Fixed(preview_height))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(Color { a: 0.08, ..accent })),
            border: iced::Border {
                color: Color { a: 0.24, ..accent },
                width: 1.0,
                radius: theme.radius.md.into(),
            },
            ..iced::widget::container::Style::default()
        });

    let body = column![
        preview,
        text(card.icon).size(28),
        text(card.name).size(20),
        text(card.description).size(13).style({
            let color = theme.palette.muted_foreground;
            move |_| iced::widget::text::Style { color: Some(color) }
        }),
        cta
    ]
    .spacing(10)
    .width(Length::Fill)
    .align_x(Alignment::Start);

    let surface = decorative_surface(
        container(body).height(Length::Fill).padding([16.0, 16.0]),
        Vec::new(),
        Vec::new(),
        DecorativeSurfaceProps::new()
            .width(Length::Fill)
            .height(Length::Fixed(props.card_height))
            .clip(true)
            .radius(theme.radius.lg)
            .background(theme.palette.card)
            .border_color(theme.palette.border)
            .border_width(1.0),
        theme,
    );

    if let Some(message) = card.on_press {
        iced::widget::button(surface)
            .padding(0)
            .style(iced::widget::button::text)
            .on_press(message)
            .into()
    } else {
        surface
    }
}
