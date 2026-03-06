# Phase 3: Rendering, Text, and Input — Design

## Overview

Phase 3 adds GPU rendering via wgpu, text shaping/rendering via cosmic-text, a glyph atlas for efficient text display, and a keyboard shortcut system. This phase turns the abstract paint commands from Phase 2 into real pixels on screen.

**Scope:** wgpu rendering pipeline, text shaping/measurement, glyph atlas, keyboard shortcuts.
**Deferred:** IME, text editing, selection, command routing through focus chain.

## New Crates

### `velox-render`

GPU rendering backend. Owns wgpu setup and the rendering pipeline.

**Dependencies:** `velox-scene` (for Rect, Color, CommandList, PaintCommand), `velox-text` (for glyph rasterization), `wgpu`

**Components:**

- **`GpuContext`** — owns `wgpu::Instance`, `wgpu::Adapter`, `wgpu::Device`, `wgpu::Queue`. Created once at app startup.
- **`WindowSurface`** — per-window `wgpu::Surface` + `SurfaceConfiguration`. Created when a window opens, resized on window resize.
- **`Renderer`** — consumes a `CommandList` and renders to a surface:
  - Colored rect rendering (vertex buffer with position + color)
  - Textured quad rendering (for glyphs from the atlas)
  - Clip stack via scissor rects
- **`GlyphAtlas`** — texture atlas for rasterized glyphs. Receives glyph bitmaps from velox-text, packs into a GPU texture using shelf-packing, returns UV coordinates per glyph.

**Render pipeline:**
1. `scene.layout()` then `scene.paint()` → produces `CommandList`
2. `renderer.render(surface, command_list, glyph_atlas)` → GPU draw calls
3. Present the frame

### `velox-text`

Text shaping, measurement, and glyph rasterization. No GPU knowledge.

**Dependencies:** `cosmic-text`

**Components:**

- **`FontSystem`** — wraps `cosmic_text::FontSystem`. System font discovery and fallback.
- **`TextBuffer`** — wraps `cosmic_text::Buffer`. Laid-out text block:
  - `set_text(&mut self, text: &str, attrs: TextAttrs)`
  - `set_size(&mut self, width: f32, height: f32)`
  - `shape(&mut self, font_system: &mut FontSystem)`
  - `layout_runs(&self)` — iterate shaped glyph runs
- **`TextAttrs`** — font family, size, weight, style, line height
- **`GlyphRasterizer`** — uses `cosmic_text::SwashCache` to rasterize glyphs into alpha-mask bitmaps
- **`TextMetrics`** — measurement result: total width, height, line count

**Usage flow:**
1. Create `TextBuffer`, set text and size constraints
2. Call `shape()` for shaping/layout
3. Read `layout_runs()` for positioned glyphs
4. Check glyph atlas; rasterize missing glyphs via `GlyphRasterizer`
5. Emit `DrawGlyphs` paint command

## Modified Crates

### `velox-scene`

**New paint commands:**
- `DrawGlyphs { glyphs: Vec<GlyphInstance>, color: Color }` — positioned glyph references
- `GlyphInstance { glyph_id: GlyphId, x: f32, y: f32, width: f32, height: f32 }`
- `GlyphId` — unique identifier for a shaped glyph (font + glyph index + size)

**New module: `shortcut.rs`**

Keyboard shortcut registry:

- `KeyCombo` — key + modifiers
- `Modifiers` — bitflags: Shift, Ctrl, Alt, Super
- `Key` — enum of logical key values (A-Z, 0-9, F1-F12, Enter, Escape, Tab, arrows, etc.)
- `ShortcutId` — handle for unregistering
- `ShortcutRegistry`:
  - `register(combo: KeyCombo, callback: impl FnMut() + 'static) -> ShortcutId`
  - `unregister(id: ShortcutId)`
  - `handle_key_event(key: Key, modifiers: Modifiers) -> bool` — returns true if consumed

### `velox-app`

**VeloxHandler changes:**
- Owns `GpuContext` (created in `resumed()`)
- Each window gets a `WindowSurface` alongside its `Scene`
- `RedrawRequested`: layout → paint → render → present
- `Resized`: reconfigure window surface
- `KeyboardInput`: check shortcut registry first, then route to scene

### `velox` (facade)

Re-export `velox-render` and `velox-text`. Update prelude.

## Dependency Flow

```
velox-app → velox-render → velox-text
         → velox-scene      velox-scene
         → velox-window
         → velox-runtime
         → velox-reactive
```

## Technology

- **wgpu** — cross-platform GPU abstraction (Vulkan/Metal/DX12/OpenGL)
- **cosmic-text** — text shaping, layout, font fallback, glyph rasterization

## Success Criteria

Phase 3 is done when:
1. `velox-render` and `velox-text` compile, all tests pass
2. wgpu initializes and creates surfaces per window
3. Colored rects render correctly via GPU
4. Text renders via glyph atlas (cosmic-text → rasterize → atlas → textured quads)
5. Window resize reconfigures surfaces correctly
6. Keyboard shortcuts register and fire callbacks
7. Demo app shows colored rectangles and text in a window
