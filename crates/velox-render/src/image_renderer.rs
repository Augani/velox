use bytemuck::{Pod, Zeroable};

use crate::gpu::GpuContext;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ImageVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

pub struct ImageQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv: [f32; 4],
    pub opacity: f32,
    pub clip: Option<[u32; 4]>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct DrawBatch {
    start: u32,
    count: u32,
    clip: Option<[u32; 4]>,
}

pub struct ImageRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    vertices: Vec<ImageVertex>,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_capacity: usize,
    vertex_count: u32,
    draw_batches: Vec<DrawBatch>,
    surface_width: u32,
    surface_height: u32,
}

impl ImageRenderer {
    pub fn new(gpu: &GpuContext, target_format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("image_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image.wgsl").into()),
            });

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image_uniform"),
            size: std::mem::size_of::<ScreenUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("image_bind_group_layout"),
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
                label: Some("image_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("image_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ImageVertex>() as u64,
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
                                format: wgpu::VertexFormat::Float32,
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

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group_layout,
            bind_group: None,
            sampler,
            vertices: Vec::new(),
            vertex_buffer: None,
            vertex_capacity: 0,
            vertex_count: 0,
            draw_batches: Vec::new(),
            surface_width: 1,
            surface_height: 1,
        }
    }

    pub fn bind_texture(&mut self, gpu: &GpuContext, view: &wgpu::TextureView) {
        self.bind_group = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
    }

    pub fn prepare(
        &mut self,
        gpu: &GpuContext,
        surface_width: u32,
        surface_height: u32,
        quads: &[ImageQuad],
    ) {
        self.surface_width = surface_width.max(1);
        self.surface_height = surface_height.max(1);
        let uniform = ScreenUniform {
            screen_size: [surface_width as f32, surface_height as f32],
            _padding: [0.0; 2],
        };
        gpu.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        self.draw_batches.clear();
        self.vertices.clear();
        self.vertices.reserve(quads.len() * 6);

        for q in quads {
            if q.width <= 0.0 || q.height <= 0.0 {
                continue;
            }

            let start = self.vertices.len() as u32;
            let x0 = q.x;
            let y0 = q.y;
            let x1 = q.x + q.width;
            let y1 = q.y + q.height;
            let [u0, v0, u1, v1] = q.uv;
            let op = q.opacity;

            self.vertices.push(ImageVertex {
                position: [x0, y0],
                uv: [u0, v0],
                opacity: op,
            });
            self.vertices.push(ImageVertex {
                position: [x1, y0],
                uv: [u1, v0],
                opacity: op,
            });
            self.vertices.push(ImageVertex {
                position: [x0, y1],
                uv: [u0, v1],
                opacity: op,
            });
            self.vertices.push(ImageVertex {
                position: [x0, y1],
                uv: [u0, v1],
                opacity: op,
            });
            self.vertices.push(ImageVertex {
                position: [x1, y0],
                uv: [u1, v0],
                opacity: op,
            });
            self.vertices.push(ImageVertex {
                position: [x1, y1],
                uv: [u1, v1],
                opacity: op,
            });

            self.push_batch(start, 6, q.clip);
        }

        self.vertex_count = self.vertices.len() as u32;
        if self.vertices.is_empty() {
            return;
        }

        let needed = self.vertices.len();
        if needed > self.vertex_capacity {
            let capacity = needed.next_power_of_two();
            let size = (capacity * std::mem::size_of::<ImageVertex>()) as u64;
            self.vertex_buffer = Some(gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("image_vertices"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertex_capacity = capacity;
        }

        if let Some(vb) = &self.vertex_buffer {
            gpu.queue
                .write_buffer(vb, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.vertex_count == 0 || self.draw_batches.is_empty() {
            return;
        }
        let (Some(vb), Some(bg)) = (&self.vertex_buffer, &self.bind_group) else {
            return;
        };
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bg, &[]);
        render_pass.set_vertex_buffer(0, vb.slice(..));
        for batch in &self.draw_batches {
            let [x, y, w, h] = batch
                .clip
                .unwrap_or([0, 0, self.surface_width, self.surface_height]);
            if w == 0 || h == 0 {
                continue;
            }
            render_pass.set_scissor_rect(x, y, w, h);
            render_pass.draw(batch.start..(batch.start + batch.count), 0..1);
        }
    }

    fn push_batch(&mut self, start: u32, count: u32, clip: Option<[u32; 4]>) {
        if count == 0 {
            return;
        }

        if let Some(last) = self.draw_batches.last_mut()
            && last.clip == clip
            && last.start + last.count == start
        {
            last.count += count;
            return;
        }

        self.draw_batches.push(DrawBatch { start, count, clip });
    }
}
