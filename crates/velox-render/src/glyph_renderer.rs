use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::glyph_atlas::GlyphAtlas;
use crate::gpu::GpuContext;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GlyphVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

pub struct GlyphQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv: [f32; 4],
    pub color: [f32; 4],
}

pub struct GlyphRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    vertices: Vec<GlyphVertex>,
    vertex_buffer: Option<wgpu::Buffer>,
}

impl GlyphRenderer {
    pub fn new(gpu: &GpuContext, target_format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("glyph_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/glyph.wgsl").into()),
            });

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph_uniform"),
            size: std::mem::size_of::<ScreenUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("glyph_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
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

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("glyph_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("glyph_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GlyphVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 8,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        Self {
            pipeline,
            uniform_buffer,
            bind_group_layout,
            bind_group: None,
            vertices: Vec::new(),
            vertex_buffer: None,
        }
    }

    pub fn upload_atlas(&mut self, gpu: &GpuContext, atlas: &GlyphAtlas) {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas_texture"),
            size: wgpu::Extent3d {
                width: atlas.width(),
                height: atlas.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas.texture_data(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.width()),
                rows_per_image: Some(atlas.height()),
            },
            wgpu::Extent3d {
                width: atlas.width(),
                height: atlas.height(),
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph_atlas_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        self.bind_group = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        }));
    }

    pub fn prepare(&mut self, gpu: &GpuContext, width: u32, height: u32, quads: &[GlyphQuad]) {
        let uniform = ScreenUniform {
            screen_size: [width as f32, height as f32],
            _padding: [0.0; 2],
        };
        gpu.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        self.vertices.clear();
        for q in quads {
            let x0 = q.x;
            let y0 = q.y;
            let x1 = q.x + q.width;
            let y1 = q.y + q.height;
            let [u0, v0, u1, v1] = q.uv;
            let c = q.color;

            self.vertices.push(GlyphVertex {
                position: [x0, y0],
                uv: [u0, v0],
                color: c,
            });
            self.vertices.push(GlyphVertex {
                position: [x1, y0],
                uv: [u1, v0],
                color: c,
            });
            self.vertices.push(GlyphVertex {
                position: [x0, y1],
                uv: [u0, v1],
                color: c,
            });
            self.vertices.push(GlyphVertex {
                position: [x0, y1],
                uv: [u0, v1],
                color: c,
            });
            self.vertices.push(GlyphVertex {
                position: [x1, y0],
                uv: [u1, v0],
                color: c,
            });
            self.vertices.push(GlyphVertex {
                position: [x1, y1],
                uv: [u1, v1],
                color: c,
            });
        }

        self.vertex_buffer = if self.vertices.is_empty() {
            None
        } else {
            Some(
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("glyph_vertices"),
                        contents: bytemuck::cast_slice(&self.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
            )
        };
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        let (Some(vb), Some(bg)) = (&self.vertex_buffer, &self.bind_group) else {
            return;
        };
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bg, &[]);
        render_pass.set_vertex_buffer(0, vb.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}
