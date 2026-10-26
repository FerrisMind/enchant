//! The canvas backend: the same beam rendered through
//! [`EffectProgram`](crate::primitives::EffectProgram) instead of the image
//! widget path.
//!
//! [`BorderBeam`](super::BorderBeam) owns a clock and fades the effect over its
//! child; [`BorderBeamCanvas`] is the stateless counterpart for callers that
//! drive the clock themselves (their own [`canvas::Program`], a game loop, a
//! headless frame). Both share [`BorderBeamOptions`], the frame model and
//! [`BorderBeamRaster`], so the layers are identical.
//!
//! The canvas cannot read the surrounding iced theme, so `Auto` resolves to the
//! dark preset here; pass [`BorderBeamCanvas::system`] to follow the app theme
//! yourself. `pulse-outside` paints its bloom outside the layout bounds through
//! `EffectCanvas`'s draw overflow, so the element must be placed anywhere that
//! does not clip its parent (for example a [`Stack`](iced::widget::stack)).

use std::cell::RefCell;
use std::fmt;
use std::time::Duration;

use iced::widget::canvas;
use iced::{Element, Padding, Point, Rectangle, Renderer, mouse};

use super::frame::BorderBeamFrame;
use super::options::BorderBeamOptions;
use super::palette::{BeamHost, ResolvedBeamTheme};
use super::raster::{BorderBeamRaster, outside_margin};
use super::widget::LayerImage;
use crate::{EffectCanvasProps, EffectProgram, effect_canvas};

/// A stateless canvas variant of [`BorderBeam`](super::BorderBeam).
///
/// Draw one frame at an explicit time and opacity. The element is laid out at
/// the given host size and draws the `pulse-outside` bloom outside those
/// bounds; the other sizes stay inside.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::{BorderBeamCanvas, BorderBeamOptions, BorderBeamSize};
/// use iced::Element;
/// use std::time::Duration;
///
/// #[derive(Clone)]
/// enum Message {}
///
/// fn beam<'a>() -> Element<'a, Message> {
///     BorderBeamCanvas::new(
///         BorderBeamOptions::new()
///             .size(BorderBeamSize::PulseOutside)
///             .radius(20.0),
///     )
///     .host(348.0, 122.0)
///     .elapsed(Duration::from_secs_f32(0.8))
///     .pixel_density(2.0)
///     .into_element()
/// }
/// # let _ = beam;
/// ```
#[must_use = "canvas beams do nothing unless turned into an iced Element"]
#[derive(Clone)]
pub struct BorderBeamCanvas {
    options: BorderBeamOptions,
    host: BeamHost,
    elapsed: Duration,
    system: ResolvedBeamTheme,
    density: f32,
    opacity: f32,
}

impl fmt::Debug for BorderBeamCanvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorderBeamCanvas")
            .field("options", &self.options)
            .field("host", &self.host)
            .field("elapsed", &self.elapsed)
            .field("system", &self.system)
            .field("density", &self.density)
            .field("opacity", &self.opacity)
            .finish()
    }
}

impl BorderBeamCanvas {
    /// Creates the canvas with a zero host and the dark preset. Set at least
    /// [`host`](Self::host).
    pub fn new(options: BorderBeamOptions) -> Self {
        Self {
            options,
            host: BeamHost::new(0.0, 0.0),
            elapsed: Duration::ZERO,
            system: ResolvedBeamTheme::Dark,
            density: 1.0,
            opacity: 1.0,
        }
    }

    /// The logical size the beam is drawn for (the child's size in the widget
    /// variant). Non-finite and negative values are treated as `0`.
    pub fn host(mut self, width: f32, height: f32) -> Self {
        self.host = BeamHost::new(width, height);
        self
    }

    /// Animation time. Wraps through the same keyframes at every duration.
    pub fn elapsed(mut self, elapsed: Duration) -> Self {
        self.elapsed = elapsed;
        self
    }

    /// System theme used to resolve
    /// [`BorderBeamTheme::Auto`](super::BorderBeamTheme::Auto).
    pub fn system(mut self, system: ResolvedBeamTheme) -> Self {
        self.system = system;
        self
    }

    /// Device pixels per logical pixel for the rasterized layers
    /// (`0.5..=4`, default 1).
    pub fn pixel_density(mut self, density: f32) -> Self {
        self.density = finite_clamp(density, 0.5, 4.0, 1.0);
        self
    }

