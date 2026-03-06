# Phase 3: Rendering, Text, and Input — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add GPU rendering via wgpu, text shaping/rendering via cosmic-text with a glyph atlas, and a keyboard shortcut system — turning Phase 2's abstract paint commands into real pixels on screen.

**Architecture:** Two new crates (`velox-render`, `velox-text`) plus modifications to existing crates. `velox-text` wraps cosmic-text for font/shaping/rasterization. `velox-render` owns wgpu setup, the rendering pipeline, and a glyph atlas. `velox-scene` gains glyph-related paint commands and a shortcut registry. `velox-app` wires everything together in the event loop.

**Tech Stack:** Rust (edition 2024), wgpu 24, cosmic-text 0.12, pollster (for blocking on wgpu async)

---

## Context

### Existing Crates (Phases 1-2)

- `velox-reactive` — Signal, Computed, Event, Batch, Subscription
- `velox-runtime` — Runtime, FrameClock, PowerPolicy, task executors
- `velox-platform` — platform traits with stub impls
- `velox-window` — WindowConfig, WindowId, WindowManager, ManagedWindow
- `velox-scene` — NodeTree, Scene, Painter, Layout, CommandList, PaintCommand, FocusState, OverlayStack
- `velox-app` — App builder, VeloxHandler (ApplicationHandler impl)
- `velox` — facade re-exports + prelude

### Key Files

- `crates/velox-scene/src/paint.rs` — PaintCommand enum (FillRect, StrokeRect, PushClip, PopClip), Color, CommandList
- `crates/velox-scene/src/scene.rs` — Scene struct with layout(), paint(), hit_test()
- `crates/velox-app/src/handler.rs` — VeloxHandler: owns Runtime + WindowManager + HashMap<WindowId, Scene>
- `crates/velox-app/src/app.rs` — App builder with run()
- `crates/velox-window/src/manager.rs` — WindowManager with create_window(), HashMap<WindowId, ManagedWindow>

### wgpu API (v24+)

Key types: `Instance`, `Adapter`, `Device`, `Queue`, `Surface`, `SurfaceConfiguration`, `RenderPipeline`, `Buffer`, `Texture`, `BindGroup`.

Setup flow:
1. `Instance::new()` → `instance.create_surface(window)` → `instance.request_adapter()` → `adapter.request_device()` → `surface.configure(device, config)`
2. Each frame: `surface.get_current_texture()` → `texture.create_view()` → `device.create_command_encoder()` → `encoder.begin_render_pass()` → draw calls → `queue.submit()` → `output.present()`

### cosmic-text API

```rust
let mut font_system = FontSystem::new();
let mut swash_cache = SwashCache::new();
let mut buffer = Buffer::new(&mut font_system, Metrics::new(font_size, line_height));
buffer.set_size(&mut font_system, Some(width), Some(height));
buffer.set_text(&mut font_system, text, &Attrs::new(), Shaping::Advanced, None);
buffer.shape_until_scroll(&mut font_system, true);
for run in buffer.layout_runs() {
    for glyph in run.glyphs.iter() {
        let physical = glyph.physical((0.0, 0.0), 1.0);
        if let Some(image) = swash_cache.get_image(&mut font_system, physical.cache_key) {
            // image.placement.width/height, image.data (alpha bitmap)
        }
    }
}
```

---

## Task 1: Scaffold `velox-text` Crate

**Files:**
- Create: `crates/velox-text/Cargo.toml`
- Create: `crates/velox-text/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "velox-text"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
cosmic-text = "0.12"
```

**Step 2: Create src/lib.rs**

```rust
pub use cosmic_text;
```

**Step 3: Add to workspace root Cargo.toml**

Add `"crates/velox-text"` to members. Add workspace deps:
```toml
velox-text = { path = "crates/velox-text" }
cosmic-text = "0.12"
```

**Step 4: Verify**

Run: `cargo build -p velox-text`
Expected: compiles

**Step 5: Commit**

```bash
git add crates/velox-text/ Cargo.toml Cargo.lock
git commit -m "feat(text): scaffold velox-text crate with cosmic-text dependency"
```

---

## Task 2: FontSystem and TextBuffer Wrappers

**Files:**
- Create: `crates/velox-text/src/font_system.rs`
- Create: `crates/velox-text/src/buffer.rs`
- Create: `crates/velox-text/src/attrs.rs`
- Modify: `crates/velox-text/src/lib.rs`

