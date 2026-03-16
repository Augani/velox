use crate::length::{Length, pct};
use crate::style::*;
use velox_scene::Color;

pub trait Styled: Sized {
    fn style_mut(&mut self) -> &mut Style;

    fn display(mut self, val: Display) -> Self {
        self.style_mut().display = Some(val);
        self
    }

    fn flex(self) -> Self {
        self.display(Display::Flex)
    }
    fn flex_row(mut self) -> Self {
        self.style_mut().display = Some(Display::Flex);
        self.style_mut().flex_direction = Some(FlexDirection::Row);
        self
    }
    fn flex_col(mut self) -> Self {
        self.style_mut().display = Some(Display::Flex);
        self.style_mut().flex_direction = Some(FlexDirection::Column);
        self
    }
    fn grid(self) -> Self {
        self.display(Display::Grid)
    }
    fn block(self) -> Self {
        self.display(Display::Block)
    }
    fn hidden(self) -> Self {
        self.display(Display::None)
    }

    fn w(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().width = Some(val.into());
        self
    }
    fn h(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().height = Some(val.into());
        self
    }
    fn size(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().width = Some(v);
        self.style_mut().height = Some(v);
        self
    }
    fn size_full(self) -> Self {
        self.w(pct(100.0)).h(pct(100.0))
    }
    fn min_w(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().min_width = Some(val.into());
        self
    }
    fn max_w(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().max_width = Some(val.into());
        self
    }
    fn min_h(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().min_height = Some(val.into());
        self
    }
    fn max_h(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().max_height = Some(val.into());
        self
    }
    fn w_full(self) -> Self {
        self.w(pct(100.0))
    }
    fn h_full(self) -> Self {
        self.h(pct(100.0))
    }
    fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.style_mut().aspect_ratio = Some(ratio);
        self
    }

    fn p(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        let s = self.style_mut();
        s.padding_top = Some(v);
        s.padding_right = Some(v);
        s.padding_bottom = Some(v);
        s.padding_left = Some(v);
        self
    }
    fn px_pad(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().padding_left = Some(v);
        self.style_mut().padding_right = Some(v);
        self
    }
    fn py(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().padding_top = Some(v);
        self.style_mut().padding_bottom = Some(v);
        self
    }
    fn pt(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_top = Some(val.into());
        self
    }
    fn pr(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_right = Some(val.into());
        self
    }
    fn pb(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_bottom = Some(val.into());
        self
    }
    fn pl(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_left = Some(val.into());
        self
    }

    fn m(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        let s = self.style_mut();
        s.margin_top = Some(v);
        s.margin_right = Some(v);
        s.margin_bottom = Some(v);
        s.margin_left = Some(v);
        self
    }
    fn mx(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().margin_left = Some(v);
        self.style_mut().margin_right = Some(v);
        self
    }
    fn my(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().margin_top = Some(v);
        self.style_mut().margin_bottom = Some(v);
        self
    }
    fn mt(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_top = Some(val.into());
        self
    }
    fn mr(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_right = Some(val.into());
        self
    }
    fn mb(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_bottom = Some(val.into());
        self
    }
    fn ml(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_left = Some(val.into());
        self
    }

    fn gap(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().gap = Some(val.into());
        self
    }
    fn row_gap(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().row_gap = Some(val.into());
        self
    }
    fn column_gap(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().column_gap = Some(val.into());
        self
    }

    fn flex_1(mut self) -> Self {
        self.style_mut().flex_grow = Some(1.0);
        self.style_mut().flex_shrink = Some(1.0);
        self.style_mut().flex_basis = Some(Length::Px(0.0));
        self
    }
    fn flex_grow(mut self) -> Self {
        self.style_mut().flex_grow = Some(1.0);
        self
    }
    fn flex_shrink(mut self) -> Self {
        self.style_mut().flex_shrink = Some(1.0);
        self
    }
    fn flex_none(mut self) -> Self {
        self.style_mut().flex_grow = Some(0.0);
        self.style_mut().flex_shrink = Some(0.0);
        self
    }
    fn flex_basis(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().flex_basis = Some(val.into());
        self
    }
    fn flex_wrap(mut self) -> Self {
        self.style_mut().flex_wrap = Some(FlexWrap::Wrap);
        self
    }

    fn items_start(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::Start);
        self
    }
    fn items_center(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::Center);
        self
    }
    fn items_end(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::End);
        self
    }
    fn items_stretch(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::Stretch);
        self
    }

    fn justify_start(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::Start);
        self
    }
    fn justify_center(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::Center);
        self
    }
    fn justify_end(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::End);
        self
    }
    fn justify_between(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::SpaceBetween);
        self
    }
    fn justify_around(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::SpaceAround);
        self
    }
    fn justify_evenly(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::SpaceEvenly);
        self
    }

    fn self_start(mut self) -> Self {
        self.style_mut().align_self = Some(AlignItems::Start);
        self
    }
    fn self_center(mut self) -> Self {
        self.style_mut().align_self = Some(AlignItems::Center);
        self
    }
    fn self_end(mut self) -> Self {
        self.style_mut().align_self = Some(AlignItems::End);
        self
    }
    fn order(mut self, val: i32) -> Self {
        self.style_mut().order = Some(val);
        self
    }

    fn grid_cols(mut self, cols: Vec<TrackSize>) -> Self {
        let s = self.style_mut();
        let tpl = s.grid_template.get_or_insert_with(|| GridTemplate {
            columns: vec![],
            rows: vec![],
        });
        tpl.columns = cols;
        self
    }
    fn grid_rows(mut self, rows: Vec<TrackSize>) -> Self {
        let s = self.style_mut();
        let tpl = s.grid_template.get_or_insert_with(|| GridTemplate {
            columns: vec![],
            rows: vec![],
        });
        tpl.rows = rows;
        self
    }
    fn grid_cols_count(self, count: usize) -> Self {
        self.grid_cols(vec![TrackSize::Fr(1.0); count])
    }
    fn grid_col_span(mut self, span: usize) -> Self {
        self.style_mut().grid_column_end = Some(span as i16 + 1);
        self.style_mut().grid_column_start = Some(1);
        self
    }
    fn grid_row_span(mut self, span: usize) -> Self {
        self.style_mut().grid_row_end = Some(span as i16 + 1);
        self.style_mut().grid_row_start = Some(1);
        self
    }

    fn relative(mut self) -> Self {
        self.style_mut().position = Some(Position::Relative);
        self
    }
    fn absolute(mut self) -> Self {
        self.style_mut().position = Some(Position::Absolute);
        self
    }
    fn inset(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        let s = self.style_mut();
        s.inset_top = Some(v);
        s.inset_right = Some(v);
        s.inset_bottom = Some(v);
        s.inset_left = Some(v);
        self
    }
    fn top(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_top = Some(val.into());
        self
    }
    fn right(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_right = Some(val.into());
        self
    }
    fn bottom(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_bottom = Some(val.into());
        self
    }
    fn left(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_left = Some(val.into());
        self
    }
    fn z_index(mut self, val: i32) -> Self {
        self.style_mut().z_index = Some(val);
        self
    }

    fn border(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else {
            return self;
        };
        let s = self.style_mut();
        s.border_top_width = Some(w);
        s.border_right_width = Some(w);
        s.border_bottom_width = Some(w);
        s.border_left_width = Some(w);
        self
    }
    fn border_t(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else {
            return self;
        };
        self.style_mut().border_top_width = Some(w);
        self
    }
    fn border_r(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else {
            return self;
        };
        self.style_mut().border_right_width = Some(w);
        self
    }
    fn border_b(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else {
            return self;
        };
        self.style_mut().border_bottom_width = Some(w);
        self
    }
    fn border_l(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else {
            return self;
        };
        self.style_mut().border_left_width = Some(w);
        self
    }
    fn border_color(mut self, color: impl Into<Color>) -> Self {
        self.style_mut().border_color = Some(color.into());
        self
    }
    fn border_dashed(mut self) -> Self {
        self.style_mut().border_style = Some(BorderStyle::Dashed);
        self
    }
    fn rounded(mut self, radius: impl Into<Length>) -> Self {
        let Length::Px(r) = radius.into() else {
            return self;
        };
        let s = self.style_mut();
        s.border_radius_tl = Some(r);
        s.border_radius_tr = Some(r);
        s.border_radius_bl = Some(r);
        s.border_radius_br = Some(r);
        self
    }
    fn rounded_t(mut self, radius: impl Into<Length>) -> Self {
        let Length::Px(r) = radius.into() else {
            return self;
        };
        self.style_mut().border_radius_tl = Some(r);
        self.style_mut().border_radius_tr = Some(r);
        self
    }
    fn rounded_b(mut self, radius: impl Into<Length>) -> Self {
        let Length::Px(r) = radius.into() else {
            return self;
        };
        self.style_mut().border_radius_bl = Some(r);
        self.style_mut().border_radius_br = Some(r);
        self
    }
    fn rounded_full(mut self) -> Self {
        let s = self.style_mut();
        s.border_radius_tl = Some(9999.0);
        s.border_radius_tr = Some(9999.0);
        s.border_radius_bl = Some(9999.0);
        s.border_radius_br = Some(9999.0);
        self
    }

    fn bg(mut self, color: impl Into<Color>) -> Self {
        self.style_mut().background = Some(color.into());
        self
    }

    fn shadow(mut self, shadow: BoxShadowStyle) -> Self {
        self.style_mut().box_shadows.push(shadow);
        self
    }

    fn opacity(mut self, val: f32) -> Self {
        self.style_mut().opacity = Some(val);
        self
    }

    fn overflow_hidden(mut self) -> Self {
        self.style_mut().overflow_x = Some(Overflow::Hidden);
        self.style_mut().overflow_y = Some(Overflow::Hidden);
        self
    }
    fn overflow_scroll(mut self) -> Self {
        self.style_mut().overflow_x = Some(Overflow::Scroll);
        self.style_mut().overflow_y = Some(Overflow::Scroll);
        self
    }
    fn overflow_y_scroll(mut self) -> Self {
        self.style_mut().overflow_y = Some(Overflow::Scroll);
        self
    }

    fn cursor_pointer(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::Pointer);
        self
    }
    fn cursor_text(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::Text);
        self
    }
    fn cursor_grab(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::Grab);
        self
    }
    fn cursor_not_allowed(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::NotAllowed);
        self
    }

    fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.style_mut().text_color = Some(color.into());
        self
    }
    fn text_size(mut self, size: impl Into<Length>) -> Self {
        let Length::Px(s) = size.into() else {
            return self;
        };
        self.style_mut().font_size = Some(s);
        self
    }
    fn text_xs(mut self) -> Self {
        self.style_mut().font_size = Some(12.0);
        self
    }
    fn text_sm(mut self) -> Self {
        self.style_mut().font_size = Some(14.0);
        self
    }
    fn text_base(mut self) -> Self {
        self.style_mut().font_size = Some(16.0);
        self
    }
    fn text_lg(mut self) -> Self {
        self.style_mut().font_size = Some(18.0);
        self
    }
    fn text_xl(mut self) -> Self {
        self.style_mut().font_size = Some(20.0);
        self
    }

    fn font_weight(mut self, weight: FontWeight) -> Self {
        self.style_mut().font_weight = Some(weight);
        self
    }
    fn line_height(mut self, val: f32) -> Self {
        self.style_mut().line_height = Some(val);
        self
    }
    fn letter_spacing(mut self, val: f32) -> Self {
        self.style_mut().letter_spacing = Some(val);
        self
    }
    fn text_align(mut self, align: TextAlign) -> Self {
        self.style_mut().text_align = Some(align);
        self
    }
    fn text_ellipsis(mut self) -> Self {
        self.style_mut().text_overflow = Some(TextOverflow::Ellipsis);
        self
    }
    fn text_wrap(mut self) -> Self {
        self.style_mut().text_overflow = Some(TextOverflow::Wrap);
        self
    }
    fn text_nowrap(mut self) -> Self {
        self.style_mut().text_overflow = Some(TextOverflow::NoWrap);
        self
    }
    fn text_decoration(mut self, decoration: TextDecoration) -> Self {
        self.style_mut().text_decoration = Some(decoration);
        self
    }

    fn bg_linear_gradient(mut self, angle: f32, stops: Vec<(f32, velox_scene::Color)>) -> Self {
        self.style_mut().background_gradient = Some(velox_scene::Gradient::Linear {
            angle_deg: angle,
            stops: stops
                .into_iter()
                .map(|(offset, color)| velox_scene::GradientStop { offset, color })
                .collect(),
        });
        self
    }

    fn bg_radial_gradient(mut self, stops: Vec<(f32, velox_scene::Color)>) -> Self {
        self.style_mut().background_gradient = Some(velox_scene::Gradient::Radial {
            center_x: 0.5,
            center_y: 0.5,
            stops: stops
                .into_iter()
                .map(|(offset, color)| velox_scene::GradientStop { offset, color })
                .collect(),
        });
        self
    }

    fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        if condition { then(self) } else { self }
    }

    fn apply(mut self, refinement: &Style) -> Self {
        self.style_mut().merge(refinement);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;

    struct TestBox {
        style: Style,
    }
    impl TestBox {
        fn new() -> Self {
            Self {
                style: Style::new(),
            }
        }
    }
    impl Styled for TestBox {
        fn style_mut(&mut self) -> &mut Style {
            &mut self.style
        }
    }

    #[test]
    fn flex_row_sets_display_and_direction() {
        let b = TestBox::new().flex_row();
        assert_eq!(b.style.display, Some(Display::Flex));
        assert_eq!(b.style.flex_direction, Some(FlexDirection::Row));
    }

    #[test]
    fn chained_sizing() {
        let b = TestBox::new().w(px(100.0)).h(px(50.0));
        assert_eq!(b.style.width, Some(Length::Px(100.0)));
        assert_eq!(b.style.height, Some(Length::Px(50.0)));
    }

    #[test]
    fn padding_shorthand() {
        let b = TestBox::new().p(px(16.0));
        assert_eq!(b.style.padding_top, Some(Length::Px(16.0)));
        assert_eq!(b.style.padding_right, Some(Length::Px(16.0)));
        assert_eq!(b.style.padding_bottom, Some(Length::Px(16.0)));
        assert_eq!(b.style.padding_left, Some(Length::Px(16.0)));
    }

    #[test]
    fn flex_1_sets_grow_shrink_basis() {
        let b = TestBox::new().flex_1();
        assert_eq!(b.style.flex_grow, Some(1.0));
        assert_eq!(b.style.flex_shrink, Some(1.0));
        assert_eq!(b.style.flex_basis, Some(Length::Px(0.0)));
    }

    #[test]
    fn conditional_when() {
        let b = TestBox::new()
            .bg(Color::rgb(255, 0, 0))
            .when(true, |s| s.opacity(0.5))
            .when(false, |s| s.hidden());
        assert_eq!(b.style.opacity, Some(0.5));
        assert_ne!(b.style.display, Some(Display::None));
    }

    #[test]
    fn rounded_full() {
        let b = TestBox::new().rounded_full();
        assert_eq!(b.style.border_radius_tl, Some(9999.0));
        assert_eq!(b.style.border_radius_br, Some(9999.0));
    }

    #[test]
    fn grid_cols_count() {
        let b = TestBox::new().grid().grid_cols_count(3);
        let tpl = b.style.grid_template.unwrap();
        assert_eq!(tpl.columns.len(), 3);
        assert_eq!(tpl.columns[0], TrackSize::Fr(1.0));
    }

    #[test]
    fn size_full_shorthand() {
        let b = TestBox::new().size_full();
        assert_eq!(b.style.width, Some(Length::Pct(100.0)));
        assert_eq!(b.style.height, Some(Length::Pct(100.0)));
    }
}
