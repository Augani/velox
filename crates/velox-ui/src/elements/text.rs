use crate::accessibility::{AccessibilityProps, AccessibleElement};
use crate::element::{
    AccessibilityInfo, AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest,
    PaintContext,
};
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::{PositionedGlyph, Rect};

pub struct TextElement {
    pub(crate) content: String,
    pub(crate) style: Style,
    pub(crate) accessibility: AccessibilityProps,
}

pub fn text(content: impl Into<String>) -> TextElement {
    TextElement {
        content: content.into(),
        style: Style::new(),
        accessibility: AccessibilityProps::default(),
    }
}

impl Styled for TextElement {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for TextElement {
    fn get_style(&self) -> &Style {
        &self.style
    }
}

impl AccessibleElement for TextElement {
    fn accessibility_props_mut(&mut self) -> &mut AccessibilityProps {
        &mut self.accessibility
    }
}

pub struct TextState {
    buffer: Option<velox_text::TextBuffer>,
    last_content: String,
    last_width: f32,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            buffer: None,
            last_content: String::new(),
            last_width: 0.0,
        }
    }
}

impl TextState {
    fn ensure_buffer(
        &mut self,
        content: &str,
        style: &Style,
        available_width: f32,
        font_system: &mut velox_text::FontSystem,
    ) {
        let font_size = style.font_size.unwrap_or(14.0);
        let line_height = style.line_height.unwrap_or(font_size * 1.2);

        let needs_rebuild = self.buffer.is_none()
            || self.last_content != content
            || (self.last_width - available_width).abs() > 0.5;

        if needs_rebuild {
            let mut buffer = velox_text::TextBuffer::new(font_system, font_size, line_height);

            let mut attrs = velox_text::TextAttrs {
                size: font_size,
                ..Default::default()
            };
            if let Some(ref family) = style.font_family {
                attrs.family = velox_text::FontFamily::Named(family.clone());
            }
            if let Some(weight) = style.font_weight {
                attrs.weight = weight.to_u16();
            }

            buffer.set_text(font_system, content, attrs);
            let buf_width = if available_width > 0.0 {
                available_width
            } else {
                f32::MAX
            };
            buffer.set_size(font_system, buf_width, f32::MAX);
            buffer.shape(font_system);

            self.buffer = Some(buffer);
            self.last_content = content.to_string();
            self.last_width = available_width;
        }
    }
}

impl Element for TextElement {
    type State = TextState;

    fn accessibility(
        &mut self,
        state: &mut TextState,
        _children: &[AnyElement],
    ) -> AccessibilityInfo {
        let text_runs = state
            .buffer
            .as_ref()
            .map(|buffer| buffer.accessibility_runs(&self.content))
            .unwrap_or_default()
            .into_iter()
            .map(|run| {
                velox_scene::AccessibilityTextRun::new(
                    run.text,
                    run.byte_start,
                    Rect::new(run.x, run.y, run.width, run.height),
                )
            })
            .collect();

        if self.accessibility.is_empty() {
            AccessibilityInfo {
                node: None,
                text_content: Some(self.content.clone()),
                text_runs,
            }
        } else {
            AccessibilityInfo {
                node: Some(self.accessibility.resolve(
                    velox_scene::AccessibilityRole::Label,
                    Some(self.content.clone()),
                    None,
                    false,
                )),
                text_content: Some(self.content.clone()),
                text_runs,
            }
        }
    }

    fn layout(
        &mut self,
        state: &mut TextState,
        _children: &[AnyElement],
        cx: &mut LayoutContext,
    ) -> LayoutRequest {
        let mut taffy_style = crate::layout_engine::convert_style(&self.style);

        let available_width = match &self.style.width {
            Some(crate::length::Length::Px(w)) => *w,
            _ => 0.0,
        };

        state.ensure_buffer(
            &self.content,
            &self.style,
            available_width,
            cx.font_system(),
        );

        if let Some(buffer) = &state.buffer
            && (self.style.width.is_none() || self.style.height.is_none())
        {
            let mut max_w: f32 = 0.0;
            let mut total_h: f32 = 0.0;
            for run in buffer.layout_runs() {
                let run_w: f32 = run.glyphs.iter().map(|g| g.w).sum();
                max_w = max_w.max(run_w);
                total_h = total_h.max(run.line_y + run.line_height);
            }
            if self.style.width.is_none() {
                taffy_style.size.width = taffy::Dimension::Length(max_w.ceil());
            }
            if self.style.height.is_none() {
                taffy_style.size.height = taffy::Dimension::Length(total_h.ceil());
            }
        }

        LayoutRequest { taffy_style }
    }