**Step 1: Write tests in buffer.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_system::FontSystem;

    #[test]
    fn create_buffer_and_shape() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, "Hello, world!", TextAttrs::default());
        buf.shape(&mut fs);
        let runs: Vec<_> = buf.layout_runs().collect();
        assert!(!runs.is_empty());
    }

    #[test]
    fn empty_text_has_no_runs() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, "", TextAttrs::default());
        buf.shape(&mut fs);
        let runs: Vec<_> = buf.layout_runs().collect();
        assert!(runs.is_empty());
    }

    #[test]
    fn multiline_text() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, "Line 1\nLine 2\nLine 3", TextAttrs::default());
        buf.shape(&mut fs);
        let runs: Vec<_> = buf.layout_runs().collect();
        assert!(runs.len() >= 3);
    }
}
```

**Step 2: Implement font_system.rs**

```rust
pub struct FontSystem {
    inner: cosmic_text::FontSystem,
}

impl FontSystem {
    pub fn new() -> Self {
        Self {
            inner: cosmic_text::FontSystem::new(),
        }
    }

    pub(crate) fn inner_mut(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.inner
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Implement attrs.rs**

```rust
#[derive(Debug, Clone)]
pub struct TextAttrs {
    pub family: FontFamily,
    pub size: f32,
    pub weight: u16,
    pub style: FontStyle,
}

#[derive(Debug, Clone)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

impl Default for TextAttrs {
    fn default() -> Self {
        Self {
            family: FontFamily::SansSerif,
            size: 14.0,
            weight: 400,
            style: FontStyle::Normal,
        }
    }
}

impl TextAttrs {
    pub(crate) fn to_cosmic(&self) -> cosmic_text::Attrs<'_> {
        let family = match &self.family {
            FontFamily::SansSerif => cosmic_text::Family::SansSerif,
            FontFamily::Serif => cosmic_text::Family::Serif,
            FontFamily::Monospace => cosmic_text::Family::Monospace,
            FontFamily::Named(name) => cosmic_text::Family::Name(name.as_str()),
        };
        let style = match self.style {
            FontStyle::Normal => cosmic_text::Style::Normal,
            FontStyle::Italic => cosmic_text::Style::Italic,
        };
        cosmic_text::Attrs::new()
            .family(family)
            .weight(cosmic_text::Weight(self.weight))
            .style(style)
    }
}
```

**Step 4: Implement buffer.rs**

```rust
use cosmic_text::{Buffer, Metrics, Shaping};

use crate::attrs::TextAttrs;
use crate::font_system::FontSystem;

pub struct TextBuffer {
    inner: Buffer,
}

impl TextBuffer {
    pub fn new(font_system: &mut FontSystem, font_size: f32, line_height: f32) -> Self {
        Self {
            inner: Buffer::new(font_system.inner_mut(), Metrics::new(font_size, line_height)),
        }
    }

    pub fn set_size(&mut self, font_system: &mut FontSystem, width: f32, height: f32) {
        self.inner.set_size(font_system.inner_mut(), Some(width), Some(height));
    }

    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str, attrs: TextAttrs) {
        let cosmic_attrs = attrs.to_cosmic();
        self.inner.set_text(font_system.inner_mut(), text, &cosmic_attrs, Shaping::Advanced, None);
    }

    pub fn shape(&mut self, font_system: &mut FontSystem) {
        self.inner.shape_until_scroll(font_system.inner_mut(), true);
    }

    pub fn layout_runs(&self) -> impl Iterator<Item = &cosmic_text::LayoutRun<'_>> {
        self.inner.layout_runs()
    }

    pub(crate) fn inner(&self) -> &Buffer {
        &self.inner
    }
}
```

**Step 5: Update lib.rs**

```rust
mod attrs;
mod buffer;
mod font_system;

pub use attrs::{FontFamily, FontStyle, TextAttrs};
pub use buffer::TextBuffer;
pub use font_system::FontSystem;

pub use cosmic_text;
```

**Step 6: Run tests**

Run: `cargo test -p velox-text`
Expected: 3 tests pass

**Step 7: Commit**

```bash
git add crates/velox-text/
git commit -m "feat(text): add FontSystem, TextBuffer, and TextAttrs wrappers"
```

---

## Task 3: Glyph Rasterizer

**Files:**
- Create: `crates/velox-text/src/rasterizer.rs`
- Modify: `crates/velox-text/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::TextAttrs;
    use crate::buffer::TextBuffer;
    use crate::font_system::FontSystem;

    #[test]
    fn rasterize_glyphs_from_buffer() {
        let mut fs = FontSystem::new();
        let mut rasterizer = GlyphRasterizer::new();
        let mut buf = TextBuffer::new(&mut fs, 24.0, 30.0);
        buf.set_size(&mut fs, 400.0, 100.0);
        buf.set_text(&mut fs, "Hello", TextAttrs::default());
        buf.shape(&mut fs);

        let mut rasterized_count = 0;
        for run in buf.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                if let Some(image) = rasterizer.rasterize(&mut fs, physical.cache_key) {
                    assert!(image.width > 0 || image.height > 0 || image.data.is_empty());
                    rasterized_count += 1;
                }
            }
        }
        assert!(rasterized_count > 0);
    }
}
```

**Step 2: Implement rasterizer.rs**

```rust
use cosmic_text::{CacheKey, SwashCache};

use crate::font_system::FontSystem;

pub struct RasterizedGlyph {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub data: Vec<u8>,
}

pub struct GlyphRasterizer {
    swash_cache: SwashCache,
}

impl GlyphRasterizer {
    pub fn new() -> Self {
        Self {
            swash_cache: SwashCache::new(),
        }
    }

    pub fn rasterize(&mut self, font_system: &mut FontSystem, cache_key: CacheKey) -> Option<RasterizedGlyph> {
        let image = self.swash_cache.get_image(font_system.inner_mut(), cache_key)?;
        Some(RasterizedGlyph {
            width: image.placement.width as u32,
            height: image.placement.height as u32,
            left: image.placement.left,
            top: image.placement.top,
            data: image.data.clone(),
        })
    }
}

impl Default for GlyphRasterizer {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Update lib.rs**

Add `mod rasterizer;` and `pub use rasterizer::{GlyphRasterizer, RasterizedGlyph};`

**Step 4: Run tests**

Run: `cargo test -p velox-text`
Expected: 4 tests pass

**Step 5: Commit**

```bash
git add crates/velox-text/
git commit -m "feat(text): add GlyphRasterizer with SwashCache integration"
```

---

## Task 4: Scaffold `velox-render` Crate + GpuContext

**Files:**
- Create: `crates/velox-render/Cargo.toml`
- Create: `crates/velox-render/src/lib.rs`
- Create: `crates/velox-render/src/gpu.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "velox-render"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
velox-scene = { workspace = true }
velox-text = { workspace = true }
wgpu = { workspace = true }
pollster = "0.4"
```

**Step 2: Add workspace deps to root Cargo.toml**

```toml
velox-render = { path = "crates/velox-render" }
wgpu = "24"
pollster = "0.4"
```

Add `"crates/velox-render"` to workspace members.

**Step 3: Implement gpu.rs**

```rust
use std::sync::Arc;

pub struct GpuContext {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
}

impl GpuContext {
    pub fn new(compatible_surface: Option<&wgpu::Surface<'_>>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface,
            force_fallback_adapter: false,
        }))
        .expect("failed to find a suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("velox"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            },
        ))
        .expect("failed to create GPU device");

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }
}
```

**Step 4: Create lib.rs**

```rust
mod gpu;

