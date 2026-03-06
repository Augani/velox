use velox_scene::{Color, CommandList, PaintCommand, Rect};

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
            buffer: vec![0xFF1A1A1E; pixel_count],
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let pixel_count = (width as usize) * (height as usize);
        self.buffer.resize(pixel_count, 0xFF1A1A1E);
    }

    pub fn render(&mut self, commands: &CommandList) {
        self.clear(0xFF1A1A1E);
        for cmd in commands.commands() {
            if let PaintCommand::FillRect { rect, color } = cmd {
                self.fill_rect(*rect, *color);
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

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x0 = (rect.x.max(0.0) as u32).min(self.width);
        let y0 = (rect.y.max(0.0) as u32).min(self.height);
        let x1 = ((rect.x + rect.width).max(0.0) as u32).min(self.width);
        let y1 = ((rect.y + rect.height).max(0.0) as u32).min(self.height);

        let pixel = ((color.a as u32) << 24)
            | ((color.r as u32) << 16)
            | ((color.g as u32) << 8)
            | (color.b as u32);

        for y in y0..y1 {
            let row_start = (y as usize) * (self.width as usize);
            for x in x0..x1 {
                self.buffer[row_start + x as usize] = pixel;
            }
        }
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
    use velox_scene::CommandList;

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
}
