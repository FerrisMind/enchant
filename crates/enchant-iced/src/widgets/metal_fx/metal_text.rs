//! Bare metal-filled text — Libraries.dev `MetalText.tsx` / Swift `MetalText`.

use iced::widget::{container, text};
use iced::{Background, Color, Element, Length};

use super::mask::{MetalFxMask, MetalGlowMode};
use super::options::MetalFxSurface;
use super::preset::MetalFxTheme;
use super::text_mask::{
    METAL_TEXT_DEFAULTS, MetalTextFont, TextInnerShadow, rasterize_text_mask, text_inner_shadow_mask,
};
use super::widget::MetalFx;

/// Metal inside the glyphs of a word.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::{MetalText, MetalTextFont};
/// use iced::{Color, Element};
///
/// #[derive(Clone)]
/// enum Message {}
///
/// fn view<'a>() -> Element<'a, Message> {
///     MetalText::new("Pro")
///         .font(MetalTextFont { size: 24.0, weight: 500.0 })
///         .color(Color::from_rgb(0.89, 0.89, 0.89))
///         .into()
/// }
/// # let _ = view;
/// ```
#[must_use = "widgets do nothing unless turned into an iced Element"]
#[derive(Clone, Debug)]
pub struct MetalText {
    content: String,
    font: MetalTextFont,
    color: Color,
    theme: MetalFxTheme,
    strength: f32,
    metal_opacity: f32,
    shader_scale: f32,
    inner_shadow: Option<TextInnerShadow>,
    glow: bool,
    glow_gain: f32,
    paused: bool,
}

impl MetalText {
    /// Builds metal text for `content`.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            font: MetalTextFont::default(),
            color: Color::from_rgb8(0xE2, 0xE2, 0xE2),
            theme: MetalFxTheme::Auto,
            strength: 1.0,
            metal_opacity: METAL_TEXT_DEFAULTS.metal_opacity,
            shader_scale: METAL_TEXT_DEFAULTS.shader_scale,
            inner_shadow: Some(METAL_TEXT_DEFAULTS.inner_shadow),
            glow: false,
            glow_gain: METAL_TEXT_DEFAULTS.glow_gain,
            paused: false,
        }
    }

    /// Font size / weight (Geist variable).
    pub fn font(mut self, font: MetalTextFont) -> Self {
        self.font = font;
        self
    }

    /// Base text colour the metal composites over.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Theme mode.
    pub fn theme(mut self, theme: MetalFxTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Effect strength `0..=1`.
    pub fn strength(mut self, strength: f32) -> Self {
        if strength.is_finite() {
            self.strength = strength.clamp(0.0, 1.0);
        }
        self
    }

    /// How much metal shows over the base colour.
    pub fn metal_opacity(mut self, opacity: f32) -> Self {
        if opacity.is_finite() {
            self.metal_opacity = opacity.clamp(0.0, 1.0);
        }
        self
    }

    /// Zoom of the metal inside the glyphs.
    pub fn shader_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.shader_scale = scale.max(0.01);
        }
        self
    }

    /// Top-edge light rim inside the glyphs; `None` disables it.
    pub fn inner_shadow(mut self, shadow: Option<TextInnerShadow>) -> Self {
        self.inner_shadow = shadow;
        self
    }

    /// Halo on the glyphs (off by default).
    pub fn glow(mut self, glow: bool) -> Self {
        self.glow = glow;
        self
    }

    /// Glow multiplier when glow is on.
    pub fn glow_gain(mut self, gain: f32) -> Self {
        if gain.is_finite() {
            self.glow_gain = gain.max(0.0);
        }
        self
    }

    /// Freezes the metal animation.
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Builds the iced element.
    pub fn into_element<'a, Message>(self) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        let density = 2.0;
        let pad = 2.0;
        let mask = rasterize_text_mask(&self.content, &self.font, density, pad);
        let (pw, ph) = mask.pixel_size();
        let css_w = pw as f32 / density;
        let css_h = ph as f32 / density;

        // Keep the rim bake exercised for parity (GPU rim is analytic + mask).
        if let Some(shadow) = self.inner_shadow {
            let _ = text_inner_shadow_mask(&mask, shadow);
        }

        let label = text(self.content.clone())
            .size(self.font.size)
            .color(self.color);
        let body = container(label)
            .width(Length::Fixed(css_w))
            .height(Length::Fixed(css_h))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..container::Style::default()
            });

        MetalFx::new(body)
            .theme(self.theme)
            .strength(self.strength * self.metal_opacity)
            .shader_scale(self.shader_scale)
            .border_radius(4.0)
            .glow(self.glow)
            .glow_gain(self.glow_gain)
            .inner_shadow(self.inner_shadow.is_some())
            .mask(MetalFxMask::Custom(mask))
            .glow_mode(MetalGlowMode::Mask)
            .surface(MetalFxSurface::Transparent)
            .paused(self.paused)
            .into()
    }
}

impl<'a, Message> From<MetalText> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(value: MetalText) -> Self {
        value.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::metal_fx::{
        MetalFxHost, MetalFxOptions, MetalFxResolvedTheme, MetalRenderOptions,
    };
    use std::time::Duration;

    #[test]
    fn glyph_mask_keeps_metal_inside_letters() {
        let mask = rasterize_text_mask("Pro", &MetalTextFont::default(), 2.0, 2.0);
        let (pw, ph) = mask.pixel_size();
        let options = MetalFxOptions::new()
            .mask(MetalFxMask::Custom(mask.clone()))
            .glow(false)
            .shader_scale(2.8)
            .surface(MetalFxSurface::Transparent)
            .border_radius(4.0);
        let host = MetalFxHost::new(pw as f32 / 2.0, ph as f32 / 2.0);
        let bitmap = options.render_at(
            Duration::from_millis(900),
            host,
            MetalFxResolvedTheme::Dark,
            &MetalRenderOptions::new().density(2.0),
        );
        // Outside the glyph box the metal must be gone.
        let corner = bitmap.pixel(0, 0);
        assert_eq!(corner[3], 0, "transparent outside glyphs");
        assert!(!mask.interior_points(2.0).is_empty());
    }
}
