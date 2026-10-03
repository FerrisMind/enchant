//! Bento grid layout — Magic UI `bento-grid` port for iced.
//!
//! Reference (MIT): `magicui/apps/www/registry/magicui/bento-grid.tsx`
//! https://github.com/magicuidesign/magicui/blob/main/apps/www/registry/magicui/bento-grid.tsx
//! License: MIT (Magic UI) — clean-room reimplementation for iced.
//!
//! CSS contract (desktop / iced default):
//! - grid: `w-full auto-rows-[22rem] grid-cols-3 gap-4`
//! - card: `relative flex flex-col justify-between overflow-hidden rounded-xl`
//! - hover: meta `-translate-y-10`, icon `scale-75`, CTA fades in at bottom,
//!   light overlay (`bg-black/3` / `dark:bg-neutral-800/10`) with
//!   `pointer-events-none` so the CTA stays clickable
//!
//! Reuses `iced-shadcn` decorative surfaces and link buttons (MIT OR Apache-2.0).

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::widget::{Space, column, container, mouse_area, row, stack, text};
use iced::{
    Alignment, Background, Color, Element, Event, Length, Padding, Rectangle, Shadow, Size, Vector,
};
use iced_shadcn::{
    ButtonProps, ButtonSize, ButtonVariant, DecorativeSurfaceProps, Theme, button_content,
    decorative_surface,
};

/// Magic UI `auto-rows-[22rem]` → 22 × 16px.
pub const BENTO_ROW_HEIGHT: f32 = 352.0;
/// Magic UI `gap-4`.
pub const BENTO_GAP: f32 = 16.0;
/// Magic UI default `grid-cols-3`.
pub const BENTO_COLUMNS: u16 = 3;
/// Magic UI `group-hover:-translate-y-10` (2.5rem).
const HOVER_LIFT: f32 = 40.0;
/// Magic UI icon base size `h-12 w-12`.
const ICON_SIZE: f32 = 48.0;
/// Magic UI `rounded-xl`: `0.625rem + 4px` in the pinned site theme.
const CARD_RADIUS: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BentoGridProps {
    pub columns: u16,
    pub gap: f32,
    /// Auto-row height (Magic UI `22rem`).
    pub row_height: f32,
}

impl Default for BentoGridProps {
    fn default() -> Self {
        Self {
            columns: BENTO_COLUMNS,
            gap: BENTO_GAP,
            row_height: BENTO_ROW_HEIGHT,
        }
    }
}

impl BentoGridProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn columns(mut self, columns: u16) -> Self {
        self.columns = columns.max(1);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    pub fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height.max(1.0);
        self
    }

    /// Total height for a card spanning `row_span` rows (including inter-row gaps).
    pub fn card_height(self, row_span: u16) -> f32 {
        let span = row_span.max(1) as f32;
        self.row_height * span + self.gap * (span - 1.0)
    }
}

/// Props for a single bento cell (Magic UI `BentoCard`).
pub struct BentoCardProps<'a, Message> {
    pub name: String,
    pub description: String,
    pub cta: String,
    /// Feature icon (e.g. Lucide glyph). Sized ~48px in the card.
    pub icon: Element<'a, Message>,
    pub col_span: u16,
    pub row_span: u16,
    /// Optional 1-based CSS-grid column start.
    pub column_start: Option<u16>,
    /// Optional 1-based CSS-grid row start.
    pub row_start: Option<u16>,
    pub background: Element<'a, Message>,
    pub on_cta: Option<Message>,
    /// Desktop hover state (Magic UI `group-hover`).
    pub hovered: bool,
    /// Animated hover amount in `0..=1`; `hovered(bool)` sets the endpoints.
    pub hover_progress: f32,
    pub on_hover_enter: Option<Message>,
    pub on_hover_exit: Option<Message>,
    /// When true, CTA stays visible without hover (Magic UI mobile `lg:hidden` path).
    /// Default false → desktop path: CTA only on hover.
    pub always_show_cta: bool,
}