pub use gpu::GpuContext;
```

**Step 5: Verify**

Run: `cargo build -p velox-render`
Expected: compiles

**Step 6: Commit**

```bash
git add crates/velox-render/ Cargo.toml Cargo.lock
git commit -m "feat(render): scaffold velox-render crate with GpuContext"
```

---

## Task 5: WindowSurface

**Files:**
- Create: `crates/velox-render/src/surface.rs`
- Modify: `crates/velox-render/src/lib.rs`

**Step 1: Implement surface.rs**

```rust
use std::sync::Arc;

use crate::gpu::GpuContext;

pub struct WindowSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    width: u32,
    height: u32,
}

impl WindowSurface {
    pub fn new(gpu: &GpuContext, window: Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();
        let surface = gpu.instance.create_surface(window).expect("failed to create surface");

        let caps = surface.get_capabilities(&gpu.adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&gpu.device, &config);

        Self {
            surface,
            config,
            width: size.width.max(1),
            height: size.height.max(1),
        }
    }

    pub fn resize(&mut self, gpu: &GpuContext, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&gpu.device, &self.config);
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn surface(&self) -> &wgpu::Surface<'_> {
        &self.surface
    }
}
```

**Step 2: Add winit dep to velox-render Cargo.toml**

```toml
winit = { workspace = true }
```

**Step 3: Update lib.rs**

```rust
mod gpu;
mod surface;

