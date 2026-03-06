use bytemuck::{Pod, Zeroable};

use crate::gpu::GpuContext;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RectVertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

pub struct RectData {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
}

pub struct RectRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertices: Vec<RectVertex>,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_capacity: usize,
    vertex_count: u32,
}

impl RectRenderer {
    pub fn new(gpu: &GpuContext, target_format: wgpu::TextureFormat) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("rect_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
            });

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect_uniform"),
            size: std::mem::size_of::<ScreenUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("rect_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let uniform_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rect_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rect_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("rect_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RectVertex>() as u64,
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
            uniform_bind_group,
            vertices: Vec::new(),
            vertex_buffer: None,
            vertex_capacity: 0,
            vertex_count: 0,
        }
    }

    pub fn prepare(&mut self, gpu: &GpuContext, width: u32, height: u32, rects: &[RectData]) {
        let uniform = ScreenUniform {
            screen_size: [width as f32, height as f32],
            _padding: [0.0; 2],
        };
        gpu.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        self.vertices.clear();
        self.vertices.reserve(rects.len() * 6);
        for r in rects {
            let x0 = r.x;
            let y0 = r.y;
            let x1 = r.x + r.width;
            let y1 = r.y + r.height;
            let c = r.color;

            self.vertices.push(RectVertex {
                position: [x0, y0],
                color: c,
            });
            self.vertices.push(RectVertex {
                position: [x1, y0],
                color: c,
            });
            self.vertices.push(RectVertex {
                position: [x0, y1],
                color: c,
            });
            self.vertices.push(RectVertex {
                position: [x0, y1],
                color: c,
            });
            self.vertices.push(RectVertex {
                position: [x1, y0],
                color: c,
            });
            self.vertices.push(RectVertex {
                position: [x1, y1],
                color: c,
            });
        }

        self.vertex_count = self.vertices.len() as u32;
        if self.vertices.is_empty() {
            return;
        }

        let needed_vertices = self.vertices.len();
        if needed_vertices > self.vertex_capacity {
            let capacity = needed_vertices.next_power_of_two();
            let size = (capacity * std::mem::size_of::<RectVertex>()) as u64;
            self.vertex_buffer = Some(gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rect_vertices"),
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
        if self.vertex_count == 0 {
            return;
        }
        if let Some(ref vb) = self.vertex_buffer {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.draw(0..self.vertex_count, 0..1);
        }
    }
}
