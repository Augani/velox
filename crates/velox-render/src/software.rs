use velox_scene::{Color, CommandList, PaintCommand, Rect};

use crate::paint_utils::{
    effective_layer_opacity, intersect_rect, modulate_color, sample_gradient_color, shadow_layers,
};

const CLEAR_COLOR: u32 = 0xFFFFFFFF;

pub struct SoftwareRenderer {
    width: u32,
    height: u32,
    buffer: Vec<u32>,
}

impl SoftwareRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width as usize) * (height as usize);
        Self {
            width,
            height,
            buffer: vec![CLEAR_COLOR; pixel_count],
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let pixel_count = (width as usize) * (height as usize);
        self.buffer.resize(pixel_count, CLEAR_COLOR);
    }

    pub fn render(&mut self, commands: &CommandList) {
        self.clear(CLEAR_COLOR);
        let mut clip_stack: Vec<Option<Rect>> = Vec::new();
        let mut layer_stack: Vec<f32> = Vec::new();
        for cmd in commands.commands() {
            match cmd {
                PaintCommand::FillRect { rect, color } => {
                    let Some(clip) = current_clip_rect(&clip_stack) else {
                        continue;
                    };
                    self.fill_rect(
                        *rect,
                        modulate_color(*color, effective_layer_opacity(&layer_stack)),
                        clip,
                    );
                }
                PaintCommand::StrokeRect { rect, color, width } => {
                    let Some(clip) = current_clip_rect(&clip_stack) else {
                        continue;
                    };
                    self.stroke_rect(
                        *rect,
                        modulate_color(*color, effective_layer_opacity(&layer_stack)),
                        *width,
                        clip,
                    );
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
                PaintCommand::PushLayer {
                    opacity,
                    blend_mode: _,
                } => {
                    layer_stack.push(opacity.clamp(0.0, 1.0));
                }
                PaintCommand::PopLayer => {
                    let _ = layer_stack.pop();
                }
                PaintCommand::FillRoundedRect {
                    rect,
                    color,
                    corner_radius,
                } => {
                    let Some(clip) = current_clip_rect(&clip_stack) else {
                        continue;
                    };
                    self.fill_rounded_rect(
                        *rect,
                        modulate_color(*color, effective_layer_opacity(&layer_stack)),
                        *corner_radius,
                        clip,
                    );
                }
                PaintCommand::FillGradient { rect, gradient } => {
                    let Some(clip) = current_clip_rect(&clip_stack) else {
                        continue;
                    };
                    self.fill_gradient(
                        *rect,
                        gradient,
                        effective_layer_opacity(&layer_stack),
                        clip,
                    );
                }
                PaintCommand::BoxShadow {
                    rect,
                    color,
                    blur_radius,
                    offset,
                    spread,
                } => {
                    let Some(clip) = current_clip_rect(&clip_stack) else {
                        continue;
                    };
                    self.fill_box_shadow(
                        *rect,
                        *color,
                        *blur_radius,
                        *offset,
                        *spread,
                        effective_layer_opacity(&layer_stack),
                        clip,
                    );
                }
                PaintCommand::DrawGlyphs { .. } | PaintCommand::DrawImage { .. } => {}
            }
        }
    }

    pub fn buffer(&self) -> &[u32] {
        &self.buffer
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn clear(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color, clip: Option<Rect>) {
        let Some((x0, y0, x1, y1)) = self.raster_bounds(rect, clip) else {
            return;
        };

        for y in y0..y1 {
            for x in x0..x1 {
                self.blend_pixel(x, y, color);
            }
        }
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32, clip: Option<Rect>) {
        if width <= 0.0 {
            return;
        }

        self.fill_rect(Rect::new(rect.x, rect.y, rect.width, width), color, clip);
        self.fill_rect(
            Rect::new(rect.x, rect.y + rect.height - width, rect.width, width),
            color,
            clip,
        );
        self.fill_rect(Rect::new(rect.x, rect.y, width, rect.height), color, clip);
        self.fill_rect(
            Rect::new(rect.x + rect.width - width, rect.y, width, rect.height),
            color,
            clip,
        );
    }

    fn fill_rounded_rect(
        &mut self,
        rect: Rect,
        color: Color,
        corner_radius: f32,
        clip: Option<Rect>,
    ) {
        if corner_radius <= f32::EPSILON {
            self.fill_rect(rect, color, clip);
            return;
        }

        let Some((x0, y0, x1, y1)) = self.raster_bounds(rect, clip) else {
            return;
        };
        let radius = corner_radius.min(rect.width * 0.5).min(rect.height * 0.5);

        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                if point_in_rounded_rect(px, py, rect, radius) {
                    self.blend_pixel(x, y, color);
                }
            }
        }
    }

    fn fill_gradient(
        &mut self,
        rect: Rect,
        gradient: &velox_scene::Gradient,
        opacity: f32,
        clip: Option<Rect>,
    ) {
        let Some((x0, y0, x1, y1)) = self.raster_bounds(rect, clip) else {
            return;
        };

        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let color = modulate_color(sample_gradient_color(gradient, rect, px, py), opacity);
                self.blend_pixel(x, y, color);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_box_shadow(
        &mut self,
        rect: Rect,
        color: Color,
        blur_radius: f32,
        offset: velox_scene::Point,
        spread: f32,
        opacity: f32,
        clip: Option<Rect>,
    ) {
        for (shadow_rect, weight) in shadow_layers(rect, blur_radius, offset, spread) {
            let layer_color = modulate_color(color, opacity * weight);
            self.fill_rect(shadow_rect, layer_color, clip);
        }
    }

    fn raster_bounds(&self, rect: Rect, clip: Option<Rect>) -> Option<(u32, u32, u32, u32)> {
        let rect = match clip {
            Some(clip_rect) => intersect_rect(rect, clip_rect)?,
            None => rect,
        };

        let x0 = rect.x.max(0.0).floor() as u32;
        let y0 = rect.y.max(0.0).floor() as u32;
        let x1 = (rect.x + rect.width).min(self.width as f32).ceil() as u32;
        let y1 = (rect.y + rect.height).min(self.height as f32).ceil() as u32;

        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        Some((x0, y0, x1, y1))
    }

    fn blend_pixel(&mut self, x: u32, y: u32, src: Color) {
        if src.a == 0 {
            return;
        }

        let idx = (y as usize) * (self.width as usize) + x as usize;
        let dst = unpack_color(self.buffer[idx]);

        let src_alpha = src.a as f32 / 255.0;
        let dst_alpha = dst.a as f32 / 255.0;
        let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);

        let blend_channel = |src_channel: u8, dst_channel: u8| -> u8 {
            if out_alpha <= f32::EPSILON {
                return 0;
            }

            (((src_channel as f32 * src_alpha)
                + (dst_channel as f32 * dst_alpha * (1.0 - src_alpha)))
                / out_alpha)
                .round()
                .clamp(0.0, 255.0) as u8
        };

        self.buffer[idx] = pack_color(Color::rgba(
            blend_channel(src.r, dst.r),
            blend_channel(src.g, dst.g),
            blend_channel(src.b, dst.b),
            (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        ));
    }
}