pub use gpu::GpuContext;
pub use surface::WindowSurface;
```

**Step 4: Verify**

Run: `cargo build -p velox-render`
Expected: compiles

**Step 5: Commit**

```bash
git add crates/velox-render/
git commit -m "feat(render): add WindowSurface with wgpu surface management"
```

---

## Task 6: Rect Renderer (Shader + Pipeline + Vertex Buffer)

**Files:**
- Create: `crates/velox-render/src/shaders/rect.wgsl`
- Create: `crates/velox-render/src/rect_renderer.rs`
- Modify: `crates/velox-render/src/lib.rs`

**Step 1: Create rect.wgsl shader**

```wgsl
struct Uniform {
    screen_size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let x = (in.position.x / u.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (in.position.y / u.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
```

**Step 2: Implement rect_renderer.rs**

```rust
use wgpu::util::DeviceExt;

use crate::gpu::GpuContext;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RectVertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

pub struct RectRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    vertices: Vec<RectVertex>,
}

impl RectRenderer {
    pub fn new(gpu: &GpuContext, target_format: wgpu::TextureFormat) -> Self {
        let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rect.wgsl").into()),
        });

        let uniform_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rect_uniform"),
            contents: bytemuck::bytes_of(&ScreenUniform {
                screen_size: [800.0, 600.0],
                _padding: [0.0; 2],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let pipeline_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
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
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
        }
    }

    pub fn prepare(&mut self, gpu: &GpuContext, width: u32, height: u32, rects: &[RectData]) {
        gpu.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&ScreenUniform {
                screen_size: [width as f32, height as f32],
                _padding: [0.0; 2],
            }),
        );

        self.vertices.clear();
        for rect in rects {
            let x0 = rect.x;
            let y0 = rect.y;
            let x1 = rect.x + rect.width;
            let y1 = rect.y + rect.height;
            let c = rect.color;
            self.vertices.extend_from_slice(&[
                RectVertex { position: [x0, y0], color: c },
                RectVertex { position: [x1, y0], color: c },
                RectVertex { position: [x0, y1], color: c },
                RectVertex { position: [x1, y0], color: c },
                RectVertex { position: [x1, y1], color: c },
                RectVertex { position: [x0, y1], color: c },
            ]);
        }
    }

    pub fn render<'a>(&'a self, gpu: &GpuContext, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.vertices.is_empty() {
            return;
        }
        let vertex_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rect_vertices"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}

pub struct RectData {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
}
```

**Step 3: Add bytemuck dep**

In `crates/velox-render/Cargo.toml`:
```toml
bytemuck = { version = "1", features = ["derive"] }
```

In workspace root:
```toml
bytemuck = { version = "1", features = ["derive"] }
```

**Step 4: Create shaders directory and update lib.rs**

```rust
mod gpu;
mod rect_renderer;
mod surface;

pub use gpu::GpuContext;
pub use rect_renderer::{RectData, RectRenderer};
pub use surface::WindowSurface;
```

**Step 5: Verify**

Run: `cargo build -p velox-render`
Expected: compiles

**Step 6: Commit**

```bash
git add crates/velox-render/ Cargo.toml Cargo.lock
git commit -m "feat(render): add RectRenderer with wgpu pipeline and WGSL shader"
```

---

## Task 7: Glyph Atlas

**Files:**
- Create: `crates/velox-render/src/glyph_atlas.rs`
- Modify: `crates/velox-render/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_packer_insert_and_lookup() {
        let mut packer = ShelfPacker::new(256, 256);
        let region = packer.allocate(10, 12);
        assert!(region.is_some());
        let r = region.unwrap();
        assert_eq!(r.width, 10);
        assert_eq!(r.height, 12);
    }

    #[test]
    fn atlas_packer_fills_shelf() {
        let mut packer = ShelfPacker::new(64, 64);
        for _ in 0..6 {
            assert!(packer.allocate(10, 10).is_some());
        }
    }

    #[test]
    fn atlas_packer_returns_none_when_full() {
        let mut packer = ShelfPacker::new(16, 16);
        packer.allocate(16, 16);
        assert!(packer.allocate(1, 1).is_none());
    }
}
```

**Step 2: Implement glyph_atlas.rs**

```rust
use std::collections::HashMap;

use cosmic_text::CacheKey;

#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

