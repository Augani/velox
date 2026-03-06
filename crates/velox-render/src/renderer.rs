use velox_scene::{Color, CommandList, PaintCommand, Rect, TextureId};

use crate::glyph_atlas::GlyphAtlas;
use crate::glyph_renderer::{GlyphQuad, GlyphRenderer};
use crate::gpu::GpuContext;
use crate::image_renderer::{ImageQuad, ImageRenderer};
use crate::rect_renderer::{RectData, RectRenderer};
use crate::surface::WindowSurface;
use crate::texture_manager::TextureManager;

pub struct Renderer {
    rect_renderer: RectRenderer,
    glyph_renderer: GlyphRenderer,
    image_renderer: ImageRenderer,
    texture_manager: TextureManager,
    rect_scratch: Vec<RectData>,
    glyph_scratch: Vec<GlyphQuad>,
    image_scratch: Vec<(TextureId, ImageQuad)>,
    cached_command_epoch: Option<u64>,
    cached_surface_size: Option<(u32, u32)>,
}

impl Renderer {
    pub fn new(gpu: &GpuContext, target_format: wgpu::TextureFormat) -> Self {
        Self {
            rect_renderer: RectRenderer::new(gpu, target_format),
            glyph_renderer: GlyphRenderer::new(gpu, target_format),
            image_renderer: ImageRenderer::new(gpu, target_format),
            texture_manager: TextureManager::new(256 * 1024 * 1024),
            rect_scratch: Vec::new(),
            glyph_scratch: Vec::new(),
            image_scratch: Vec::new(),
            cached_command_epoch: None,
            cached_surface_size: None,
        }
    }

    pub fn texture_manager(&mut self) -> &mut TextureManager {
        &mut self.texture_manager
    }

    pub fn render(
        &mut self,
        gpu: &GpuContext,
        surface: &WindowSurface,
        commands: &CommandList,
        atlas: &mut GlyphAtlas,
    ) -> Result<(), wgpu::SurfaceError> {
        let surface_size = (surface.width(), surface.height());
        let command_epoch = commands.epoch();
        let commands_changed = self.cached_command_epoch != Some(command_epoch);
        let surface_changed = self.cached_surface_size != Some(surface_size);

        if commands_changed {
            for upload in commands.glyph_uploads() {
                atlas.insert(upload.cache_key, upload.width, upload.height, &upload.data);
            }
        }

        if commands_changed || surface_changed {
            self.rect_scratch.clear();
            self.glyph_scratch.clear();
            self.image_scratch.clear();
            let mut clip_stack: Vec<Option<Rect>> = Vec::new();

            for cmd in commands.commands() {
                match cmd {
                    PaintCommand::FillRect { rect, color } => {
                        let clip = match clip_stack.last().copied() {
                            Some(None) => continue,
                            Some(Some(active)) => {
                                let Some(clip) =
                                    rect_to_scissor(active, surface.width(), surface.height())
                                else {
                                    continue;
                                };
                                Some(clip)
                            }
                            None => None,
                        };

                        self.rect_scratch.push(RectData {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: rect.height,
                            color: color_to_f32(color),
                            clip,
                        });
                    }
                    PaintCommand::StrokeRect { rect, color, width } => {
                        let w = *width;
                        if w <= 0.0 {
                            continue;
                        }

                        let clip = match clip_stack.last().copied() {
                            Some(None) => continue,
                            Some(Some(active)) => {
                                let Some(clip) =
                                    rect_to_scissor(active, surface.width(), surface.height())
                                else {
                                    continue;
                                };
                                Some(clip)
                            }
                            None => None,
                        };

                        let c = color_to_f32(color);
                        self.rect_scratch.push(RectData {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: w,
                            color: c,
                            clip,
                        });
                        self.rect_scratch.push(RectData {
                            x: rect.x,
                            y: rect.y + rect.height - w,
                            width: rect.width,
                            height: w,
                            color: c,
                            clip,
                        });
                        self.rect_scratch.push(RectData {
                            x: rect.x,
                            y: rect.y + w,
                            width: w,
                            height: rect.height - 2.0 * w,
                            color: c,
                            clip,
                        });
                        self.rect_scratch.push(RectData {
                            x: rect.x + rect.width - w,
                            y: rect.y + w,
                            width: w,
                            height: rect.height - 2.0 * w,
                            color: c,
                            clip,
                        });
                    }
                    PaintCommand::DrawGlyphs { glyphs, color } => {
                        let clip = match clip_stack.last().copied() {
                            Some(None) => continue,
                            Some(Some(active)) => {
                                let Some(clip) =
                                    rect_to_scissor(active, surface.width(), surface.height())
                                else {
                                    continue;
                                };
                                Some(clip)
                            }
                            None => None,
                        };

                        let c = color_to_f32(color);
                        for glyph in glyphs {
                            if let Some(region) = atlas.get(&glyph.cache_key) {
                                let uv = atlas.uv(region);
                                self.glyph_scratch.push(GlyphQuad {
                                    x: glyph.x,
                                    y: glyph.y,
                                    width: glyph.width,
                                    height: glyph.height,
                                    uv,
                                    color: c,
                                    clip,
                                });
                            }
                        }
                    }
                    PaintCommand::DrawImage {
                        texture_id,
                        src_rect,
                        dst_rect,
                        opacity,
                    } => {
                        let clip = match clip_stack.last().copied() {
                            Some(None) => continue,
                            Some(Some(active)) => {
                                let Some(clip) =
                                    rect_to_scissor(active, surface.width(), surface.height())
                                else {
                                    continue;
                                };
                                Some(clip)
                            }
                            None => None,
                        };

                        self.image_scratch.push((
                            *texture_id,
                            ImageQuad {
                                x: dst_rect.x,
                                y: dst_rect.y,
                                width: dst_rect.width,
                                height: dst_rect.height,
                                uv: [
                                    src_rect.x,
                                    src_rect.y,
                                    src_rect.x + src_rect.width,
                                    src_rect.y + src_rect.height,
                                ],
                                opacity: *opacity,
                                clip,
                            },
                        ));
                    }
                    PaintCommand::PushClip(rect) => {
                        let next_clip = match clip_stack.last().copied() {
                            None => Some(*rect),
                            Some(None) => None,
                            Some(Some(current)) => intersect_rect(current, *rect),
                        };
                        clip_stack.push(next_clip);
                    }
                    PaintCommand::PopClip => {
                        let _ = clip_stack.pop();
                    }
                    PaintCommand::PushLayer { .. } => {}
                    PaintCommand::PopLayer => {}
                    PaintCommand::BoxShadow { .. } => {}
                    PaintCommand::FillGradient { .. } => {}
                }
            }
        }

        if commands_changed || surface_changed {
            self.rect_renderer
                .prepare(gpu, surface.width(), surface.height(), &self.rect_scratch);

            self.glyph_renderer.prepare(
                gpu,
                surface.width(),
                surface.height(),
                &self.glyph_scratch,
            );
        }

        if atlas.is_dirty() {
            self.glyph_renderer.upload_atlas(gpu, atlas);
            atlas.clear_dirty();
        }

        self.texture_manager.tick_frame();

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

        self.render_image_batches(gpu, surface, &view, &mut encoder);

        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.cached_command_epoch = Some(command_epoch);
        self.cached_surface_size = Some(surface_size);

        Ok(())
    }

