//! Bare metal-filled text — Libraries.dev `MetalText.tsx` / Swift `MetalText`.

use iced::widget::{container, stack};
use iced::{Background, Color, Element, Length};

use super::mask::{MetalFxMask, MetalGlowMode};
use super::options::MetalFxSurface;
use super::preset::MetalFxTheme;
use super::text_mask::{
    METAL_TEXT_DEFAULTS, METAL_TEXT_PAD_CSS, MetalTextFont, TextInnerShadow, glyph_mask_image,
    rasterize_text_mask, text_inner_shadow_mask,
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
    anchor_id: Option<String>,
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
            anchor_id: None,
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

    /// Publishes this text as a reflection anchor.
    pub fn anchor_id(mut self, id: impl Into<String>) -> Self {
        self.anchor_id = Some(id.into());
        self
    }

    /// Builds the iced element.
    pub fn into_element<'a, Message>(self) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        let density = 2.0;
        let mask = rasterize_text_mask(&self.content, &self.font, density, METAL_TEXT_PAD_CSS);
        let (pw, ph) = mask.pixel_size();
        let css_w = pw as f32 / density;
        let css_h = ph as f32 / density;

        // Base colour and metal share the same swash mask so glyphs never
        // drift (iced `text` uses different metrics than the mask bake).
        let base = glyph_mask_image(&mask, self.color);
        let body = container(base)
            .width(Length::Fixed(css_w))
            .height(Length::Fixed(css_h))
            .style(|_| container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                ..container::Style::default()
            });

        let mut fx = MetalFx::new(body)
            .theme(self.theme)
            .strength(self.strength * self.metal_opacity)
            .shader_scale(self.shader_scale)
            .border_radius(4.0)
            .ring_px(0.0)
            .glow(self.glow)
            .glow_gain(self.glow_gain)
            // Bare text: no pill ring rim / after-hairline — glyph rim is a
            // separate overlay (matches BARE_CSS + drawInnerShadow).
            .inner_shadow(false)
            .mask(MetalFxMask::Custom(mask.clone()))
            .glow_mode(MetalGlowMode::Mask)
            .surface(MetalFxSurface::Transparent)
            .paused(self.paused);
        if let Some(id) = self.anchor_id {
            fx = fx.anchor_id(id);
        }

        let metal: Element<'a, Message> = fx.into();
        if let Some(shadow) = self.inner_shadow {
            let rim = text_inner_shadow_mask(&mask, shadow);
            let rim_img = glyph_mask_image(&rim, Color::WHITE);
            stack![metal, rim_img]
                .width(Length::Fixed(css_w))
                .height(Length::Fixed(css_h))
                .into()
        } else {
            metal
        }
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
        AlphaMask, MetalFxHost, MetalFxOptions, MetalFxResolvedTheme, MetalRenderOptions,
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
            .border_radius(4.0)
            .inner_shadow(false)
            .ring_px(0.0);
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

    #[test]
    fn bare_text_skips_surface_and_box_rim_layers() {
        use super::super::scene::{Layer, tests::scene};
        let mut s = scene(MetalFxResolvedTheme::Dark, false);
        s.surface = None;
        s.rim = None;
        s.mask = MetalFxMask::Custom(AlphaMask::solid(8, 4));
        s.glow_mode = MetalGlowMode::Mask;
        assert!(!s.has_layer(Layer::Surface));
        assert!(!s.has_layer(Layer::Rim));
        assert!(s.has_layer(Layer::Ring));
    }

    #[test]
    fn base_tint_shares_mask_pixel_grid() {
        let mask = rasterize_text_mask("Pro", &MetalTextFont::default(), 2.0, 2.0);
        let (pw, ph) = mask.pixel_size();
        assert!(pw > 8 && ph > 8);
        // Interior ink must exist so the tinted base is visible under metal.
        assert!(mask.interior_points(2.0).len() > 4);
    }
}