struct Shelf {
    y: u32,
    height: u32,
    x_cursor: u32,
}

pub(crate) struct ShelfPacker {
    width: u32,
    height: u32,
    shelves: Vec<Shelf>,
    y_cursor: u32,
}

impl ShelfPacker {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            shelves: Vec::new(),
            y_cursor: 0,
        }
    }

    pub fn allocate(&mut self, w: u32, h: u32) -> Option<AtlasRegion> {
        for shelf in &mut self.shelves {
            if h <= shelf.height && shelf.x_cursor + w <= self.width {
                let region = AtlasRegion {
                    x: shelf.x_cursor,
                    y: shelf.y,
                    width: w,
                    height: h,
                };
                shelf.x_cursor += w;
                return Some(region);
            }
        }

        if self.y_cursor + h <= self.height {
            let shelf = Shelf {
                y: self.y_cursor,
                height: h,
                x_cursor: w,
            };
            let region = AtlasRegion {
                x: 0,
                y: self.y_cursor,
                width: w,
                height: h,
            };
            self.y_cursor += h;
            self.shelves.push(shelf);
            return Some(region);
        }

        None
    }
}

pub struct GlyphAtlas {
    packer: ShelfPacker,
    entries: HashMap<CacheKey, AtlasRegion>,
    texture_data: Vec<u8>,
    atlas_width: u32,
    atlas_height: u32,
    dirty: bool,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            packer: ShelfPacker::new(width, height),
            entries: HashMap::new(),
            texture_data: vec![0; (width * height) as usize],
            atlas_width: width,
            atlas_height: height,
            dirty: false,
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<AtlasRegion> {
        self.entries.get(key).copied()
    }

    pub fn insert(&mut self, key: CacheKey, width: u32, height: u32, data: &[u8]) -> Option<AtlasRegion> {
        if let Some(existing) = self.entries.get(&key) {
            return Some(*existing);
        }
        if width == 0 || height == 0 {
            return None;
        }
        let region = self.packer.allocate(width, height)?;
        for row in 0..height {
            let src_start = (row * width) as usize;
            let dst_start = ((region.y + row) * self.atlas_width + region.x) as usize;
            let len = width as usize;
            if src_start + len <= data.len() && dst_start + len <= self.texture_data.len() {
                self.texture_data[dst_start..dst_start + len].copy_from_slice(&data[src_start..src_start + len]);
            }
        }
        self.entries.insert(key, region);
        self.dirty = true;
        Some(region)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn texture_data(&self) -> &[u8] {
        &self.texture_data
    }

    pub fn width(&self) -> u32 {
        self.atlas_width
    }

    pub fn height(&self) -> u32 {
        self.atlas_height
    }

    pub fn uv(&self, region: &AtlasRegion) -> [f32; 4] {
        let w = self.atlas_width as f32;
        let h = self.atlas_height as f32;
        [
            region.x as f32 / w,
            region.y as f32 / h,
            (region.x + region.width) as f32 / w,
            (region.y + region.height) as f32 / h,
        ]
    }
}
```

**Step 3: Update lib.rs**

Add `mod glyph_atlas;` and `pub use glyph_atlas::{AtlasRegion, GlyphAtlas};`

**Step 4: Run tests**

Run: `cargo test -p velox-render`
Expected: 3 tests pass

**Step 5: Commit**

```bash
git add crates/velox-render/
git commit -m "feat(render): add GlyphAtlas with shelf-packing algorithm"
```

---

## Task 8: Glyph Renderer (Textured Quad Pipeline)

**Files:**
- Create: `crates/velox-render/src/shaders/glyph.wgsl`
- Create: `crates/velox-render/src/glyph_renderer.rs`
- Modify: `crates/velox-render/src/lib.rs`

**Step 1: Create glyph.wgsl shader**

```wgsl
struct Uniform {
    screen_size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniform;
@group(0) @binding(1)
var atlas_texture: texture_2d<f32>;
@group(0) @binding(2)
var atlas_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let x = (in.position.x / u.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (in.position.y / u.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_texture, atlas_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
```

**Step 2: Implement glyph_renderer.rs**

This is similar to RectRenderer but uses textured quads with a glyph atlas texture. Key differences:
- Vertex has position (2f), uv (2f), color (4f)
- Bind group includes atlas texture + sampler
- Fragment shader samples alpha from atlas, multiplies by vertex color

```rust
use wgpu::util::DeviceExt;

use crate::gpu::GpuContext;
use crate::glyph_atlas::GlyphAtlas;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GlyphVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

pub struct GlyphRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    atlas_texture: Option<wgpu::Texture>,
    sampler: wgpu::Sampler,
    vertices: Vec<GlyphVertex>,
}

impl GlyphRenderer {
    pub fn new(gpu: &GpuContext, target_format: wgpu::TextureFormat) -> Self {
        let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glyph_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/glyph.wgsl").into()),
        });

        let uniform_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glyph_uniform"),
            contents: bytemuck::bytes_of(&ScreenUniform {
                screen_size: [800.0, 600.0],
                _padding: [0.0; 2],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glyph_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glyph_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                        wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                        wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x4 },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
            atlas_texture: None,
            sampler,
            vertices: Vec::new(),
        }
    }

