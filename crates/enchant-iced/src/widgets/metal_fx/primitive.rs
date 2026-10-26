//! The iced shader primitive: one layer of one instance.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use iced::Rectangle;
use iced::wgpu;
use iced::widget::shader;

use super::pipeline::MetalFxPipeline;
use super::scene::{Layer, Scene};

/// One draw of the shared pipeline: a layer of a [`Scene`].
///
/// iced prepares every primitive of a frame before drawing any, in submission
/// order; `prepare` records the pipeline slot it was given so `draw` can find
/// it again through `&self`.
#[derive(Debug)]
pub(crate) struct MetalFxPrimitive {
    scene: Arc<Scene>,
    layer: Layer,
    slot: AtomicUsize,
}

impl MetalFxPrimitive {
    pub(crate) fn new(scene: Arc<Scene>, layer: Layer) -> Self {
        Self {
            scene,
            layer,
            slot: AtomicUsize::new(usize::MAX),
        }
    }
}

impl shader::Primitive for MetalFxPrimitive {
    type Pipeline = MetalFxPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let slot = pipeline.prepare(
            device,
            queue,
            &self.scene,
            self.layer,
            viewport.scale_factor(),
        );
        self.slot.store(slot, Ordering::Release);
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        let slot = self.slot.load(Ordering::Acquire);
        pipeline.draw(render_pass, slot, self.layer, self.scene.theme.is_light());
        true
    }
}
