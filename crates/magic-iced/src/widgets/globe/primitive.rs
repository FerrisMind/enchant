use crate::widgets::globe::pipeline::GlobePipeline;
use crate::widgets::globe::{GlobeProps, GlobeState};
use iced::Rectangle;
use iced::wgpu;
use iced::widget::shader;

#[derive(Debug)]
pub struct GlobePrimitive {
    state: GlobeState,
    props: GlobeProps,
    dpr: f32,
}

impl GlobePrimitive {
    pub fn new(state: &GlobeState, props: &GlobeProps) -> Self {
        Self {
            state: state.clone(),
            props: props.clone(),
            dpr: props.device_pixel_ratio,
        }
    }
}

impl shader::Primitive for GlobePrimitive {
    type Pipeline = GlobePipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        pipeline.prepare(
            device,
            queue,
            viewport,
            &self.state,
            &self.props,
            *bounds,
            self.dpr,
        );
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        pipeline.draw(render_pass);
        true
    }
}
