//! The one shared wgpu pipeline every [`MetalFx`](super::MetalFx) instance
//! draws with.
//!
//! iced keeps a single [`MetalFxPipeline`] per device (see
//! [`Storage`](iced::widget::shader::Storage)); every layer of every instance
//! is a [`MetalFxPrimitive`](super::primitive::MetalFxPrimitive) that is
//! `prepare`d into a *slot* of this pipeline (its own uniform block plus the
//! glow sprites it samples) and later `draw`n from that slot, so any number of
//! instances share the shader module, the two render pipelines (normal and
//! `multiply` blend) and the sprite textures. Slots are recycled every frame in
//! [`Pipeline::trim`](iced::widget::shader::Pipeline::trim).

use std::borrow::Cow;
use std::collections::HashMap;

use iced::wgpu;
use iced::wgpu::util::DeviceExt;
use iced::widget::shader;

use super::scene::{Layer, Scene, UNIFORM_SIZE, Uniforms};

/// WGSL source of the metal-fx program.
pub const WGSL: &str = include_str!("shaders/metal_fx.wgsl");

/// Frames a sprite texture survives without being used.
const SPRITE_TTL_FRAMES: u64 = 240;

struct GpuSprite {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    last_used: u64,
}

struct Slot {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    halo_id: u64,
    extra_id: u64,
    mask_id: u64,
}

/// The shared metal-fx GPU state.
pub struct MetalFxPipeline {
    normal: wgpu::RenderPipeline,
    multiply: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    dummy: wgpu::TextureView,
    _dummy_texture: wgpu::Texture,
    /// 1×1 white — custom masks multiply against this when none is bound.
    dummy_mask: wgpu::TextureView,
    _dummy_mask_texture: wgpu::Texture,
    sprites: HashMap<u64, GpuSprite>,
    masks: HashMap<u64, GpuSprite>,
    slots: Vec<Slot>,
    next_slot: usize,
    frame: u64,
    srgb: bool,
}

impl std::fmt::Debug for MetalFxPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetalFxPipeline")
            .field("slots", &self.slots.len())
            .field("sprites", &self.sprites.len())
            .field("srgb", &self.srgb)
            .finish_non_exhaustive()
    }
}

fn premultiplied_over() -> wgpu::BlendState {
    let component = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };
    wgpu::BlendState {
        color: component,
        alpha: component,
    }
}

/// CSS `mix-blend-mode: multiply` for a premultiplied source:
/// `src·dst + dst·(1 − srcA)`, destination alpha untouched.
fn multiply() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Dst,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

