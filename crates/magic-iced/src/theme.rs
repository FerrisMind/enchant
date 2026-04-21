use iced_shadcn::Theme;
use twill::prelude::{
    DynamicSemanticTheme, Easing as TwillEasing, MotionDefaults, SemanticColor, SemanticThemeVars,
    ThemeVariant, TransitionDuration,
};

pub struct MagicTheme;

impl MagicTheme {
    pub fn light() -> Theme {
        with_magic_registry(
            Theme::from_semantic_theme(SemanticThemeVars::shadcn_neutral(), ThemeVariant::Light),
            ThemeVariant::Light,
        )
    }

    pub fn dark() -> Theme {
        with_magic_registry(
            Theme::from_semantic_theme(SemanticThemeVars::shadcn_neutral(), ThemeVariant::Dark),
            ThemeVariant::Dark,
        )
    }

    pub fn from_dynamic(theme: &DynamicSemanticTheme, variant: ThemeVariant) -> Theme {
        with_magic_registry(Theme::from_dynamic_semantic_theme(theme, variant), variant)
    }
}

fn with_magic_registry(mut theme: Theme, variant: ThemeVariant) -> Theme {
    let primary = theme.semantic_color(SemanticColor::Primary);
    let accent = theme.semantic_color(SemanticColor::Accent);
    let ring = theme.semantic_color(SemanticColor::Ring);
    let chart2 = theme.semantic_color(SemanticColor::Chart2);
    let chart4 = theme.semantic_color(SemanticColor::Chart4);
    let foreground = theme.semantic_color(SemanticColor::Foreground);
    let registry = &mut theme.registry;

    registry.set_color("magic.beam.from", primary);
    registry.set_color("magic.beam.to", chart2);
    registry.set_color("magic.shine.color", foreground);
    registry.set_color("magic.gradient.from", primary);
    registry.set_color("magic.gradient.via", accent);
    registry.set_color("magic.gradient.to", chart4);
    registry.set_color("magic.glow.color", ring);
    registry.set_color("magic.typing.cursor", foreground);

    registry.set_number("magic.opacity.glow", glow_opacity(variant));
    registry.set_number("magic.opacity.beam_path", 0.22);
    registry.set_number("magic.opacity.surface_noise", 0.08);

    registry.set_duration_ms(
        "magic.duration.default",
        duration_ms(MotionDefaults::default().duration()),
    );
    registry.set_duration_ms(
        "magic.duration.fast",
        duration_ms(TransitionDuration::Ms150),
    );
    registry.set_duration_ms(
        "magic.duration.normal",
        duration_ms(TransitionDuration::Ms300),
    );
    registry.set_duration_ms(
        "magic.duration.slow",
        duration_ms(TransitionDuration::Ms700),
    );

    registry.set_string("magic.easing.default", TwillEasing::InOut.value());
    registry.set_string("magic.media.masking", "image-slot-only");
    registry.set_string("magic.geometry.measurement", "widget-id-operation");

    theme
}

fn duration_ms(token: TransitionDuration) -> u64 {
    token.as_millis() as u64
}

fn glow_opacity(variant: ThemeVariant) -> f32 {
    match variant {
        ThemeVariant::Light => 0.24,
        ThemeVariant::Dark => 0.42,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_light_theme_populates_registry_keys() {
        let theme = MagicTheme::light();

        assert!(theme.registry.color("magic.beam.from").is_some());
        assert!(theme.registry.color("magic.gradient.to").is_some());
        assert_eq!(theme.registry.duration_ms("magic.duration.fast"), Some(150));
        assert_eq!(
            theme.registry.string("magic.media.masking"),
            Some("image-slot-only")
        );
    }

    #[test]
    fn magic_theme_does_not_leak_later_phase_widget_keys() {
        let theme = MagicTheme::dark();

        assert!(theme.registry.color("magic.typing.cursor").is_some());
        assert!(theme.registry.color("magic.card.glow.from").is_none());
        assert!(theme.registry.color("magic.button.shimmer").is_none());
        assert!(theme.registry.color("magic.pattern.grid").is_none());
        assert!(theme.registry.color("magic.dock.surface").is_none());
        assert!(theme.registry.color("magic.light_rays.color").is_none());
        assert!(theme.registry.color("magic.globe.base").is_none());
        assert!(theme.registry.color("magic.video.text.tint").is_none());
        assert!(theme.registry.number("magic.card.glow.opacity").is_none());
        assert!(theme.registry.duration_ms("magic.duration.hyper").is_none());
    }

    #[test]
    fn magic_dynamic_theme_uses_dynamic_semantic_colors() {
        let dynamic = DynamicSemanticTheme::from_brand_oklch(0.628, 0.258, 29.234);
        let theme = MagicTheme::from_dynamic(&dynamic, ThemeVariant::Dark);
        let expected_primary =
            iced_shadcn::Theme::from_dynamic_semantic_theme(&dynamic, ThemeVariant::Dark)
                .palette
                .primary;

        assert_eq!(theme.palette.primary, expected_primary);
        assert!(theme.registry.color("magic.beam.to").is_some());
    }

    #[test]
    fn magic_theme_registry_is_limited_to_phase0_foundation_keys() {
        let theme = MagicTheme::dark();

        assert!(theme.registry.color("magic.beam.from").is_some());
        assert!(theme.registry.color("magic.gradient.to").is_some());
        assert!(theme.registry.color("magic.glow.color").is_some());
        assert_eq!(
            theme.registry.string("magic.geometry.measurement"),
            Some("widget-id-operation")
        );

        assert!(theme.registry.color("magic.card.glow.from").is_none());
        assert!(theme.registry.color("magic.button.shimmer").is_none());
        assert!(theme.registry.color("magic.dock.surface").is_none());
        assert!(theme.registry.color("magic.retro_grid.light").is_none());
        assert!(theme.registry.color("magic.globe.base").is_none());
        assert!(theme.registry.color("magic.video.text.tint").is_none());
    }
}