impl<'a, Message> BentoCardProps<'a, Message> {
    pub fn new(
        name: impl Into<String>,
        icon: impl Into<Element<'a, Message>>,
        background: impl Into<Element<'a, Message>>,
    ) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            cta: String::new(),
            icon: icon.into(),
            col_span: 1,
            row_span: 1,
            column_start: None,
            row_start: None,
            background: background.into(),
            on_cta: None,
            hovered: false,
            hover_progress: 0.0,
            on_hover_enter: None,
            on_hover_exit: None,
            always_show_cta: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn cta(mut self, cta: impl Into<String>) -> Self {
        self.cta = cta.into();
        self
    }

    pub fn col_span(mut self, col_span: u16) -> Self {
        self.col_span = col_span.max(1);
        self
    }

    pub fn row_span(mut self, row_span: u16) -> Self {
        self.row_span = row_span.max(1);
        self
    }

    /// Set the 1-based CSS-grid column start.
    pub fn column_start(mut self, column: u16) -> Self {
        self.column_start = Some(column.max(1));
        self
    }

    /// Set the 1-based CSS-grid row start.
    pub fn row_start(mut self, row: u16) -> Self {
        self.row_start = Some(row.max(1));
        self
    }

    /// Set 1-based CSS-grid coordinates `(column, row)`.
    pub fn position(mut self, column: u16, row: u16) -> Self {
        self.column_start = Some(column.max(1));
        self.row_start = Some(row.max(1));
        self
    }

    pub fn on_cta(mut self, message: Message) -> Self {
        self.on_cta = Some(message);
        self
    }

    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self.hover_progress = if hovered { 1.0 } else { 0.0 };
        self
    }

    pub fn hover_progress(mut self, progress: f32) -> Self {
        self.hover_progress = progress.clamp(0.0, 1.0);
        self.hovered = self.hover_progress > 0.0;
        self
    }

    pub fn on_hover(mut self, enter: Message, exit: Message) -> Self {
        self.on_hover_enter = Some(enter);
        self.on_hover_exit = Some(exit);
        self
    }

    /// Keep CTA visible without hover (mobile parity path).
    pub fn always_show_cta(mut self, always: bool) -> Self {
        self.always_show_cta = always;
        self
    }
}

/// Magic UI `BentoGrid` with CSS-like column/row spans and optional explicit placement.
pub fn bento_grid<'a, Message: Clone + 'a>(
    cards: Vec<BentoCardProps<'a, Message>>,
    props: BentoGridProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let columns = props.columns.max(1);
    let mut occupied = Vec::new();
    let mut children = Vec::with_capacity(cards.len());

    for card in cards {
        let placement = place_grid_item(
            &mut occupied,
            columns,
            card.col_span,
            card.row_span,
            card.column_start,
            card.row_start,
        );
        children.push(BentoLayoutChild {
            placement,
            content: bento_card(card, props, theme),
        });
    }

    Element::new(BentoLayout {
        props,
        rows: occupied.len() as u16,
        children,
    })
}

