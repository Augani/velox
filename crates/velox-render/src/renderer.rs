use velox_scene::{Color, CommandList, PaintCommand};

use crate::glyph_atlas::GlyphAtlas;
use crate::glyph_renderer::GlyphRenderer;
use crate::gpu::GpuContext;
use crate::rect_renderer::{RectData, RectRenderer};
use crate::surface::WindowSurface;

pub struct Renderer {
    rect_renderer: RectRenderer,
    glyph_renderer: GlyphRenderer,
}

impl Renderer {
    pub fn new(gpu: &GpuContext, target_format: wgpu::TextureFormat) -> Self {
        Self {
            rect_renderer: RectRenderer::new(gpu, target_format),
            glyph_renderer: GlyphRenderer::new(gpu, target_format),
        }
    }

    pub fn render(
        &mut self,
        gpu: &GpuContext,
        surface: &WindowSurface,
        commands: &CommandList,
        atlas: &mut GlyphAtlas,
    ) -> Result<(), wgpu::SurfaceError> {
        let mut rects = Vec::new();

        for cmd in commands.commands() {
            match cmd {
                PaintCommand::FillRect { rect, color } => {
                    rects.push(RectData {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height,
                        color: color_to_f32(color),
                    });
                }
                PaintCommand::StrokeRect { rect, color, width } => {
                    let w = *width;
                    let c = color_to_f32(color);
                    rects.push(RectData {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: w,
                        color: c,
                    });
                    rects.push(RectData {
                        x: rect.x,
                        y: rect.y + rect.height - w,
                        width: rect.width,
                        height: w,
                        color: c,
                    });
                    rects.push(RectData {
                        x: rect.x,
                        y: rect.y + w,
                        width: w,
                        height: rect.height - 2.0 * w,
                        color: c,
                    });
                    rects.push(RectData {
                        x: rect.x + rect.width - w,
                        y: rect.y + w,
                        width: w,
                        height: rect.height - 2.0 * w,
                        color: c,
                    });
                }
                PaintCommand::PushClip(_) | PaintCommand::PopClip => {}
            }
        }

        self.rect_renderer
            .prepare(gpu, surface.width(), surface.height(), &rects);

        if atlas.is_dirty() {
            self.glyph_renderer.upload_atlas(gpu, atlas);
            atlas.clear_dirty();
        }

        let output = surface.surface().get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("velox_render"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velox_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.rect_renderer.render(&mut render_pass);
            self.glyph_renderer.render(&mut render_pass);
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn color_to_f32(c: &Color) -> [f32; 4] {
    [
        c.r as f32 / 255.0,
        c.g as f32 / 255.0,
        c.b as f32 / 255.0,
        c.a as f32 / 255.0,
    ]
}
