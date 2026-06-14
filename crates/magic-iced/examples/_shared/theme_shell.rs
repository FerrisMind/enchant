use iced::time::Instant;
use iced::widget::{column, container, row, stack, text, Space};
use iced::widget::Id;
use iced::{Alignment, Background, Color, Element, Length, Task};
use iced_shadcn::Theme;
use magic_iced::{
    anchor, measure_anchors, AnimatedThemeTogglerController, AnimatedThemeTogglerProps,
    animated_theme_toggler_with_controller, theme_transition_overlay, AnchorMeasurements,
};
use magic_iced::theme::MagicTheme;

pub const TOGGLER_ANCHOR: &str = "theme-toggler";

#[derive(Clone, Debug)]
pub struct ThemeShell {
    pub controller: AnimatedThemeTogglerController,
    pub props: AnimatedThemeTogglerProps,
    pub now: Instant,
    display_theme: Theme,
}

impl ThemeShell {
    pub fn new() -> Self {
        Self::with_mode(false)
    }

    pub fn new_dark() -> Self {
        Self::with_mode(true)
    }

    pub fn with_mode(dark_mode: bool) -> Self {
        let props = AnimatedThemeTogglerProps::new();
        let now = Instant::now();
        let controller = AnimatedThemeTogglerController::new(dark_mode, &props);
        let display_theme = controller.display_theme(now);
        Self {
            controller,
            props,
            now,
            display_theme,
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.display_theme
    }

    /// Smooth foreground during circle-reveal so labels do not pop at transition end.
    pub fn blended_foreground(&self, now: Instant) -> Color {
        self.blended_palette_color(now, |palette| palette.foreground)
    }

    /// Smooth border color during circle-reveal so grid lines do not flash on the new background.
    pub fn blended_border(&self, now: Instant) -> Color {
        let target_theme = theme_for_mode(self.controller.dark_mode());
        if !self.controller.is_transition_active(now) {
            return apparent_border_on_background(
                target_theme.palette.border,
                target_theme.palette.background,
            );
        }

        let source_theme = theme_for_mode(!self.controller.dark_mode());
        let reveal = self.controller.reveal_at(now);
        let from = apparent_border_on_background(
            source_theme.palette.border,
            source_theme.palette.background,
        );
        let to = apparent_border_on_background(
            target_theme.palette.border,
            target_theme.palette.background,
        );
        blend_colors(from, to, reveal)
    }

    fn blended_palette_color(
        &self,
        now: Instant,
        selector: impl Fn(&iced_shadcn::Palette) -> Color,
    ) -> Color {
        let target = selector(&theme_for_mode(self.controller.dark_mode()).palette);
        if !self.controller.is_transition_active(now) {
            return target;
        }

        let source = selector(&theme_for_mode(!self.controller.dark_mode()).palette);
        blend_colors(source, target, self.controller.reveal_at(now))
    }

    fn refresh_display_theme(&mut self) {
        self.display_theme = self.controller.display_theme(self.now);
    }

    pub fn tick(&mut self, now: Instant) {
        self.now = now;
        self.controller.tick(now, &self.props);
        self.refresh_display_theme();
    }

    pub fn apply_anchor_and_toggle(&mut self, measurements: &AnchorMeasurements) {
        if let Some(bounds) = measurements.get(0) {
            self.controller.set_origin(bounds.center);
        }
        self.controller.toggle(self.now, &self.props);
        self.refresh_display_theme();
    }

    pub fn measure_toggle_anchor() -> Task<AnchorMeasurements> {
        measure_anchors([Id::new(TOGGLER_ANCHOR)])
    }
}

/// Example page shell: title, body, theme toggler (top-right), circle reveal overlay.
pub fn themed_shell<'a, Message: Clone + 'a>(
    title: impl Into<String>,
    body: impl Into<Element<'a, Message>>,
    theme_shell: &ThemeShell,
    on_toggle: Message,
) -> Element<'a, Message> {
    let display_theme = theme_shell.theme().clone();
    let title = title.into();
    let body = body.into();

    let toggler = anchor(
        TOGGLER_ANCHOR,
        animated_theme_toggler_with_controller(
            &theme_shell.controller,
            theme_shell.now,
            on_toggle,
            theme_shell.props,
            &display_theme,
        ),
    );

    let header = row![
        text(title).size(28),
        Space::new().width(Length::Fill),
        toggler,
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let content = column![header, body]
        .spacing(20)
        .align_x(Alignment::Center)
        .width(Length::Fill);

    let background = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(display_theme.palette.background)),
            ..iced::widget::container::Style::default()
        });

    let page = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let mut layers = vec![background.into()];
    if let Some(overlay) =
        theme_transition_overlay(&theme_shell.controller, theme_shell.now, &theme_shell.props)
    {
        layers.push(overlay);
    }
    layers.push(page.into());

    container(stack(layers).width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn theme_for_mode(dark_mode: bool) -> Theme {
    if dark_mode {
        MagicTheme::dark()
    } else {
        MagicTheme::light()
    }
}

fn blend_colors(from: Color, to: Color, progress: f32) -> Color {
    let t = progress.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// Canvas strokes are opaque; resolve semi-transparent theme borders against their background.
fn apparent_border_on_background(border: Color, background: Color) -> Color {
    let a = border.a.clamp(0.0, 1.0);
    Color {
        r: border.r * a + background.r * (1.0 - a),
        g: border.g * a + background.g * (1.0 - a),
        b: border.b * a + background.b * (1.0 - a),
        a: 1.0,
    }
}
