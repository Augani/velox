use velox_scene::{Color, Gradient, GradientStop, Point, Rect};

pub(crate) fn modulate_color(color: Color, opacity: f32) -> Color {
    let alpha = ((color.a as f32) * opacity.clamp(0.0, 1.0)).round();
    Color::rgba(color.r, color.g, color.b, alpha.clamp(0.0, 255.0) as u8)
}

pub(crate) fn sample_gradient_color(gradient: &Gradient, rect: Rect, x: f32, y: f32) -> Color {
    match gradient {
        Gradient::Linear { angle_deg, stops } => {
            sample_stops(stops, linear_gradient_t(rect, *angle_deg, x, y))
        }
        Gradient::Radial {
            center_x,
            center_y,
            stops,
        } => sample_stops(stops, radial_gradient_t(rect, *center_x, *center_y, x, y)),
    }
}

pub(crate) fn shadow_layers(
    rect: Rect,
    blur_radius: f32,
    offset: Point,
    spread: f32,
) -> Vec<(Rect, f32)> {
    let base = Rect::new(
        rect.x + offset.x - spread,
        rect.y + offset.y - spread,
        (rect.width + spread * 2.0).max(0.0),
        (rect.height + spread * 2.0).max(0.0),
    );

    if blur_radius <= f32::EPSILON {
        return vec![(base, 1.0)];
    }

    let steps = ((blur_radius / 4.0).ceil() as usize).clamp(1, 10);
    let mut weights = Vec::with_capacity(steps + 1);
    weights.push(1.0);
    for i in 0..steps {
        let t = (i as f32 + 1.0) / (steps as f32 + 1.0);
        weights.push((1.0 - t).powf(2.0));
    }
    let total_weight: f32 = weights.iter().sum();

    let mut layers = Vec::with_capacity(weights.len());
    layers.push((base, weights[0] / total_weight));
    for i in 0..steps {
        let expansion = blur_radius * ((i + 1) as f32 / steps as f32);
        layers.push((
            Rect::new(
                base.x - expansion,
                base.y - expansion,
                base.width + expansion * 2.0,
                base.height + expansion * 2.0,
            ),
            weights[i + 1] / total_weight,
        ));
    }
    layers
}

fn sample_stops(stops: &[GradientStop], t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match stops {
        [] => Color::rgba(0, 0, 0, 0),
        [single] => single.color,
        _ => {
            if t <= stops[0].offset {
                return stops[0].color;
            }

            for window in stops.windows(2) {
                let a = &window[0];
                let b = &window[1];
                if t <= b.offset {
                    let span = (b.offset - a.offset).max(f32::EPSILON);
                    let local_t = (t - a.offset) / span;
                    return interpolate_color(a.color, b.color, local_t);
                }
            }

            stops
                .last()
                .map(|stop| stop.color)
                .unwrap_or(Color::rgba(0, 0, 0, 0))
        }
    }
}

fn interpolate_color(a: Color, b: Color, t: f32) -> Color {
    let lerp = |start: u8, end: u8| -> u8 {
        (start as f32 + (end as f32 - start as f32) * t.clamp(0.0, 1.0)).round() as u8
    };

    Color::rgba(
        lerp(a.r, b.r),
        lerp(a.g, b.g),
        lerp(a.b, b.b),
        lerp(a.a, b.a),
    )
}

fn linear_gradient_t(rect: Rect, angle_deg: f32, x: f32, y: f32) -> f32 {
    let theta = angle_deg.to_radians();
    let dx = theta.cos();
    let dy = theta.sin();
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x, rect.y + rect.height),
        (rect.x + rect.width, rect.y + rect.height),
    ];

    let mut min_proj = f32::INFINITY;
    let mut max_proj = f32::NEG_INFINITY;
    for (cx, cy) in corners {
        let proj = cx * dx + cy * dy;
        min_proj = min_proj.min(proj);
        max_proj = max_proj.max(proj);
    }

    if (max_proj - min_proj).abs() <= f32::EPSILON {
        return 0.0;
    }

    ((x * dx + y * dy) - min_proj) / (max_proj - min_proj)
}

fn radial_gradient_t(rect: Rect, center_x: f32, center_y: f32, x: f32, y: f32) -> f32 {
    let cx = rect.x + rect.width * center_x;
    let cy = rect.y + rect.height * center_y;
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x, rect.y + rect.height),
        (rect.x + rect.width, rect.y + rect.height),
    ];

    let max_distance = corners
        .into_iter()
        .map(|(px, py)| {
            let dx = px - cx;
            let dy = py - cy;
            (dx * dx + dy * dy).sqrt()
        })
        .fold(0.0, f32::max);

    if max_distance <= f32::EPSILON {
        return 0.0;
    }

    let dx = x - cx;
    let dy = y - cy;
    (dx * dx + dy * dy).sqrt() / max_distance
}

pub(crate) fn effective_layer_opacity(layer_stack: &[f32]) -> f32 {
    layer_stack
        .iter()
        .copied()
        .fold(1.0, |opacity, layer| opacity * layer)
        .clamp(0.0, 1.0)
}

pub(crate) fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.width).min(b.x + b.width);
    let y1 = (a.y + a.height).min(b.y + b.height);
    let width = x1 - x0;
    let height = y1 - y0;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(Rect::new(x0, y0, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_scene::Gradient;

    #[test]
    fn modulate_color_scales_alpha() {
        let color = Color::rgba(20, 40, 60, 200);
        assert_eq!(modulate_color(color, 0.5), Color::rgba(20, 40, 60, 100));
    }

    #[test]
    fn linear_gradient_sample_interpolates_stops() {
        let gradient = Gradient::Linear {
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
        };

        let color = sample_gradient_color(&gradient, Rect::new(0.0, 0.0, 100.0, 20.0), 50.0, 10.0);
        assert!(color.r > 100 && color.r < 200);
        assert_eq!(color.r, color.g);
        assert_eq!(color.g, color.b);
    }

    #[test]
    fn shadow_layers_expand_around_offset_rect() {
        let layers = shadow_layers(
            Rect::new(10.0, 20.0, 50.0, 30.0),
            8.0,
            Point::new(2.0, 4.0),
            3.0,
        );
        assert!(layers.len() > 1);
        assert_eq!(layers[0].0, Rect::new(9.0, 21.0, 56.0, 36.0));
        assert!(layers.last().expect("shadow layers").0.width > layers[0].0.width);
    }
}