/// Magic UI `BentoCard` — single feature cell with background + footer meta + hover CTA.
pub fn bento_card<'a, Message: Clone + 'a>(
    card: BentoCardProps<'a, Message>,
    props: BentoGridProps,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let height = props.card_height(card.row_span);
    let hover_progress = if card.hovered && card.hover_progress <= 0.0 {
        1.0
    } else {
        card.hover_progress.clamp(0.0, 1.0)
    };
    let hovered = hover_progress > 0.0;
    let dark = theme.variant().is_dark();
    let show_cta = !card.cta.is_empty() && (hovered || card.always_show_cta);

    let name_color = if dark {
        Color::from_rgb8(0xD4, 0xD4, 0xD8) // neutral-300
    } else {
        Color::from_rgb8(0x40, 0x40, 0x40) // neutral-700
    };
    let description_color = Color::from_rgb8(0xA3, 0xA3, 0xA3); // neutral-400
    let icon_color = if dark {
        Color::from_rgb8(0xD4, 0xD4, 0xD8)
    } else {
        Color::from_rgb8(0x40, 0x40, 0x40)
    };

    // CSS transforms do not change layout: keep a 48px box while the caller's
    // glyph can interpolate from 48px to 36px using the same hover progress.
    let icon = container(card.icon)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .style(move |_| container::Style {
            text_color: Some(icon_color),
            ..container::Style::default()
        });

    let meta = column![
        icon,
        text(card.name)
            .size(20)
            .line_height(text::LineHeight::Absolute(iced::Pixels(28.0)))
            .font(iced::Font {
                weight: iced::font::Weight::Semibold,
                ..iced::Font::DEFAULT
            })
            .style(move |_| text::Style {
                color: Some(name_color),
            }),
        text(card.description)
            .size(15)
            .line_height(text::LineHeight::Absolute(iced::Pixels(22.5)))
            .style(move |_| text::Style {
                color: Some(description_color),
            }),
    ]
    .spacing(4)
    .width(Length::Fill)
    .align_x(Alignment::Start);

    let cta: Element<'a, Message> = if !show_cta {
        Space::new().height(Length::Fixed(0.0)).into()
    } else {
        let label = row![
            text(card.cta.clone())
                .size(14)
                .line_height(text::LineHeight::Absolute(iced::Pixels(20.0)))
                .font(iced::Font {
                    weight: iced::font::Weight::Medium,
                    ..iced::Font::DEFAULT
                }),
            text(char::from(lucide_icons::Icon::ArrowRight).to_string())
                .font(iced::Font::with_name("lucide"))
                .size(16),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        if let Some(message) = card.on_cta.clone() {
            button_content(
                label,
                Some(message),
                ButtonProps::new()
                    .variant(ButtonVariant::Link)
                    .size(ButtonSize::Size1),
                theme,
            )
            .into()
        } else {
            // Non-interactive label when no handler (still useful for static demos).
            label.into()
        }
    };

    // The desktop metadata is bottom-aligned and translated upward by 40px on hover.
    let footer_pad_bottom = if card.always_show_cta {
        16.0
    } else {
        16.0 + HOVER_LIFT * hover_progress
    };
    let footer = container(meta)
        .width(Length::Fill)
        .padding(Padding {
            top: 16.0,
            right: 16.0,
            bottom: footer_pad_bottom,
            left: 16.0,
        })
        .align_y(Alignment::End);

    let background = container(card.background)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true);

    // Layer order mirrors Magic UI: background → overlay (pointer-events-none) → footer/CTA.
    // Overlay must sit *under* the footer so the link button remains clickable.
    let mut layers: Vec<Element<'a, Message>> = vec![background.into()];

    if hovered {
        let overlay = Color {
            a: if dark {
                0.10 * hover_progress
            } else {
                0.03 * hover_progress
            },
            ..if dark {
                Color::from_rgb8(0x26, 0x26, 0x26) // neutral-800-ish
            } else {
                Color::BLACK
            }
        };
        layers.push(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(overlay)),
                    ..container::Style::default()
                })
                .into(),
        );
    }

    layers.push(
        container(footer)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::End)
            .into(),
    );

    if show_cta {
        layers.push(
            container(cta)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(16)
                .align_y(Alignment::End)
                .into(),
        );
    }

    let shadow = if dark {
        // dark:[box-shadow:0_-20px_80px_-20px_#ffffff1f_inset] ≈ soft inset glow
        Shadow {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            offset: Vector::new(0.0, -4.0),
            blur_radius: 24.0,
        }
    } else {
        // 0 12px 24px rgba(0,0,0,.05) + hairline border
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 24.0,
        }
    };

    let border_color = if dark {
        Color::from_rgba(1.0, 1.0, 1.0, 0.10)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.03)
    };

    let surface = decorative_surface(
        stack(layers).width(Length::Fill).height(Length::Fill),
        Vec::new(),
        Vec::new(),
        DecorativeSurfaceProps::new()
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .clip(true)
            .radius(CARD_RADIUS)
            .background(theme.palette.background)
            .border_color(border_color)
            .border_width(1.0)
            .shadow(shadow),
        theme,
    );

    let mut body: Element<'a, Message> = surface;

    if card.on_hover_enter.is_some() || card.on_hover_exit.is_some() {
        let mut area = mouse_area(body);
        if let Some(enter) = card.on_hover_enter {
            area = area.on_enter(enter);
        }
        if let Some(exit) = card.on_hover_exit {
            area = area.on_exit(exit);
        }
        body = area.into();
    }

    body
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BentoPlacement {
    column: u16,
    row: u16,
    col_span: u16,
    row_span: u16,
}

