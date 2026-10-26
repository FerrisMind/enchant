//! "New" badge — Libraries.dev `MetalBadge.tsx` / Swift `MetalBadge`.
//!
//! White pill, live liquid-metal sheet fill, clean white core under the label,
//! inset glows/hairlines, and the label.

use iced::font::{Family, Font, Weight};
use iced::widget::{column, container, stack, text};
use iced::{Background, Border, Color, Element, Length, Shadow, Vector};

use super::mask::{MetalFxMask, MetalGlowMode};
use super::options::{MetalFxLayer, MetalFxSurface};
use super::preset::MetalFxTheme;
use super::widget::MetalFx;

const RADIUS: f32 = 55.556;
const WIDTH: f32 = 45.0;
const HEIGHT: f32 = 25.0;

/// White core under the label (`METAL_BADGE_DEFAULTS.core`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalBadgeCore {
    /// Solid radius, % of the ellipse.
    pub r: f32,
    /// Ramp width, %.
    pub blur: f32,
    /// Opacity.
    pub a: f32,
    /// Ellipse size, % of the box.
    pub size: f32,
}

impl Default for MetalBadgeCore {
    fn default() -> Self {
        Self {
            r: 46.0,
            blur: 100.0,
            a: 0.94,
            size: 49.0,
        }
    }
}

/// Tuned defaults from `METAL_BADGE_DEFAULTS`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalBadgeDefaults {
    pub metal_opacity: f32,
    pub shader_scale: f32,
    pub core: MetalBadgeCore,
    pub gradient: f32,
    pub glow: f32,
}

impl Default for MetalBadgeDefaults {
    fn default() -> Self {
        Self {
            metal_opacity: 0.8,
            shader_scale: 1.6,
            core: MetalBadgeCore::default(),
            gradient: 0.0,
            glow: 0.41,
        }
    }
}

/// Shared defaults (`METAL_BADGE_DEFAULTS`).
pub const METAL_BADGE_DEFAULTS: MetalBadgeDefaults = MetalBadgeDefaults {
    metal_opacity: 0.8,
    shader_scale: 1.6,
    core: MetalBadgeCore {
        r: 46.0,
        blur: 100.0,
        a: 0.94,
        size: 49.0,
    },
    gradient: 0.0,
    glow: 0.41,
};

/// The "New" metal badge.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::widgets::metal_fx::MetalBadge;
/// use iced::Element;
///
/// #[derive(Clone)]
/// enum Message {}
///
/// fn view<'a>() -> Element<'a, Message> {
///     MetalBadge::new("New").into()
/// }
/// # let _ = view;
/// ```
#[must_use = "widgets do nothing unless turned into an iced Element"]
#[derive(Clone, Debug)]
pub struct MetalBadge {
    label: String,
    theme: MetalFxTheme,
    strength: f32,
    scale: f32,
    metal_opacity: f32,
    shader_scale: f32,
    core: MetalBadgeCore,
    gradient: f32,
    glow: f32,
    text_color: Color,
    paused: bool,
}

impl Default for MetalBadge {
    fn default() -> Self {
        Self::new("New")
    }
}