    pub fn upload_atlas(&mut self, gpu: &GpuContext, atlas: &GlyphAtlas) {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d { width: atlas.width(), height: atlas.height(), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            atlas.texture_data(),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(atlas.width()), rows_per_image: Some(atlas.height()) },
            wgpu::Extent3d { width: atlas.width(), height: atlas.height(), depth_or_array_layers: 1 },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.bind_group = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&texture_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        }));

        self.atlas_texture = Some(texture);
    }

    pub fn prepare(&mut self, gpu: &GpuContext, width: u32, height: u32, quads: &[GlyphQuad]) {
        gpu.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&ScreenUniform {
                screen_size: [width as f32, height as f32],
                _padding: [0.0; 2],
            }),
        );

        self.vertices.clear();
        for q in quads {
            let x0 = q.x;
            let y0 = q.y;
            let x1 = q.x + q.width;
            let y1 = q.y + q.height;
            self.vertices.extend_from_slice(&[
                GlyphVertex { position: [x0, y0], uv: [q.uv[0], q.uv[1]], color: q.color },
                GlyphVertex { position: [x1, y0], uv: [q.uv[2], q.uv[1]], color: q.color },
                GlyphVertex { position: [x0, y1], uv: [q.uv[0], q.uv[3]], color: q.color },
                GlyphVertex { position: [x1, y0], uv: [q.uv[2], q.uv[1]], color: q.color },
                GlyphVertex { position: [x1, y1], uv: [q.uv[2], q.uv[3]], color: q.color },
                GlyphVertex { position: [x0, y1], uv: [q.uv[0], q.uv[3]], color: q.color },
            ]);
        }
    }

    pub fn render<'a>(&'a self, gpu: &GpuContext, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.vertices.is_empty() || self.bind_group.is_none() {
            return;
        }
        let vertex_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glyph_vertices"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}

pub struct GlyphQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub uv: [f32; 4],
    pub color: [f32; 4],
}
```

**Step 3: Update lib.rs**

Add `mod glyph_renderer;` and `pub use glyph_renderer::{GlyphQuad, GlyphRenderer};`

**Step 4: Verify**

Run: `cargo build -p velox-render`
Expected: compiles

**Step 5: Commit**

```bash
git add crates/velox-render/
git commit -m "feat(render): add GlyphRenderer with textured quad pipeline for text"
```

---

## Task 9: Renderer (Orchestrator)

**Files:**
- Create: `crates/velox-render/src/renderer.rs`
- Modify: `crates/velox-render/src/lib.rs`

**Step 1: Implement renderer.rs**

The `Renderer` orchestrates `RectRenderer` and `GlyphRenderer`. It takes a `CommandList` from velox-scene and converts commands into draw calls.

```rust
use velox_scene::{Color, CommandList, PaintCommand, Rect};