#[derive(Debug)]
pub enum RenderBackend {
    Gpu,
    Software,
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_scene::{BlendMode, CommandList, Gradient, GradientStop, Point};

    #[test]
    fn software_renderer_creates_buffer() {
        let renderer = SoftwareRenderer::new(100, 100);
        assert_eq!(renderer.buffer().len(), 10000);
        assert_eq!(renderer.width(), 100);
        assert_eq!(renderer.height(), 100);
    }

    #[test]
    fn software_renderer_fill_rect() {
        let mut renderer = SoftwareRenderer::new(10, 10);
        let mut commands = CommandList::new();
        commands.fill_rect(Rect::new(2.0, 2.0, 4.0, 4.0), Color::rgb(255, 0, 0));
        renderer.render(&commands);

        let pixel = renderer.buffer()[2 * 10 + 2];
        assert_eq!(pixel, 0xFFFF0000);
    }

    #[test]
    fn software_renderer_resize() {
        let mut renderer = SoftwareRenderer::new(10, 10);
        renderer.resize(20, 20);
        assert_eq!(renderer.buffer().len(), 400);
        assert_eq!(renderer.width(), 20);
        assert_eq!(renderer.height(), 20);
    }

    #[test]
    fn software_renderer_rect_clamps_to_bounds() {
        let mut renderer = SoftwareRenderer::new(10, 10);
        let mut commands = CommandList::new();
        commands.fill_rect(Rect::new(-5.0, -5.0, 20.0, 20.0), Color::rgb(0, 255, 0));
        renderer.render(&commands);
        let pixel = renderer.buffer()[0];
        assert_eq!(pixel, 0xFF00FF00);
    }

