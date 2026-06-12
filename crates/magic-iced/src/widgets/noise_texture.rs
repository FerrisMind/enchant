use crate::primitives::{EffectCanvasProps, EffectProgram, effect_canvas, stable_noise};
use iced::widget::{canvas, stack};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoiseTextureProps {
    pub width: Length,
    pub height: Length,
    /// `baseFrequency` analogue for the procedural noise field.
    pub frequency: f32,
    pub octaves: u32,
    pub slope: f32,
    pub noise_opacity: f32,
    /// Root overlay opacity (`opacity-50` in the reference).
    pub layer_opacity: f32,
    pub seed: u64,
    /// Optional radial fade from center (demo 1 mask).
    pub radial_mask_radius: Option<f32>,
}

impl Default for NoiseTextureProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fill,
            frequency: 0.4,
            octaves: 6,
            slope: 0.15,
            noise_opacity: 0.6,
            layer_opacity: 0.5,
            seed: 17,
            radial_mask_radius: None,
        }
    }
}

impl NoiseTextureProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.max(0.01);
        self
    }

    pub fn octaves(mut self, octaves: u32) -> Self {
        self.octaves = octaves.clamp(1, 8);
        self
    }

    pub fn slope(mut self, slope: f32) -> Self {
        self.slope = slope.max(0.0);
        self
    }

    pub fn noise_opacity(mut self, noise_opacity: f32) -> Self {
        self.noise_opacity = noise_opacity.clamp(0.0, 1.0);
        self
    }

    pub fn layer_opacity(mut self, layer_opacity: f32) -> Self {
        self.layer_opacity = layer_opacity.clamp(0.0, 1.0);
        self
    }

    pub fn radial_mask_radius(mut self, radius: f32) -> Self {
        self.radial_mask_radius = Some(radius.max(1.0));
        self
    }
}

pub fn noise_texture<'a, Message: 'a>(props: NoiseTextureProps) -> Element<'a, Message> {
    effect_canvas::<_, Message>(
        NoiseTextureProgram { props },
        EffectCanvasProps::new()
            .width(props.width)
            .height(props.height),
    )
    .into()
}

pub fn noise_texture_layer<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    props: NoiseTextureProps,
) -> Element<'a, Message> {
    stack(vec![content.into(), noise_texture(props)]).into()
}

struct NoiseTextureProgram {
    props: NoiseTextureProps,
}

impl<Message> EffectProgram<Message> for NoiseTextureProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let step = 3.0;
        let columns = (bounds.width / step).ceil().max(1.0) as i32;
        let rows = (bounds.height / step).ceil().max(1.0) as i32;
        let center = Point::new(bounds.width * 0.5, bounds.height * 0.5);
        let mask_radius = self.props.radial_mask_radius;

        for column in 0..columns {
            for row in 0..rows {
                let x = column as f32 * step;
                let y = row as f32 * step;
                let sample = fractal_noise(
                    x / bounds.width.max(1.0),
                    y / bounds.height.max(1.0),
                    self.props.frequency,
                    self.props.octaves,
                    self.props.seed,
                );
                let contrasted = ((sample - 0.5) * self.props.slope * 4.0 + 0.5).clamp(0.0, 1.0);
                let mut alpha = contrasted * self.props.noise_opacity * self.props.layer_opacity;

                if let Some(radius) = mask_radius {
                    let distance = ((x + step * 0.5 - center.x).powi(2)
                        + (y + step * 0.5 - center.y).powi(2))
                    .sqrt();
                    let fade = 1.0 - (distance / radius).clamp(0.0, 1.0);
                    alpha *= fade * fade;
                }

                if alpha <= f32::EPSILON {
                    continue;
                }

                frame.fill_rectangle(
                    Point::new(x, y),
                    Size::new(step, step),
                    Color::from_rgba(1.0, 1.0, 1.0, alpha),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn fractal_noise(x: f32, y: f32, frequency: f32, octaves: u32, seed: u64) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut total = 0.0;
    let mut scale = frequency.max(0.01);

    for octave in 0..octaves {
        let nx = x * scale * 24.0;
        let ny = y * scale * 24.0;
        let cell = (nx.floor() as i64)
            .wrapping_mul(73856093)
            .wrapping_add((ny.floor() as i64).wrapping_mul(19349663))
            .wrapping_add(octave as i64);
        value += stable_noise(seed, cell as u64) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        scale *= 2.0;
    }

    value / total.max(f32::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_texture_defaults_match_reference() {
        let props = NoiseTextureProps::new();
        assert!((props.frequency - 0.4).abs() < f32::EPSILON);
        assert_eq!(props.octaves, 6);
        assert!((props.slope - 0.15).abs() < f32::EPSILON);
        assert!((props.noise_opacity - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn fractal_noise_stays_normalized() {
        let sample = fractal_noise(0.42, 0.17, 0.4, 6, 9);
        assert!((0.0..=1.0).contains(&sample));
    }
}