use crate::glyph_atlas::GlyphAtlas;
use crate::glyph_renderer::{GlyphQuad, GlyphRenderer};
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
                    rects.push(RectData { x: rect.x, y: rect.y, width: rect.width, height: w, color: c });
                    rects.push(RectData { x: rect.x, y: rect.y + rect.height - w, width: rect.width, height: w, color: c });
                    rects.push(RectData { x: rect.x, y: rect.y + w, width: w, height: rect.height - 2.0 * w, color: c });
                    rects.push(RectData { x: rect.x + rect.width - w, y: rect.y + w, width: w, height: rect.height - 2.0 * w, color: c });
                }
                PaintCommand::PushClip(_) | PaintCommand::PopClip => {}
            }
        }

        self.rect_renderer.prepare(gpu, surface.width(), surface.height(), &rects);

        if atlas.is_dirty() {
            self.glyph_renderer.upload_atlas(gpu, atlas);
            atlas.clear_dirty();
        }

        let output = surface.surface().get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("velox_render"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velox_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.12, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.rect_renderer.render(gpu, &mut render_pass);
            self.glyph_renderer.render(gpu, &mut render_pass);
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
```

**Step 2: Update lib.rs**

```rust
mod glyph_atlas;
mod glyph_renderer;
mod gpu;
mod rect_renderer;
mod renderer;
mod surface;

pub use glyph_atlas::{AtlasRegion, GlyphAtlas};
pub use glyph_renderer::{GlyphQuad, GlyphRenderer};
pub use gpu::GpuContext;
pub use rect_renderer::{RectData, RectRenderer};
pub use renderer::Renderer;
pub use surface::WindowSurface;
```

**Step 3: Verify**

Run: `cargo build -p velox-render`
Expected: compiles

**Step 4: Commit**

```bash
git add crates/velox-render/
git commit -m "feat(render): add Renderer orchestrator consuming CommandList"
```

---

## Task 10: Keyboard Shortcut Registry

**Files:**
- Create: `crates/velox-scene/src/shortcut.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn register_and_fire_shortcut() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(KeyCombo::new(Key::S, Modifiers::SUPER), move || f.set(true));
        let handled = registry.handle_key_event(Key::S, Modifiers::SUPER);
        assert!(handled);
        assert!(fired.get());
    }

    #[test]
    fn unmatched_key_returns_false() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(KeyCombo::new(Key::S, Modifiers::SUPER), move || f.set(true));
        let handled = registry.handle_key_event(Key::Q, Modifiers::SUPER);
        assert!(!handled);
        assert!(!fired.get());
    }

    #[test]
    fn unregister_shortcut() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        let id = registry.register(KeyCombo::new(Key::W, Modifiers::SUPER), move || f.set(true));
        registry.unregister(id);
        let handled = registry.handle_key_event(Key::W, Modifiers::SUPER);
        assert!(!handled);
        assert!(!fired.get());
    }

    #[test]
    fn modifier_must_match() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(KeyCombo::new(Key::S, Modifiers::CTRL), move || f.set(true));
        let handled = registry.handle_key_event(Key::S, Modifiers::SUPER);
        assert!(!handled);
        assert!(!fired.get());
    }

    #[test]
    fn multiple_modifiers() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(
            KeyCombo::new(Key::S, Modifiers::CTRL | Modifiers::SHIFT),
            move || f.set(true),
        );
        let handled = registry.handle_key_event(Key::S, Modifiers::CTRL | Modifiers::SHIFT);
        assert!(handled);
        assert!(fired.get());
    }
}
```

**Step 2: Implement shortcut.rs**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Enter, Escape, Tab, Space, Backspace, Delete,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, PageUp, PageDown,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b0001;
        const CTRL  = 0b0010;
        const ALT   = 0b0100;
        const SUPER = 0b1000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyCombo {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShortcutId(u64);

pub struct ShortcutRegistry {
    shortcuts: HashMap<ShortcutId, (KeyCombo, Box<dyn FnMut()>)>,
    next_id: u64,
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        Self {
            shortcuts: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn register(&mut self, combo: KeyCombo, callback: impl FnMut() + 'static) -> ShortcutId {
        let id = ShortcutId(self.next_id);
        self.next_id += 1;
        self.shortcuts.insert(id, (combo, Box::new(callback)));
        id
    }

    pub fn unregister(&mut self, id: ShortcutId) {
        self.shortcuts.remove(&id);
    }

    pub fn handle_key_event(&mut self, key: Key, modifiers: Modifiers) -> bool {
        let combo = KeyCombo { key, modifiers };
        for (_id, (registered, callback)) in &mut self.shortcuts {
            if *registered == combo {
                callback();
                return true;
            }
        }
        false
    }
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Add bitflags dep to velox-scene Cargo.toml**

```toml
bitflags = "2"
```

Add to workspace root:
```toml
bitflags = "2"
```

**Step 4: Update lib.rs**

Add `mod shortcut;` and export:
```rust
pub use shortcut::{Key, KeyCombo, Modifiers, ShortcutId, ShortcutRegistry};
```

**Step 5: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass (46 existing + 5 new = 51)

**Step 6: Commit**

```bash
git add crates/velox-scene/ Cargo.toml Cargo.lock
git commit -m "feat(scene): add ShortcutRegistry with Key, Modifiers, KeyCombo"
```

---

## Task 11: Integrate Rendering into velox-app

**Files:**
- Modify: `crates/velox-app/Cargo.toml`
- Modify: `crates/velox-app/src/handler.rs`
- Modify: `crates/velox-app/src/app.rs`
- Modify: `crates/velox-window/src/manager.rs` (expose Arc<Window>)

**Step 1: Add deps to velox-app**

```toml
velox-render = { workspace = true }
velox-text = { workspace = true }
```

**Step 2: Update WindowManager to store Arc<Window>**

In `crates/velox-window/src/manager.rs`, change `ManagedWindow` to store `Arc<winit::window::Window>` instead of `winit::window::Window`. Add `pub fn window_arc(&self) -> Arc<winit::window::Window>` method.

Update `create_window` to wrap window in `Arc::new()`.

**Step 3: Update VeloxHandler**

The handler now owns GpuContext, Renderer, GlyphAtlas, and per-window WindowSurface alongside Scene:

```rust
use std::collections::HashMap;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use velox_render::{GlyphAtlas, GpuContext, Renderer, WindowSurface};
use velox_runtime::Runtime;
use velox_scene::{Scene, ShortcutRegistry};
use velox_window::{WindowConfig, WindowId, WindowManager};

