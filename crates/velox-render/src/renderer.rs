use velox_scene::{Color, CommandList, Gradient, PaintCommand, Rect, TextureId};

use crate::glyph_atlas::GlyphAtlas;
use crate::glyph_renderer::{GlyphQuad, GlyphRenderer};
use crate::gpu::GpuContext;
use crate::image_renderer::{ImageQuad, ImageRenderer};
use crate::paint_utils::{
    effective_layer_opacity, intersect_rect, modulate_color, sample_gradient_color, shadow_layers,
};
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
        scale_factor: f32,
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

        let sf = scale_factor;

        if commands_changed || surface_changed {
            self.rect_scratch.clear();
            self.glyph_scratch.clear();
            self.image_scratch.clear();
            let mut clip_stack: Vec<Option<Rect>> = Vec::new();
            let mut layer_stack: Vec<f32> = Vec::new();

            for cmd in commands.commands() {
                match cmd {
                    PaintCommand::FillRect { rect, color } => {
                        let Some(clip) =
                            current_clip_scissor(&clip_stack, surface.width(), surface.height())
                        else {
                            continue;
                        };

                        push_solid_rect(
                            &mut self.rect_scratch,
                            *rect,
                            *color,
                            0.0,
                            clip,
                            sf,
                            effective_layer_opacity(&layer_stack),
                        );
                    }
                    PaintCommand::StrokeRect { rect, color, width } => {
                        let w = *width * sf;
                        if w <= 0.0 {
                            continue;
                        }

                        let Some(clip) =
                            current_clip_scissor(&clip_stack, surface.width(), surface.height())
                        else {
                            continue;
                        };

                        let opacity = effective_layer_opacity(&layer_stack);
                        let sx = rect.x * sf;
                        let sy = rect.y * sf;
                        let sw = rect.width * sf;
                        let sh = rect.height * sf;
                        let stroke_color = modulate_color(*color, opacity);
                        let stroke_color = color_to_f32(&stroke_color);
                        self.rect_scratch.push(RectData {
                            x: sx,
                            y: sy,
                            width: sw,
                            height: w,
                            color: stroke_color,
                            corner_radius: 0.0,
                            clip,
                        });
                        self.rect_scratch.push(RectData {
                            x: sx,
                            y: sy + sh - w,
                            width: sw,
                            height: w,
                            color: stroke_color,
                            corner_radius: 0.0,
                            clip,
                        });
                        self.rect_scratch.push(RectData {
                            x: sx,
                            y: sy + w,
                            width: w,
                            height: sh - 2.0 * w,
                            color: stroke_color,
                            corner_radius: 0.0,
                            clip,
                        });
                        self.rect_scratch.push(RectData {
                            x: sx + sw - w,
                            y: sy + w,
                            width: w,
                            height: sh - 2.0 * w,
                            color: stroke_color,
                            corner_radius: 0.0,
                            clip,
                        });
                    }
                    PaintCommand::DrawGlyphs { glyphs, color } => {
                        let Some(clip) =
                            current_clip_scissor(&clip_stack, surface.width(), surface.height())
                        else {
                            continue;
                        };

                        let c = color_to_f32(&modulate_color(
                            *color,
                            effective_layer_opacity(&layer_stack),
                        ));
                        for glyph in glyphs {
                            if let Some(region) = atlas.get(&glyph.cache_key) {
                                let uv = atlas.uv(region);
                                self.glyph_scratch.push(GlyphQuad {
                                    x: glyph.x * sf,
                                    y: glyph.y * sf,
                                    width: glyph.width * sf,
                                    height: glyph.height * sf,
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
                        let Some(clip) =
                            current_clip_scissor(&clip_stack, surface.width(), surface.height())
                        else {
                            continue;
                        };
                        let opacity =
                            (*opacity * effective_layer_opacity(&layer_stack)).clamp(0.0, 1.0);
                        if opacity <= f32::EPSILON {
                            continue;
                        }

                        self.image_scratch.push((
                            *texture_id,
                            ImageQuad {
                                x: dst_rect.x * sf,
                                y: dst_rect.y * sf,
                                width: dst_rect.width * sf,
                                height: dst_rect.height * sf,
                                uv: [
                                    src_rect.x,
                                    src_rect.y,
                                    src_rect.x + src_rect.width,
                                    src_rect.y + src_rect.height,
                                ],
                                opacity,
                                clip,
                            },
                        ));
                    }
                    PaintCommand::FillRoundedRect {
                        rect,
                        color,
                        corner_radius,
                    } => {
                        let Some(clip) =
                            current_clip_scissor(&clip_stack, surface.width(), surface.height())
                        else {
                            continue;
                        };

                        push_solid_rect(
                            &mut self.rect_scratch,
                            *rect,
                            *color,
                            *corner_radius,
                            clip,
                            sf,
                            effective_layer_opacity(&layer_stack),
                        );
                    }
                    PaintCommand::PushClip(rect) => {
                        let scaled =
                            Rect::new(rect.x * sf, rect.y * sf, rect.width * sf, rect.height * sf);
                        let next_clip = match clip_stack.last().copied() {
                            None => Some(scaled),
                            Some(None) => None,
                            Some(Some(current)) => intersect_rect(current, scaled),
                        };
                        clip_stack.push(next_clip);
                    }
                    PaintCommand::PopClip => {
                        let _ = clip_stack.pop();
                    }
                    PaintCommand::PushLayer {
                        opacity,
                        blend_mode: _,
                    } => {
                        layer_stack.push(opacity.clamp(0.0, 1.0));
                    }
                    PaintCommand::PopLayer => {
                        let _ = layer_stack.pop();
                    }
                    PaintCommand::BoxShadow {
                        rect,
                        color,
                        blur_radius,
                        offset,
                        spread,
                    } => {
                        let Some(clip) =
                            current_clip_scissor(&clip_stack, surface.width(), surface.height())
                        else {
                            continue;
                        };

                        emit_shadow_rects(
                            &mut self.rect_scratch,
                            *rect,
                            *color,
                            *blur_radius,
                            *offset,
                            *spread,
                            clip,
                            sf,
                            effective_layer_opacity(&layer_stack),
                        );
                    }
                    PaintCommand::FillGradient { rect, gradient } => {
                        let Some(clip) =
                            current_clip_scissor(&clip_stack, surface.width(), surface.height())
                        else {
                            continue;
                        };

                        emit_gradient_rects(
                            &mut self.rect_scratch,
                            *rect,
                            gradient,
                            clip,
                            sf,
                            effective_layer_opacity(&layer_stack),
                        );
                    }
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
                            r: srgb_to_linear(1.0) as f64,
                            g: srgb_to_linear(1.0) as f64,
                            b: srgb_to_linear(1.0) as f64,
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

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn color_to_f32(c: &Color) -> [f32; 4] {
    [
        srgb_to_linear(c.r as f32 / 255.0),
        srgb_to_linear(c.g as f32 / 255.0),
        srgb_to_linear(c.b as f32 / 255.0),
        c.a as f32 / 255.0,
    ]
}

fn current_clip_scissor(
    clip_stack: &[Option<Rect>],
    surface_width: u32,
    surface_height: u32,
) -> Option<Option<[u32; 4]>> {
    match clip_stack.last().copied() {
        Some(None) => None,
        Some(Some(active)) => rect_to_scissor(active, surface_width, surface_height).map(Some),
        None => Some(None),
    }
}

fn push_solid_rect(
    rects: &mut Vec<RectData>,
    rect: Rect,
    color: Color,
    corner_radius: f32,
    clip: Option<[u32; 4]>,
    scale_factor: f32,
    opacity: f32,
) {
    let color = modulate_color(color, opacity);
    if color.a == 0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    rects.push(RectData {
        x: rect.x * scale_factor,
        y: rect.y * scale_factor,
        width: rect.width * scale_factor,
        height: rect.height * scale_factor,
        color: color_to_f32(&color),
        corner_radius: corner_radius * scale_factor,
        clip,
    });
}

fn emit_gradient_rects(
    rects: &mut Vec<RectData>,
    rect: Rect,
    gradient: &Gradient,
    clip: Option<[u32; 4]>,
    scale_factor: f32,
    opacity: f32,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    match gradient {
        Gradient::Linear { angle_deg, .. } => {
            let theta = angle_deg.to_radians();
            let vertical_slices = theta.cos().abs() >= theta.sin().abs();
            let major = if vertical_slices {
                rect.width * scale_factor
            } else {
                rect.height * scale_factor
            };
            let steps = ((major / 12.0).ceil() as usize).clamp(32, 128);

            for i in 0..steps {
                let start = i as f32 / steps as f32;
                let end = (i + 1) as f32 / steps as f32;
                let band = if vertical_slices {
                    Rect::new(
                        rect.x + rect.width * start,
                        rect.y,
                        rect.width * (end - start),
                        rect.height,
                    )
                } else {
                    Rect::new(
                        rect.x,
                        rect.y + rect.height * start,
                        rect.width,
                        rect.height * (end - start),
                    )
                };
                let sample_x = band.x + band.width * 0.5;
                let sample_y = band.y + band.height * 0.5;
                let color = sample_gradient_color(gradient, rect, sample_x, sample_y);
                push_solid_rect(rects, band, color, 0.0, clip, scale_factor, opacity);
            }
        }
        Gradient::Radial { .. } => {
            let cols = ((rect.width * scale_factor / 24.0).ceil() as usize).clamp(4, 16);
            let rows = ((rect.height * scale_factor / 24.0).ceil() as usize).clamp(4, 16);

            for row in 0..rows {
                for col in 0..cols {
                    let start_x = col as f32 / cols as f32;
                    let end_x = (col + 1) as f32 / cols as f32;
                    let start_y = row as f32 / rows as f32;
                    let end_y = (row + 1) as f32 / rows as f32;
                    let cell = Rect::new(
                        rect.x + rect.width * start_x,
                        rect.y + rect.height * start_y,
                        rect.width * (end_x - start_x),
                        rect.height * (end_y - start_y),
                    );
                    let sample_x = cell.x + cell.width * 0.5;
                    let sample_y = cell.y + cell.height * 0.5;
                    let color = sample_gradient_color(gradient, rect, sample_x, sample_y);
                    push_solid_rect(rects, cell, color, 0.0, clip, scale_factor, opacity);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_shadow_rects(
    rects: &mut Vec<RectData>,
    rect: Rect,
    color: Color,
    blur_radius: f32,
    offset: velox_scene::Point,
    spread: f32,
    clip: Option<[u32; 4]>,
    scale_factor: f32,
    opacity: f32,
) {
    for (shadow_rect, weight) in shadow_layers(rect, blur_radius, offset, spread) {
        push_solid_rect(
            rects,
            shadow_rect,
            color,
            0.0,
            clip,
            scale_factor,
            opacity * weight,
        );
    }
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