impl MetalFxPipeline {
    /// Builds the shader module, bind group layout and both render pipelines.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("metal_fx.shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL)),
        });

        let sprite_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("metal_fx.bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(UNIFORM_SIZE),
                    },
                    count: None,
                },
                sprite_entry(1),
                sprite_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                sprite_entry(4),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("metal_fx.pipeline_layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let make = |label: &'static str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview: None,
                cache: None,
            })
        };
        let normal = make("metal_fx.pipeline.normal", premultiplied_over());
        let multiply = make("metal_fx.pipeline.multiply", multiply());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("metal_fx.sprite_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let dummy_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("metal_fx.dummy_sprite"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[0],
        );
        let dummy = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_mask_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("metal_fx.dummy_mask"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[255],
        );
        let dummy_mask = dummy_mask_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            normal,
            multiply,
            layout,
            sampler,
            dummy,
            _dummy_texture: dummy_texture,
            dummy_mask,
            _dummy_mask_texture: dummy_mask_texture,
            sprites: HashMap::new(),
            masks: HashMap::new(),
            slots: Vec::new(),
            next_slot: 0,
            frame: 0,
            srgb: format.is_srgb(),
        }
    }

    /// Whether the target format is sRGB (the shader then writes linear colour).
    #[must_use]
    pub fn target_is_srgb(&self) -> bool {
        self.srgb
    }

    /// Number of sprite textures currently resident.
    #[must_use]
    pub fn sprite_count(&self) -> usize {
        self.sprites.len()
    }

    fn upload_sprite(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sprite: &super::bake::GlowSprite,
    ) {
        let frame = self.frame;
        if let Some(existing) = self.sprites.get_mut(&sprite.id()) {
            existing.last_used = frame;
            return;
        }
        let (w, h) = sprite.pixel_size();
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("metal_fx.glow_sprite"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            sprite.alpha(),
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.sprites.insert(
            sprite.id(),
            GpuSprite {
                _texture: texture,
                view,
                last_used: frame,
            },
        );
    }

    fn upload_mask(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mask: &super::mask::AlphaMask,
    ) {
        let frame = self.frame;
        if let Some(existing) = self.masks.get_mut(&mask.id()) {
            existing.last_used = frame;
            return;
        }
        let (w, h) = mask.pixel_size();
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("metal_fx.alpha_mask"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            mask.alpha(),
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.masks.insert(
            mask.id(),
            GpuSprite {
                _texture: texture,
                view,
                last_used: frame,
            },
        );
    }

    fn make_slot(&self, device: &wgpu::Device, halo_id: u64, extra_id: u64, mask_id: u64) -> Slot {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("metal_fx.uniforms"),
            size: UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.bind_group(device, &buffer, halo_id, extra_id, mask_id);
        Slot {
            buffer,
            bind_group,
            halo_id,
            extra_id,
            mask_id,
        }
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        halo_id: u64,
        extra_id: u64,
        mask_id: u64,
    ) -> wgpu::BindGroup {
        let view = |id: u64| self.sprites.get(&id).map_or(&self.dummy, |s| &s.view);
        let mask_view = if mask_id == 0 {
            &self.dummy_mask
        } else {
            self.masks
                .get(&mask_id)
                .map_or(&self.dummy_mask, |s| &s.view)
        };
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("metal_fx.bind_group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(view(halo_id)),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(view(extra_id)),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(mask_view),
                },
            ],
        })
    }

    /// Uploads `scene`'s `layer` into the next free slot and returns its index.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        layer: Layer,
        density: f32,
    ) -> usize {
        let (halo_id, extra_id) = match (&scene.glow, layer) {
            (Some(g), Layer::Glow) => {
                self.upload_sprite(device, queue, &g.halo);
                self.upload_sprite(device, queue, &g.extra);
                (g.halo.id(), g.extra.id())
            }
            _ => (0, 0),
        };
        let mask_id = if let Some(mask) = scene.mask.custom() {
            self.upload_mask(device, queue, mask);
            mask.id()
        } else {
            0
        };

        let index = self.next_slot;
        self.next_slot += 1;
        if index == self.slots.len() {
            let slot = self.make_slot(device, halo_id, extra_id, mask_id);
            self.slots.push(slot);
        } else if self.slots[index].halo_id != halo_id
            || self.slots[index].extra_id != extra_id
            || self.slots[index].mask_id != mask_id
        {
            let bind_group = self.bind_group(
                device,
                &self.slots[index].buffer,
                halo_id,
                extra_id,
                mask_id,
            );
            let slot = &mut self.slots[index];
            slot.bind_group = bind_group;
            slot.halo_id = halo_id;
            slot.extra_id = extra_id;
            slot.mask_id = mask_id;
        }

        let mut uniforms: Uniforms = scene.uniforms(layer);
        uniforms.metrics[2] = density;
        uniforms.flags[2] = if self.srgb { 1.0 } else { 0.0 };
        queue.write_buffer(&self.slots[index].buffer, 0, bytemuck::bytes_of(&uniforms));
        index
    }

    /// Draws `slot` (prepared for `layer`) into `pass`.
    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        slot: usize,
        layer: Layer,
        light_theme: bool,
    ) {
        let Some(slot) = self.slots.get(slot) else {
            return;
        };
        let pipeline = if layer == Layer::Glow && light_theme {
            &self.multiply
        } else {
            &self.normal
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &slot.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}

impl shader::Pipeline for MetalFxPipeline {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self::new(device, queue, format)
    }

    fn trim(&mut self) {
        self.next_slot = 0;
        self.frame += 1;
        let frame = self.frame;
        self.sprites
            .retain(|_, s| frame - s.last_used < SPRITE_TTL_FRAMES);
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::thread::{self, Thread};

    use super::*;
    use crate::widgets::metal_fx::preset::MetalFxResolvedTheme;
    use crate::widgets::metal_fx::rim::MetalRimOptions;
    use crate::widgets::metal_fx::scene::tests::{glow_scene, scene};

    struct Unpark(Thread);

    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = Box::pin(future);
        let waker = Waker::from(Arc::new(Unpark(thread::current())));
        let mut cx = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
                return out;
            }
            thread::park();
        }
    }

    #[test]
    fn wgsl_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(WGSL)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(WGSL)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(WGSL)));

        let entries: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| (e.name.as_str(), e.stage))
            .collect();
        assert!(
            entries.contains(&("vs_main", naga::ShaderStage::Vertex)),
            "{entries:?}"
        );
        assert!(
            entries.contains(&("fs_main", naga::ShaderStage::Fragment)),
            "{entries:?}"
        );

        // The Rust uniform block and the WGSL struct must have the same size.
        let size = module
            .types
            .iter()
            .find_map(|(_, ty)| match (&ty.name, &ty.inner) {
                (Some(name), naga::TypeInner::Struct { span, .. }) if name == "Uniforms" => {
                    Some(*span)
                }
                _ => None,
            })
            .expect("Uniforms struct");
        assert_eq!(u64::from(size), UNIFORM_SIZE);

        // Every field of the Rust block appears in the WGSL block, in order.
        let names: Vec<String> = module
            .types
            .iter()
            .find_map(|(_, ty)| match (&ty.name, &ty.inner) {
                (Some(name), naga::TypeInner::Struct { members, .. }) if name == "Uniforms" => {
                    Some(members.iter().filter_map(|m| m.name.clone()).collect())
                }
                _ => None,
            })
            .expect("Uniforms members");
        assert_eq!(names.len(), 28);
        assert_eq!(names[0], "rect");
        assert_eq!(names[25], "shadow_misc");
        assert_eq!(names[26], "bend0");
        assert_eq!(names[27], "bend1");
    }

    struct Gpu {
        device: wgpu::Device,
        queue: wgpu::Queue,
    }

    fn gpu() -> Option<Gpu> {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        Some(Gpu { device, queue })
    }

    /// Renders one layer of `scene` into an `Rgba8Unorm` texture sized to the
    /// layer's quad and returns the premultiplied RGBA bytes.
    fn render_layer(
        gpu: &Gpu,
        pipeline: &mut MetalFxPipeline,
        scene: &Scene,
        layer: Layer,
        clear: wgpu::Color,
    ) -> (u32, u32, Vec<u8>) {
        let (_, size) = scene.quad(layer);
        let w = (size[0] * scene.density).round() as u32;
        let h = (size[1] * scene.density).round() as u32;
        let slot = pipeline.prepare(&gpu.device, &gpu.queue, scene, layer, scene.density);
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("metal_fx.test.target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let row = (w * 4).div_ceil(256) * 256;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("metal_fx.test.readback"),
            size: u64::from(row * h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("metal_fx.test.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pipeline.draw(&mut pass, slot, layer, scene.theme.is_light());
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map readback"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let start = (y * row) as usize;
            out.extend_from_slice(&data[start..start + (w * 4) as usize]);
        }
        drop(data);
        readback.unmap();
        shader::Pipeline::trim(pipeline);
        (w, h, out)
    }

    /// Compares the GPU render of `layer` against the CPU twin pixel by pixel.
    /// Returns `(max error, share of pixels within `tolerance`)`.
    fn compare(
        gpu: &Gpu,
        pipeline: &mut MetalFxPipeline,
        scene: &Scene,
        layer: Layer,
        tolerance: f32,
    ) -> (f32, f32) {
        // The light glow is drawn with the `multiply` pipeline: over a white
        // target it yields `c + (1 - a)` per channel.
        let multiply = layer == Layer::Glow && scene.theme.is_light();
        let clear = if multiply {
            wgpu::Color::WHITE
        } else {
            wgpu::Color::TRANSPARENT
        };
        let (w, h, bytes) = render_layer(gpu, pipeline, scene, layer, clear);
        let (origin, _) = scene.quad(layer);
        let d = scene.density;
        let (mut max_err, mut ok) = (0.0_f32, 0usize);
        for py in 0..h {
            for px in 0..w {
                let p = [
                    origin[0] + (px as f32 + 0.5) / d,
                    origin[1] + (py as f32 + 0.5) / d,
                ];
                let mut cpu = scene.shade(layer, p, [px as f32 + 0.5, py as f32 + 0.5]);
                if multiply {
                    let a = cpu[3];
                    cpu = [cpu[0] + 1.0 - a, cpu[1] + 1.0 - a, cpu[2] + 1.0 - a, 1.0];
                }
                let i = ((py * w + px) * 4) as usize;
                let mut worst = 0.0_f32;
                for k in 0..4 {
                    let gpu_v = f32::from(bytes[i + k]);
                    worst = worst.max((gpu_v - cpu[k].clamp(0.0, 1.0) * 255.0).abs());
                }
                max_err = max_err.max(worst);
                if worst <= tolerance {
                    ok += 1;
                }
            }
        }
        (max_err, ok as f32 / (w * h) as f32)
    }

    #[test]
    fn gpu_matches_the_cpu_reference() {
        let Some(gpu) = gpu() else {
            eprintln!(
                "metal_fx GPU parity test skipped: no wgpu adapter available on this machine"
            );
            return;
        };
        let mut pipeline =
            MetalFxPipeline::new(&gpu.device, &gpu.queue, wgpu::TextureFormat::Rgba8Unorm);
        assert!(!pipeline.target_is_srgb());

        let mut hi = scene(MetalFxResolvedTheme::Dark, false);
        hi.density = 2.0;
        hi.u_time = 3.7;
        let mut light_glow = glow_scene(MetalFxResolvedTheme::Light);
        if let Some(g) = light_glow.glow.as_mut() {
            g.frame.tangent = 0.3;
        }
        let mut rim = scene(MetalFxResolvedTheme::Dark, false);
        rim.ring = 3.0;
        rim.rim = Some(MetalRimOptions::new());
        rim.density = 2.0;
        let cases: Vec<(&str, Scene, Layer)> = vec![
            (
                "ring dark button",
                scene(MetalFxResolvedTheme::Dark, false),
                Layer::Ring,
            ),
            ("ring dark button 2x", hi, Layer::Ring),
            (
                "ring light circle",
                scene(MetalFxResolvedTheme::Light, true),
                Layer::Ring,
            ),
            (
                "ring dark circle",
                scene(MetalFxResolvedTheme::Dark, true),
                Layer::Ring,
            ),
            (
                "surface",
                scene(MetalFxResolvedTheme::Dark, false),
                Layer::Surface,
            ),
            (
                "glow dark",
                glow_scene(MetalFxResolvedTheme::Dark),
                Layer::Glow,
            ),
            ("glow light (multiply + filter)", light_glow, Layer::Glow),
            ("rim 2x", rim, Layer::Rim),
        ];

        for (name, scene, layer) in &cases {
            let (max_err, share) = compare(&gpu, &mut pipeline, scene, *layer, 5.0);
            assert!(
                share > 0.985,
                "{name}: only {:.2}% of pixels within 5/255 (max err {max_err})",
                share * 100.0
            );
            eprintln!(
                "metal_fx parity {name}: {:.2}% within 5/255, max err {max_err:.1}",
                share * 100.0
            );
        }
    }

    #[test]
    fn slots_are_recycled_per_frame() {
        let Some(gpu) = gpu() else {
            eprintln!("metal_fx slot test skipped: no wgpu adapter available on this machine");
            return;
        };
        let mut pipeline =
            MetalFxPipeline::new(&gpu.device, &gpu.queue, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert!(pipeline.target_is_srgb());
        let s = glow_scene(MetalFxResolvedTheme::Dark);
        for _ in 0..3 {
            let a = pipeline.prepare(&gpu.device, &gpu.queue, &s, Layer::Ring, 1.0);
            let b = pipeline.prepare(&gpu.device, &gpu.queue, &s, Layer::Glow, 1.0);
            assert_eq!((a, b), (0, 1));
            assert_eq!(
                pipeline.sprite_count(),
                2,
                "both sprites resident, shared by id"
            );
            shader::Pipeline::trim(&mut pipeline);
        }
        assert_eq!(pipeline.slots.len(), 2);
    }
}
