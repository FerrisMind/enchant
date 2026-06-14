use iced::time::Instant;
use iced::widget::Id;
use iced::widget::{Space, column, container, row, stack, text};
use iced::{Alignment, Background, Color, Element, Length, Task};
use iced_shadcn::{Palette, Theme};
use magic_iced::theme::MagicTheme;
use magic_iced::{
    AnchorMeasurements, AnimatedThemeTogglerController, AnimatedThemeTogglerProps, GlobeDemoColors,
    anchor, animated_theme_toggler_with_controller, globe_demo_colors_blended, measure_anchors,
    theme_transition_overlay,
};

pub const TOGGLER_ANCHOR: &str = "theme-toggler";

#[derive(Clone, Debug)]
pub struct ThemeShell {
    pub controller: AnimatedThemeTogglerController,
    pub props: AnimatedThemeTogglerProps,
    pub now: Instant,
    display_theme: Theme,
    blended_theme: Theme,
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
        let blended_theme = theme_for_mode(dark_mode);
        Self {
            controller,
            props,
            now,
            display_theme,
            blended_theme,
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.display_theme
    }

    /// Theme with palette colors interpolated during circle-reveal.
    pub fn blended_theme(&self) -> &Theme {
        &self.blended_theme
    }

    /// Full theme with palette colors interpolated during circle-reveal.
    pub fn blended_theme_at(&self, now: Instant) -> Theme {
        let target = theme_for_mode(self.controller.dark_mode());
        if !self.controller.is_transition_active(now) {
            return target;
        }

        let source = theme_for_mode(!self.controller.dark_mode());
        let reveal = self.controller.reveal_at(now);
        let mut theme = target.clone();
        theme.palette = blend_palette(source.palette, target.palette, reveal);
        theme
    }

    /// Smooth foreground during circle-reveal so labels do not pop at transition end.
    pub fn blended_foreground(&self, now: Instant) -> Color {
        self.blended_palette_color(now, |palette| palette.foreground)
    }

    /// Smooth primary color during circle-reveal so accent widgets stay in sync.
    pub fn blended_primary(&self, now: Instant) -> Color {
        self.blended_palette_color(now, |palette| palette.primary)
    }

    /// Line-shadow stripe color — same reveal progress as `blended_foreground`.
    pub fn blended_line_shadow(&self, now: Instant) -> Color {
        let target_dark = self.controller.dark_mode();
        let target = magic_iced::line_shadow_color_for_mode(target_dark);
        if !self.controller.is_transition_active(now) {
            return target;
        }

        let source = magic_iced::line_shadow_color_for_mode(!target_dark);
        blend_colors(source, target, self.controller.reveal_at(now))
    }

    /// Shiny button label color — same reveal progress as `blended_foreground`.
    pub fn blended_shiny_text_color(&self, now: Instant) -> Color {
        let target_dark = self.controller.dark_mode();
        let target = magic_iced::shiny_text_color_for_mode(target_dark);
        if !self.controller.is_transition_active(now) {
            return target;
        }

        let source = magic_iced::shiny_text_color_for_mode(!target_dark);
        blend_colors(source, target, self.controller.reveal_at(now))
    }

    /// Primary accent for shiny widgets — same reveal progress as `blended_primary`.
    pub fn blended_shiny_primary(&self, now: Instant) -> Color {
        self.blended_primary(now)
    }

    /// Background fill for shiny widgets — same reveal progress as page foreground blend.
    pub fn blended_shiny_background(&self, now: Instant) -> Color {
        self.blended_palette_color(now, |palette| palette.background)
    }

    /// Border for shiny widgets — same reveal progress as `blended_border`.
    pub fn blended_shiny_border(&self, now: Instant) -> Color {
        self.blended_border(now)
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

    /// Globe demo shell colors — blended in sync with the page reveal (globe canvas stays fixed).
    pub fn blended_globe_demo_colors(&self, now: Instant) -> GlobeDemoColors {
        globe_demo_colors_blended(&self.controller, now, theme_for_mode)
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

    fn refresh_themes(&mut self) {
        self.display_theme = self.controller.display_theme(self.now);
        self.blended_theme = self.blended_theme_at(self.now);
    }

    pub fn tick(&mut self, now: Instant) {
        self.now = now;
        self.controller.tick(now, &self.props);
        self.refresh_themes();
    }

    pub fn apply_anchor_and_toggle(&mut self, measurements: &AnchorMeasurements) {
        if let Some(bounds) = measurements.get(0) {
            self.controller.set_origin(bounds.center);
        }
        self.controller.toggle(self.now, &self.props);
        self.refresh_themes();
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
    let blended_theme = theme_shell.blended_theme().clone();
    let title_color = theme_shell.blended_foreground(theme_shell.now);
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
        text(title).size(28).style({
            let color = title_color;
            move |_| iced::widget::text::Style { color: Some(color) }
        }),
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
            background: Some(Background::Color(blended_theme.palette.background)),
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

fn blend_palette(from: Palette, to: Palette, progress: f32) -> Palette {
    let blend = |from: Color, to: Color| blend_colors(from, to, progress);
    Palette {
        background: blend(from.background, to.background),
        foreground: blend(from.foreground, to.foreground),
        card: blend(from.card, to.card),
        card_foreground: blend(from.card_foreground, to.card_foreground),
        popover: blend(from.popover, to.popover),
        popover_foreground: blend(from.popover_foreground, to.popover_foreground),
        border: blend(from.border, to.border),
        input: blend(from.input, to.input),
        ring: blend(from.ring, to.ring),
        primary: blend(from.primary, to.primary),
        primary_foreground: blend(from.primary_foreground, to.primary_foreground),
        secondary: blend(from.secondary, to.secondary),
        secondary_foreground: blend(from.secondary_foreground, to.secondary_foreground),
        accent: blend(from.accent, to.accent),
        accent_foreground: blend(from.accent_foreground, to.accent_foreground),
        muted: blend(from.muted, to.muted),
        muted_foreground: blend(from.muted_foreground, to.muted_foreground),
        destructive: blend(from.destructive, to.destructive),
        destructive_foreground: blend(from.destructive_foreground, to.destructive_foreground),
        chart_1: blend(from.chart_1, to.chart_1),
        chart_2: blend(from.chart_2, to.chart_2),
        chart_3: blend(from.chart_3, to.chart_3),
        chart_4: blend(from.chart_4, to.chart_4),
        chart_5: blend(from.chart_5, to.chart_5),
        sidebar: blend(from.sidebar, to.sidebar),
        sidebar_foreground: blend(from.sidebar_foreground, to.sidebar_foreground),
        sidebar_primary: blend(from.sidebar_primary, to.sidebar_primary),
        sidebar_primary_foreground: blend(
            from.sidebar_primary_foreground,
            to.sidebar_primary_foreground,
        ),
        sidebar_accent: blend(from.sidebar_accent, to.sidebar_accent),
        sidebar_accent_foreground: blend(
            from.sidebar_accent_foreground,
            to.sidebar_accent_foreground,
        ),
        sidebar_border: blend(from.sidebar_border, to.sidebar_border),
        sidebar_ring: blend(from.sidebar_ring, to.sidebar_ring),
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