    #[test]
    fn software_renderer_respects_clip_stack() {
        let mut renderer = SoftwareRenderer::new(10, 10);
        let mut commands = CommandList::new();
        commands.push_clip(Rect::new(0.0, 0.0, 2.0, 2.0));
        commands.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), Color::rgb(255, 0, 0));
        commands.pop_clip();
        renderer.render(&commands);

        assert_eq!(renderer.buffer()[0], 0xFFFF0000);
        assert_eq!(renderer.buffer()[3], CLEAR_COLOR);
    }

    #[test]
    fn software_renderer_applies_layer_opacity() {
        let mut renderer = SoftwareRenderer::new(2, 2);
        let mut commands = CommandList::new();
        commands.push_layer(0.5, BlendMode::Normal);
        commands.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::rgb(255, 0, 0));
        commands.pop_layer();
        renderer.render(&commands);

        let pixel = renderer.buffer()[0];
        assert_ne!(pixel, CLEAR_COLOR);
        assert_ne!(pixel, 0xFFFF0000);
    }

    #[test]
    fn software_renderer_fill_gradient_varies_color_across_rect() {
        let mut renderer = SoftwareRenderer::new(10, 2);
        let mut commands = CommandList::new();
        commands.fill_gradient(
            Rect::new(0.0, 0.0, 10.0, 2.0),
            Gradient::Linear {
                angle_deg: 0.0,
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: Color::rgb(0, 0, 0),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: Color::rgb(255, 255, 255),
                    },
                ],
            },
        );
        renderer.render(&commands);

        assert_ne!(renderer.buffer()[0], renderer.buffer()[9]);
    }

    #[test]
    fn software_renderer_box_shadow_paints_outside_source_rect() {
        let mut renderer = SoftwareRenderer::new(12, 12);
        let mut commands = CommandList::new();
        commands.box_shadow(
            Rect::new(4.0, 4.0, 2.0, 2.0),
            Color::rgba(255, 0, 0, 180),
            4.0,
            Point::new(0.0, 0.0),
            0.0,
        );
        renderer.render(&commands);

        assert_ne!(renderer.buffer()[3 * 12 + 3], CLEAR_COLOR);
    }
}

fn current_clip_rect(clip_stack: &[Option<Rect>]) -> Option<Option<Rect>> {
    match clip_stack.last().copied() {
        Some(None) => None,
        Some(Some(clip)) => Some(Some(clip)),
        None => Some(None),
    }
}

fn point_in_rounded_rect(x: f32, y: f32, rect: Rect, radius: f32) -> bool {
    if x < rect.x || x >= rect.x + rect.width || y < rect.y || y >= rect.y + rect.height {
        return false;
    }

    let nearest_x = x.clamp(rect.x + radius, rect.x + rect.width - radius);
    let nearest_y = y.clamp(rect.y + radius, rect.y + rect.height - radius);
    let dx = x - nearest_x;
    let dy = y - nearest_y;
    dx * dx + dy * dy <= radius * radius
}

fn unpack_color(pixel: u32) -> Color {
    Color::rgba(
        ((pixel >> 16) & 0xFF) as u8,
        ((pixel >> 8) & 0xFF) as u8,
        (pixel & 0xFF) as u8,
        ((pixel >> 24) & 0xFF) as u8,
    )
}

fn pack_color(color: Color) -> u32 {
    ((color.a as u32) << 24) | ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32)
}