impl MetalBadge {
    /// Builds a badge with the given label (default `"New"`).
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            theme: MetalFxTheme::Auto,
            strength: 1.0,
            scale: 1.0,
            metal_opacity: METAL_BADGE_DEFAULTS.metal_opacity,
            shader_scale: METAL_BADGE_DEFAULTS.shader_scale,
            core: METAL_BADGE_DEFAULTS.core,
            gradient: METAL_BADGE_DEFAULTS.gradient,
            glow: METAL_BADGE_DEFAULTS.glow,
            text_color: Color::from_rgb8(0x32, 0x32, 0x32),
            paused: false,
        }
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

    /// Size multiplier on the Figma 45×25 metrics.
    pub fn scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.scale = scale.clamp(0.25, 8.0);
        }
        self
    }

    /// How much metal shows over the white fill.
    pub fn metal_opacity(mut self, opacity: f32) -> Self {
        if opacity.is_finite() {
            self.metal_opacity = opacity.clamp(0.0, 1.0);
        }
        self
    }

    /// Shader sampling scale.
    pub fn shader_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.shader_scale = scale.max(0.01);
        }
        self
    }

    /// White core under the label.
    pub fn core(mut self, core: MetalBadgeCore) -> Self {
        self.core = core;
        self
    }

    /// Top→bottom white wash `0..=1`.
    pub fn gradient(mut self, gradient: f32) -> Self {
        if gradient.is_finite() {
            self.gradient = gradient.clamp(0.0, 1.0);
        }
        self
    }

    /// Inner white glow strength `0..=1`.
    pub fn glow(mut self, glow: f32) -> Self {
        if glow.is_finite() {
            self.glow = glow.clamp(0.0, 1.0);
        }
        self
    }

    /// Label colour.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
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
        let k = self.scale;
        let w = WIDTH * k;
        let h = HEIGHT * k;
        let radius = RADIUS * k;
        // Figma: Inter Semi Bold 12.222 / 1.4 #323232.
        let label = text(self.label.clone())
            .size(12.222 * k)
            .font(Font {
                family: Family::SansSerif,
                weight: Weight::Semibold,
                ..Font::DEFAULT
            })
            .color(self.text_color);
        // Keep Fixed size — `center_x(Fill)` would expand the host.
        let label_box = container(label)
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center);

        // White core under the label (ellipse ≈ `core.size`% of the box) so
        // metal stays at the rim and text sits on clean white.
        let core_w = (w * self.core.size / 100.0).max(1.0);
        let core_h = (h * self.core.size / 100.0).max(1.0);
        let core_r = core_w.min(core_h) * 0.5;
        let core_blob = container(column![])
            .width(Length::Fixed(core_w))
            .height(Length::Fixed(core_h))
            .style(move |_| container::Style {
                background: Some(Background::Color(Color::from_rgba(
                    1.0,
                    1.0,
                    1.0,
                    self.core.a,
                ))),
                border: Border {
                    radius: core_r.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            });
        let core_overlay = container(core_blob)
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center);

        // Gradient wash + inset rims (layers 3 + 5) above the metal, below text.
        let glow_overlay = container(column![])
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .style(move |_| container::Style {
                background: Some(Background::Color(Color::from_rgba(
                    1.0,
                    1.0,
                    1.0,
                    self.gradient,
                ))),
                border: Border {
                    radius: radius.into(),
                    width: 0.833 * k,
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.5),
                },
                shadow: Shadow {
                    color: Color::from_rgba(1.0, 1.0, 1.0, self.glow),
                    offset: Vector::new(0.0, 0.833 * k),
                    blur_radius: 8.333 * k,
                },
                ..container::Style::default()
            });

        // Content above metal: core → rims → label (Figma bottom→top 3b/3/5/4).
        let body = stack![core_overlay, glow_overlay, label_box]
            .width(Length::Fixed(w))
            .height(Length::Fixed(h));

        MetalFx::new(body)
            .theme(self.theme)
            .strength(self.strength * self.metal_opacity)
            .shader_scale(self.shader_scale)
            .border_radius(radius)
            .glow(true)
            .glow_gain(1.0)
            .mask(MetalFxMask::Fill)
            .glow_mode(MetalGlowMode::Ring)
            .surface(MetalFxSurface::Solid(Color::WHITE))
            // Metal under the label/core — default OverChild buried "New".
            .layer(MetalFxLayer::BehindChild)
            .paused(self.paused)
            .into()
    }
}

impl<'a, Message> From<MetalBadge> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(value: MetalBadge) -> Self {
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
    fn fill_mask_covers_the_pill_interior() {
        let options = MetalFxOptions::new()
            .mask(MetalFxMask::Fill)
            .glow(false)
            .border_radius(RADIUS)
            .shader_scale(1.6)
            .surface(MetalFxSurface::Solid(Color::WHITE));
        let bitmap = options.render_at(
            Duration::from_millis(800),
            MetalFxHost::new(WIDTH, HEIGHT),
            MetalFxResolvedTheme::Light,
            &MetalRenderOptions::new(),
        );
        let mid = bitmap.pixel_at(WIDTH * 0.5, HEIGHT * 0.5);
        // Fill keeps metal across the pill (unlike the ring punch).
        assert!(mid[3] > 200, "pill interior should be opaque, got {mid:?}");
    }
}