fn place_grid_item(
    occupied: &mut Vec<Vec<bool>>,
    columns: u16,
    col_span: u16,
    row_span: u16,
    column_start: Option<u16>,
    row_start: Option<u16>,
) -> BentoPlacement {
    let columns = columns.max(1);
    let col_span = col_span.clamp(1, columns);
    let row_span = row_span.max(1);
    let max_column = columns - col_span;
    let preferred_column = column_start.map(|column| column.saturating_sub(1).min(max_column));
    let preferred_row = row_start.map(|row| row.saturating_sub(1));

    if let (Some(column), Some(row)) = (preferred_column, preferred_row) {
        let placement = BentoPlacement {
            column,
            row,
            col_span,
            row_span,
        };
        occupy(occupied, columns, placement);
        return placement;
    }

    let mut row = preferred_row.unwrap_or(0);
    loop {
        for column in 0..=max_column {
            if preferred_column.is_some_and(|preferred| preferred != column) {
                continue;
            }

            let placement = BentoPlacement {
                column,
                row,
                col_span,
                row_span,
            };
            if can_place(occupied, placement) {
                occupy(occupied, columns, placement);
                return placement;
            }
        }
        row = row.saturating_add(1);
    }
}

fn can_place(occupied: &[Vec<bool>], placement: BentoPlacement) -> bool {
    (placement.row..placement.row + placement.row_span).all(|row| {
        (placement.column..placement.column + placement.col_span).all(|column| {
            !occupied
                .get(row as usize)
                .and_then(|cells| cells.get(column as usize))
                .copied()
                .unwrap_or(false)
        })
    })
}

fn occupy(occupied: &mut Vec<Vec<bool>>, columns: u16, placement: BentoPlacement) {
    let required_rows = (placement.row + placement.row_span) as usize;
    occupied.resize_with(required_rows, || vec![false; columns as usize]);

    for row in placement.row..placement.row + placement.row_span {
        for column in placement.column..placement.column + placement.col_span {
            occupied[row as usize][column as usize] = true;
        }
    }
}

fn grid_height(props: BentoGridProps, rows: u16) -> f32 {
    if rows == 0 {
        0.0
    } else {
        props.card_height(rows)
    }
}

fn placement_bounds(
    available_width: f32,
    props: BentoGridProps,
    placement: BentoPlacement,
) -> (f32, f32, f32, f32) {
    let columns = props.columns.max(1) as f32;
    let total_gap = props.gap * (columns - 1.0);
    let track_width = ((available_width - total_gap) / columns).max(0.0);
    let x = placement.column as f32 * (track_width + props.gap);
    let y = placement.row as f32 * (props.row_height + props.gap);
    let width = track_width * placement.col_span as f32
        + props.gap * placement.col_span.saturating_sub(1) as f32;
    let height = props.card_height(placement.row_span);
    (x, y, width, height)
}

struct BentoLayoutChild<'a, Message> {
    placement: BentoPlacement,
    content: Element<'a, Message>,
}