    fn render_image_batches(
        &mut self,
        gpu: &GpuContext,
        surface: &WindowSurface,
        target_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if self.image_scratch.is_empty() {
            return;
        }

        let mut current_texture: Option<TextureId> = None;
        let mut batch: Vec<ImageQuad> = Vec::new();
        let scratch = std::mem::take(&mut self.image_scratch);

        for (tex_id, quad) in &scratch {
            if current_texture != Some(*tex_id) {
                if !batch.is_empty() {
                    self.flush_image_batch(gpu, surface, target_view, encoder, &batch);
                    batch.clear();
                }

                if let Some(view) = self.texture_manager.get_view(*tex_id) {
                    self.image_renderer.bind_texture(gpu, view);
                    current_texture = Some(*tex_id);
                } else {
                    current_texture = None;
                    continue;
                }
            }

            batch.push(ImageQuad {
                x: quad.x,
                y: quad.y,
                width: quad.width,
                height: quad.height,
                uv: quad.uv,
                opacity: quad.opacity,
                clip: quad.clip,
            });
        }

        if !batch.is_empty() {
            self.flush_image_batch(gpu, surface, target_view, encoder, &batch);
        }

        self.image_scratch = scratch;
    }

    fn flush_image_batch(
        &mut self,
        gpu: &GpuContext,
        surface: &WindowSurface,
        target_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        batch: &[ImageQuad],
    ) {
        self.image_renderer
            .prepare(gpu, surface.width(), surface.height(), batch);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("velox_image_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.image_renderer.render(&mut pass);
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

fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.width).min(b.x + b.width);
    let y1 = (a.y + a.height).min(b.y + b.height);
    let w = x1 - x0;
    let h = y1 - y0;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some(Rect::new(x0, y0, w, h))
}

fn rect_to_scissor(rect: Rect, surface_width: u32, surface_height: u32) -> Option<[u32; 4]> {
    let sw = surface_width as f32;
    let sh = surface_height as f32;

    let x0 = rect.x.max(0.0).floor();
    let y0 = rect.y.max(0.0).floor();
    let x1 = (rect.x + rect.width).min(sw).ceil();
    let y1 = (rect.y + rect.height).min(sh).ceil();

    if x1 <= x0 || y1 <= y0 {
        return None;
    }

    Some([x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}