    /// Layer opacity in `0..=1` (default 1). An inactive
    /// [`BorderBeamOptions::active`] gates this to zero; drive the fade by
    /// changing `opacity` while keeping the effect active.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = finite_clamp(opacity, 0.0, 1.0, 0.0);
        self
    }

    /// Layout size requested from iced.
    pub fn size(&self) -> iced::Size {
        iced::Size::new(self.host.width(), self.host.height())
    }

    /// The draw overflow needed by this size (`0` except `pulse-outside`).
    pub fn draw_overflow(&self) -> f32 {
        outside_margin(&self.options.spec(self.system))
    }

    /// Turns the program into an [`Element`].
    ///
    /// The element is fixed-size (the host) and never requests redraws: drive
    /// animation by replacing the value, like any other iced widget.
    pub fn into_element<'a, Message: 'a>(self) -> Element<'a, Message> {
        let halo = self.draw_overflow();
        // Device-pixel rounding makes the padded bitmap up to a couple of
        // logical pixels wider than `host + 2 * halo`; give the canvas frame
        // enough overflow that `canvas::Frame` cannot clip its right/bottom.
        let margin = if halo > 0.0 {
            halo + 3.0 / self.density
        } else {
            0.0
        };
        let size = self.size();
        effect_canvas::<Self, Message>(
            self,
            EffectCanvasProps::new()
                .width(size.width)
                .height(size.height)
                .draw_overflow(Padding {
                    top: margin,
                    right: margin,
                    bottom: margin,
                    left: margin,
                }),
        )
        .into()
    }

    fn effective_opacity(&self) -> f32 {
        if self.options.is_active() {
            self.opacity
        } else {
            0.0
        }
    }
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

/// Opaque per-element cache for [`BorderBeamCanvas`].
///
/// It exists only as [`EffectProgram::State`]; construct it with
/// [`Default`] and do not inspect it.
///
/// # Examples
///
/// ```rust
/// use enchant_iced::BorderBeamCanvasState;
///
/// let _state = BorderBeamCanvasState::default();
/// ```
#[derive(Default)]
pub struct BorderBeamCanvasState {
    inner: RefCell<CanvasInner>,
}

impl fmt::Debug for BorderBeamCanvasState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cached = self.inner.try_borrow().map(|inner| inner.cache.is_some());
        f.debug_struct("BorderBeamCanvasState")
            .field("cached", &cached.unwrap_or(true))
            .finish()
    }
}

#[derive(Default)]
struct CanvasInner {
    raster: BorderBeamRaster,
    cache: Option<CanvasCache>,
}

struct CanvasCache {
    frame: BorderBeamFrame,
    opacity_bits: u32,
    density_bits: u32,
    above: Option<LayerImage>,
    behind: Option<LayerImage>,
}

impl<'a, Message: 'a> From<BorderBeamCanvas> for Element<'a, Message> {
    fn from(beam: BorderBeamCanvas) -> Self {
        beam.into_element()
    }
}

impl<Message> EffectProgram<Message> for BorderBeamCanvas {
    type State = BorderBeamCanvasState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let opacity = self.effective_opacity();
        if bounds.width < 1.0 || bounds.height < 1.0 || opacity <= 0.0 {
            return Vec::new();
        }

        let frame = self.options.frame_at(self.elapsed, self.host, self.system);
        let opacity_bits = opacity.to_bits();
        let density_bits = self.density.to_bits();
        let mut inner = state.inner.borrow_mut();

        let hit = inner.cache.as_ref().is_some_and(|cache| {
            cache.frame == frame
                && cache.opacity_bits == opacity_bits
                && cache.density_bits == density_bits
        });
        if !hit {
            let layers = inner.raster.render(&frame, opacity, self.density);
            let (above, behind) = layers.into_parts();
            inner.cache = Some(CanvasCache {
                frame,
                opacity_bits,
                density_bits,
                above: above.map(LayerImage::new),
                behind: behind.map(LayerImage::new),
            });
        }

        let Some(cache) = inner.cache.as_ref() else {
            return Vec::new();
        };
        let origin = Point::new(
            (bounds.width - self.host.width()) * 0.5,
            (bounds.height - self.host.height()) * 0.5,
        );
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        if let Some(behind) = cache.behind.as_ref() {
            frame.draw_image(behind.destination_at(origin), behind.handle());
        }
        if let Some(above) = cache.above.as_ref() {
            frame.draw_image(above.destination_at(origin), above.handle());
        }
        vec![frame.into_geometry()]
    }
}