/// Layout/event forwarding follows `iced::widget::Grid` (MIT).
/// Copyright 2019 Hector Ramon, Iced contributors.
struct BentoLayout<'a, Message> {
    props: BentoGridProps,
    rows: u16,
    children: Vec<BentoLayoutChild<'a, Message>>,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for BentoLayout<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children
            .iter()
            .map(|child| Tree::new(&child.content))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children_custom(
            &self.children,
            |tree, child| tree.diff(child.content.as_widget()),
            |child| Tree::new(&child.content),
        );
    }

    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fill,
            Length::Fixed(grid_height(self.props, self.rows)),
        )
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let available_width = limits.max().width;
        let height = grid_height(self.props, self.rows);
        let nodes = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(child, tree)| {
                let (x, y, width, height) =
                    placement_bounds(available_width, self.props, child.placement);
                let child_limits =
                    layout::Limits::new(Size::new(width, height), Size::new(width, height));
                child
                    .content
                    .as_widget_mut()
                    .layout(tree, renderer, &child_limits)
                    .move_to((x, y))
            })
            .collect();

        layout::Node::with_children(Size::new(available_width, height), nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .content
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.content.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> iced::mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .content
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(viewport) = layout.bounds().intersection(viewport) {
            for ((child, tree), layout) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .filter(|(_, layout)| layout.bounds().intersects(&viewport))
            {
                child
                    .content
                    .as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, &viewport);
            }
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        let children = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((child, state), layout)| {
                child.content.as_widget_mut().overlay(
                    state,
                    layout,
                    renderer,
                    viewport,
                    translation,
                )
            })
            .collect::<Vec<_>>();

        (!children.is_empty()).then(|| overlay::Group::with_children(children).overlay())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_props_match_magic_ui_grid() {
        let props = BentoGridProps::default();
        assert_eq!(props.columns, 3);
        assert!((props.gap - 16.0).abs() < f32::EPSILON);
        assert!((props.row_height - 352.0).abs() < f32::EPSILON);
    }

    #[test]
    fn card_height_includes_row_gaps() {
        let props = BentoGridProps::default();
        assert!((props.card_height(1) - 352.0).abs() < f32::EPSILON);
        assert!((props.card_height(2) - (352.0 * 2.0 + 16.0)).abs() < f32::EPSILON);
        assert!((props.card_height(3) - (352.0 * 3.0 + 16.0 * 2.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn col_span_clamped_to_at_least_one() {
        let col = BentoCardProps::<()>::new("x", text(""), Space::new()).col_span(0);
        assert_eq!(col.col_span, 1);
        let row = BentoCardProps::<()>::new("x", text(""), Space::new()).row_span(0);
        assert_eq!(row.row_span, 1);
    }

    #[test]
    fn grid_builder_clamps_columns_and_gap() {
        let props = BentoGridProps::new().columns(0).gap(-4.0).row_height(0.0);
        assert_eq!(props.columns, 1);
        assert!((props.gap - 0.0).abs() < f32::EPSILON);
        assert!((props.row_height - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn card_builder_clamps_hover_and_one_based_position() {
        let card = BentoCardProps::<()>::new("x", text(""), Space::new())
            .hover_progress(2.0)
            .position(0, 0);

        assert!((card.hover_progress - 1.0).abs() < f32::EPSILON);
        assert!(card.hovered);
        assert_eq!(card.column_start, Some(1));
        assert_eq!(card.row_start, Some(1));
    }

    #[test]
    fn auto_placement_honors_column_and_row_spans() {
        let mut occupied = Vec::new();
        let first = place_grid_item(&mut occupied, 3, 1, 1, None, None);
        let second = place_grid_item(&mut occupied, 3, 2, 1, None, None);
        let third = place_grid_item(&mut occupied, 3, 2, 1, None, None);
        let fourth = place_grid_item(&mut occupied, 3, 1, 1, None, None);

        assert_eq!((first.column, first.row), (0, 0));
        assert_eq!((second.column, second.row), (1, 0));
        assert_eq!((third.column, third.row), (0, 1));
        assert_eq!((fourth.column, fourth.row), (2, 1));
    }

    #[test]
    fn explicit_position_is_one_based_and_clamped_to_grid() {
        let mut occupied = Vec::new();
        let placement = place_grid_item(&mut occupied, 3, 2, 3, Some(3), Some(2));

        assert_eq!(placement.column, 1);
        assert_eq!(placement.row, 1);
        assert_eq!(occupied.len(), 4);
    }

    #[test]
    fn track_geometry_matches_browser_reference() {
        let props = BentoGridProps::default();
        let small = placement_bounds(
            694.0,
            props,
            BentoPlacement {
                column: 0,
                row: 0,
                col_span: 1,
                row_span: 1,
            },
        );
        let wide = placement_bounds(
            694.0,
            props,
            BentoPlacement {
                column: 1,
                row: 0,
                col_span: 2,
                row_span: 1,
            },
        );

        assert!((small.2 - 220.666_67).abs() < 0.001);
        assert!((wide.0 - 236.666_67).abs() < 0.001);
        assert!((wide.2 - 457.333_34).abs() < 0.001);
        assert!((wide.3 - 352.0).abs() < f32::EPSILON);
    }
}
