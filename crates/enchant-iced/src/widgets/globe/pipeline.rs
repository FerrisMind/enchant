use crate::widgets::globe::{GlobeMarker, GlobeProps, GlobeState};
use iced::Rectangle;
use iced::wgpu;
use iced::wgpu::util::DeviceExt;
use iced::widget::shader::Viewport;

const COBE_MAP: &[u8] = include_bytes!("../../../assets/cobe_map.png");

#[repr(C, align(16))]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlobeUniforms {
    pub resolution: [f32; 2],
    pub offset: [f32; 2],
    pub rotation: [f32; 2],
    pub dots: f32,
    pub scale: f32,
    pub map_base_brightness: f32,
    pub padding0: f32,
    /// WGSL aligns `vec4` fields to 16 bytes (offset 48).
    pub _vec4_align: [f32; 2],
    pub base_color: [f32; 4],
    pub glow_color: [f32; 4],
    pub render_params: [f32; 4],
    pub _struct_pad: [f32; 4],
}

/// WGSL uniform block size for `GlobeUniforms` (must match shader layout).
const GLOBE_UNIFORM_SIZE: u64 = 112;

const _: () = assert!(std::mem::size_of::<GlobeUniforms>() as u64 == GLOBE_UNIFORM_SIZE);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MarkerUniforms {
    pub phi: f32,
    pub theta: f32,
    pub resolution: [f32; 2],
    pub scale: f32,
    pub marker_elevation: f32,
    pub offset: [f32; 2],
    pub marker_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MarkerInstance {
    pub pos: [f32; 3],
    pub size: f32,
    pub color: [f32; 3],
    pub has_color: f32,
}

pub struct GlobePipeline {
    globe_pipeline: wgpu::RenderPipeline,
    marker_pipeline: wgpu::RenderPipeline,
    globe_bind_group: wgpu::BindGroup,
    globe_uniform_buffer: wgpu::Buffer,
    marker_uniform_buffer: wgpu::Buffer,
    marker_bind_group: wgpu::BindGroup,
    quad_buffer: wgpu::Buffer,
    marker_instance_buffer: wgpu::Buffer,
    _map_texture: wgpu::Texture,
    _map_view: wgpu::TextureView,
    _map_sampler: wgpu::Sampler,
    marker_capacity: usize,
    pending_globe: GlobeUniforms,
    pending_marker: MarkerUniforms,
    pending_markers: Vec<MarkerInstance>,
    marker_count: u32,
}

impl GlobePipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let globe_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globe.uniforms"),
            size: GLOBE_UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let marker_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globe.marker.uniforms"),
            size: uniform_buffer_size::<MarkerUniforms>(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let map_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("globe.map.sampler"),
            // COBE map UVs can be < 0 on V; Repeat matches WebGL `REPEAT` wrap.
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let rgba = image::load_from_memory(COBE_MAP)
            .expect("cobe map png")
            .to_rgba8();
        let (width, height) = rgba.dimensions();

        let map_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("globe.map.texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &map_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let map_view = map_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let globe_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globe.bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(
                                wgpu::BufferSize::new(GLOBE_UNIFORM_SIZE)
                                    .expect("globe uniform size"),
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let globe_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globe.bind_group"),
            layout: &globe_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globe_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&map_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&map_sampler),
                },
            ],
        });

        let marker_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globe.marker.bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            wgpu::BufferSize::new(uniform_buffer_size::<MarkerUniforms>())
                                .expect("marker uniform size"),
                        ),
                    },
                    count: None,
                }],
            });

        let marker_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globe.marker.bind_group"),
            layout: &marker_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: marker_uniform_buffer.as_entire_binding(),
            }],
        });

        let globe_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("globe.shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/globe.wgsl"
            ))),
        });

        let marker_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("globe.marker.shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/marker.wgsl"
            ))),
        });

        let globe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("globe.pipeline_layout"),
            bind_group_layouts: &[&globe_bind_group_layout],
            push_constant_ranges: &[],
        });

        let marker_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("globe.marker.pipeline_layout"),
            bind_group_layouts: &[&marker_bind_group_layout],
            push_constant_ranges: &[],
        });

        let quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globe.quad"),
            contents: bytemuck::cast_slice(&[
                -1.0_f32, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let marker_capacity = 32usize;
        let marker_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globe.marker.instances"),
            size: (marker_capacity * std::mem::size_of::<MarkerInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let globe_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("globe.pipeline"),
            layout: Some(&globe_layout),
            vertex: wgpu::VertexState {
                module: &globe_shader,
                entry_point: Some("vs_globe"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &globe_shader,
                entry_point: Some("fs_globe"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        let marker_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("globe.marker.pipeline"),
            layout: Some(&marker_layout),
            vertex: wgpu::VertexState {
                module: &marker_shader,
                entry_point: Some("vs_marker"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 8,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MarkerInstance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 12,
                                shader_location: 2,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 16,
                                shader_location: 3,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 28,
                                shader_location: 4,
                            },
                        ],
                    },
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &marker_shader,
                entry_point: Some("fs_marker"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        Self {
            globe_pipeline,
            marker_pipeline,
            globe_bind_group,
            globe_uniform_buffer,
            marker_uniform_buffer,
            marker_bind_group,
            quad_buffer,
            marker_instance_buffer,
            _map_texture: map_texture,
            _map_view: map_view,
            _map_sampler: map_sampler,
            marker_capacity,
            pending_globe: bytemuck::Zeroable::zeroed(),
            pending_marker: bytemuck::Zeroable::zeroed(),
            pending_markers: Vec::new(),
            marker_count: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: &Viewport,
        state: &GlobeState,
        props: &GlobeProps,
        bounds: Rectangle,
        _dpr: f32,
    ) {
        let scale_factor = viewport.scale_factor();
        let width = (bounds.width * scale_factor).max(1.0);
        let height = (bounds.height * scale_factor).max(1.0);

        let base = color_to_vec4(
            props
                .base_color
                .unwrap_or(crate::widgets::globe::DEFAULT_BASE),
        );
        let glow = color_to_vec4(
            props
                .glow_color
                .unwrap_or(crate::widgets::globe::DEFAULT_GLOW),
        );
        let opacity = state.opacity;

        self.pending_globe = GlobeUniforms {
            resolution: [width, height],
            offset: [
                props.offset[0] * scale_factor,
                props.offset[1] * scale_factor,
            ],
            rotation: [state.phi, state.theta],
            dots: props.map_samples as f32,
            scale: props.scale,
            map_base_brightness: props.map_base_brightness,
            padding0: if props.outer_glow { 1.0 } else { 0.0 },
            _vec4_align: [0.0; 2],
            base_color: base,
            glow_color: glow,
            render_params: [props.map_brightness, props.diffuse, props.dark, opacity],
            _struct_pad: [0.0; 4],
        };

        let marker_color = color_to_vec4(
            props
                .marker_color
                .unwrap_or(crate::widgets::globe::DEFAULT_MARKER),
        );

        self.pending_marker = MarkerUniforms {
            phi: state.phi,
            theta: state.theta,
            resolution: [width, height],
            scale: props.scale,
            marker_elevation: props.marker_elevation,
            offset: [
                props.offset[0] * scale_factor,
                props.offset[1] * scale_factor,
            ],
            marker_color,
        };

        self.pending_markers = props.markers.iter().map(marker_instance).collect();
        self.marker_count = self.pending_markers.len() as u32;

        if self.pending_markers.len() > self.marker_capacity {
            self.marker_capacity = self.pending_markers.len().next_power_of_two().max(8);
            self.marker_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("globe.marker.instances"),
                size: (self.marker_capacity * std::mem::size_of::<MarkerInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(
            &self.globe_uniform_buffer,
            0,
            bytemuck::bytes_of(&self.pending_globe),
        );
        queue.write_buffer(
            &self.marker_uniform_buffer,
            0,
            bytemuck::bytes_of(&self.pending_marker),
        );
        if !self.pending_markers.is_empty() {
            queue.write_buffer(
                &self.marker_instance_buffer,
                0,
                bytemuck::cast_slice(&self.pending_markers),
            );
        }

        let _ = device;
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.globe_pipeline);
        render_pass.set_bind_group(0, &self.globe_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
        render_pass.draw(0..6, 0..1);

        if self.marker_count > 0 {
            render_pass.set_pipeline(&self.marker_pipeline);
            render_pass.set_bind_group(0, &self.marker_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.marker_instance_buffer.slice(..));
            render_pass.draw(0..6, 0..self.marker_count);
        }
    }
}

fn uniform_buffer_size<T>() -> u64 {
    let size = std::mem::size_of::<T>() as u64;
    size.div_ceil(16) * 16
}

fn color_to_vec4(color: iced::Color) -> [f32; 4] {
    color.into_linear()
}

fn marker_instance(marker: &GlobeMarker) -> MarkerInstance {
    let pos = lat_lon_to_unit(marker.lat, marker.lng);
    MarkerInstance {
        pos,
        size: marker.size,
        color: [0.0, 0.0, 0.0],
        has_color: 0.0,
    }
}

/// COBE `latLonTo3D` from `cobe/src/index.js`.
pub fn lat_lon_to_unit(lat: f32, lng: f32) -> [f32; 3] {
    let lat_rad = lat.to_radians();
    let lon_rad = lng.to_radians() - std::f32::consts::PI;
    let cos_lat = lat_rad.cos();
    [
        -cos_lat * lon_rad.cos(),
        lat_rad.sin(),
        cos_lat * lon_rad.sin(),
    ]
}