    fn paint(&mut self, state: &mut TextState, bounds: Rect, cx: &mut PaintContext) {
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }

        if self.content.is_empty() {
            return;
        }

        state.ensure_buffer(&self.content, &self.style, bounds.width, cx.font_system());

        let color = self
            .style
            .text_color
            .unwrap_or(velox_scene::Color::rgb(0, 0, 0));

        let Some(ref buffer) = state.buffer else {
            return;
        };

        let sf = cx.scale_factor();
        let mut glyphs = Vec::new();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), sf);

                let rasterized = cx
                    .glyph_rasterizer
                    .rasterize(cx.font_system, physical.cache_key);

                let Some(raster) = rasterized else {
                    continue;
                };
                if raster.width == 0 || raster.height == 0 {
                    continue;
                }

                cx.commands().upload_glyph(
                    physical.cache_key,
                    raster.width,
                    raster.height,
                    raster.data,
                );

                glyphs.push(PositionedGlyph {
                    cache_key: physical.cache_key,
                    x: bounds.x + physical.x as f32 / sf + raster.left as f32 / sf,
                    y: bounds.y + run.line_y + physical.y as f32 / sf - raster.top as f32 / sf,
                    width: raster.width as f32 / sf,
                    height: raster.height as f32 / sf,
                });
            }
        }

        if !glyphs.is_empty() {
            cx.commands().draw_glyphs(glyphs, color);
        }
    }
}

impl IntoElement for TextElement {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        self
    }
}

impl IntoAnyElement for TextElement {
    fn into_any_element(self) -> crate::element::AnyElement {
        crate::element::AnyElement::new(self, None, vec![])
    }
}

impl IntoElement for &str {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        text(self)
    }
}

impl IntoElement for String {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        text(self)
    }
}

impl IntoAnyElement for &str {
    fn into_any_element(self) -> crate::element::AnyElement {
        text(self).into_any_element()
    }
}

impl IntoAnyElement for String {
    fn into_any_element(self) -> crate::element::AnyElement {
        text(self).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::FontWeight;

    #[test]
    fn text_from_str() {
        let t = text("Hello");
        assert_eq!(t.content, "Hello");
    }

    #[test]
    fn text_styled() {
        let t = text("Hello")
            .text_lg()
            .font_weight(FontWeight::Bold)
            .text_color(velox_scene::Color::rgb(255, 255, 255));

        assert_eq!(t.style.font_size, Some(18.0));
        assert_eq!(t.style.font_weight, Some(FontWeight::Bold));
    }

    #[test]
    fn str_into_element() {
        let el: TextElement = "hello".into_element();
        assert_eq!(el.content, "hello");
    }

    #[test]
    fn string_into_element() {
        let el: TextElement = String::from("world").into_element();
        assert_eq!(el.content, "world");
    }

    #[test]
    fn paint_emits_draw_glyphs() {
        let mut el = text("Hi");
        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut glyph_rasterizer = velox_text::GlyphRasterizer::new();
        let mut state = TextState::default();

        let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut font_system,
            glyph_rasterizer: &mut glyph_rasterizer,
            hovered_node: None,
            active_node: None,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };
        el.paint(&mut state, bounds, &mut cx);

        let has_draw_glyphs = commands
            .commands()
            .iter()
            .any(|c| matches!(c, velox_scene::PaintCommand::DrawGlyphs { .. }));
        assert!(has_draw_glyphs);
    }

    #[test]
    fn empty_text_no_commands() {
        let mut el = text("");
        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut glyph_rasterizer = velox_text::GlyphRasterizer::new();
        let mut state = TextState::default();

        let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut font_system,
            glyph_rasterizer: &mut glyph_rasterizer,
            hovered_node: None,
            active_node: None,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };
        el.paint(&mut state, bounds, &mut cx);

        assert!(commands.commands().is_empty());
    }

    #[test]
    fn layout_computes_intrinsic_size() {
        let mut el = text("Hello World");
        let mut font_system = velox_text::FontSystem::new();
        let mut taffy = taffy::TaffyTree::new();
        let mut state = TextState::default();

        let mut cx = LayoutContext {
            taffy: &mut taffy,
            font_system: &mut font_system,
        };
        let req = el.layout(&mut state, &[], &mut cx);

        match req.taffy_style.size.width {
            taffy::Dimension::Length(w) => assert!(w > 0.0),
            _ => panic!("expected intrinsic width"),
        }
        match req.taffy_style.size.height {
            taffy::Dimension::Length(h) => assert!(h > 0.0),
            _ => panic!("expected intrinsic height"),
        }
    }
}