struct WindowState {
    scene: Scene,
    surface: WindowSurface,
}

pub(crate) struct VeloxHandler {
    runtime: Runtime,
    window_manager: WindowManager,
    windows: HashMap<WindowId, WindowState>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    glyph_atlas: GlyphAtlas,
    shortcuts: ShortcutRegistry,
    pending_windows: Vec<WindowConfig>,
    initialized: bool,
}
```

In `resumed()`:
1. Create first window to get a surface for adapter selection
2. Create `GpuContext` with that surface
3. Create `Renderer` with the surface format
4. Create `WindowSurface` for each window
5. Store everything

In `window_event` for `RedrawRequested`:
1. `scene.layout()` → `scene.paint()`
2. `renderer.render(gpu, surface, scene.commands(), &mut glyph_atlas)`

In `window_event` for `Resized`:
1. `surface.resize(gpu, width, height)`

In `window_event` for `KeyboardInput`:
1. Convert winit key to velox Key
2. Call `shortcuts.handle_key_event(key, modifiers)`

**Step 4: Verify**

Run: `cargo build --workspace`
Expected: compiles

**Step 5: Commit**

```bash
git add crates/velox-app/ crates/velox-window/ Cargo.lock
git commit -m "feat(app): integrate wgpu rendering pipeline into VeloxHandler"
```

---

## Task 12: Update Facade, Demo, and CLAUDE.md

**Files:**
- Modify: `crates/velox/Cargo.toml`
- Modify: `crates/velox/src/lib.rs`
- Create: `crates/velox/examples/phase3_demo.rs`
- Modify: `CLAUDE.md`

**Step 1: Update facade**

Add `velox-render` and `velox-text` deps. Re-export them. Update prelude:

```rust
pub use velox_app as app;
pub use velox_platform as platform;
pub use velox_reactive as reactive;
pub use velox_render as render;
pub use velox_runtime as runtime;
pub use velox_scene as scene;
pub use velox_text as text;
pub use velox_window as window;

pub mod prelude {
    pub use velox_app::App;
    pub use velox_reactive::{Batch, Computed, Event, Signal, Subscription, SubscriptionBag};
    pub use velox_render::{GpuContext, Renderer};
    pub use velox_runtime::{PowerClass, PowerPolicy};
    pub use velox_scene::{NodeId, NodeTree, Point, Rect, Scene, Size};
    pub use velox_text::{FontSystem, TextBuffer};
    pub use velox_window::WindowConfig;
}
```

**Step 2: Create phase3_demo.rs**

A demo that opens a window and renders colored rectangles:

```rust
use velox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Phase 3 Demo")
        .window(
            WindowConfig::new("main")
                .title("Velox — GPU Rendering")
                .size(1200, 800),
        )
        .run()
}
```

**Step 3: Run full verification**

Run: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace`

**Step 4: Update CLAUDE.md**

Add velox-render and velox-text to implemented crates. Update status to Phase 3 complete.

**Step 5: Commit**

```bash
git add crates/velox/ CLAUDE.md
git commit -m "feat: Phase 3 complete — GPU rendering, text, and keyboard shortcuts"
```
