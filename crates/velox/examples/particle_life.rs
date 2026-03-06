use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use velox::prelude::*;
use velox::scene::{
    ButtonState, Color, CommandList, EventContext, EventHandler, Key, KeyEvent, MouseButton,
    MouseEvent, Painter, ScrollEvent,
};
use velox::style::{Theme, ThemeManager};

const W: f32 = 1200.0;
const H: f32 = 800.0;
const NUM_SPECIES: usize = 6;
const PARTICLES_PER_SPECIES: usize = 200;
const TOTAL_PARTICLES: usize = NUM_SPECIES * PARTICLES_PER_SPECIES;
const INTERACTION_RADIUS: f32 = 80.0;
const FRICTION: f32 = 0.92;
const DT: f32 = 0.016;
const FORCE_SCALE: f32 = 8.0;
const PARTICLE_RADIUS: f32 = 2.5;

const SPECIES_COLORS: [Color; NUM_SPECIES] = [
    Color {
        r: 255,
        g: 60,
        b: 80,
        a: 255,
    },
    Color {
        r: 60,
        g: 220,
        b: 120,
        a: 255,
    },
    Color {
        r: 60,
        g: 140,
        b: 255,
        a: 255,
    },
    Color {
        r: 255,
        g: 200,
        b: 40,
        a: 255,
    },
    Color {
        r: 200,
        g: 80,
        b: 255,
        a: 255,
    },
    Color {
        r: 255,
        g: 140,
        b: 60,
        a: 255,
    },
];

fn simple_hash(seed: u64) -> u64 {
    let mut x = seed;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

fn hash_f32(seed: u64) -> f32 {
    (simple_hash(seed) & 0xFFFFFF) as f32 / 0xFFFFFF as f32
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    species: usize,
}

struct SimState {
    particles: Vec<Particle>,
    interaction_matrix: [[f32; NUM_SPECIES]; NUM_SPECIES],
    rng_counter: u64,
    last_tick: Instant,
    frame_count: u64,
    fps: f32,
    fps_update_time: Instant,
    fps_frame_count: u64,
    paused: bool,
    show_trails: bool,
    trail_alpha: u8,
    zoom: f32,
}

impl SimState {
    fn new() -> Self {
        let mut rng_counter = 42u64;
        let mut particles = Vec::with_capacity(TOTAL_PARTICLES);

        for species in 0..NUM_SPECIES {
            for _ in 0..PARTICLES_PER_SPECIES {
                let x = hash_f32(rng_counter) * W;
                rng_counter += 1;
                let y = hash_f32(rng_counter) * H;
                rng_counter += 1;
                particles.push(Particle {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                    species,
                });
            }
        }

        let mut interaction_matrix = [[0.0f32; NUM_SPECIES]; NUM_SPECIES];
        for row in &mut interaction_matrix {
            for cell in row.iter_mut() {
                *cell = hash_f32(rng_counter) * 2.0 - 1.0;
                rng_counter += 1;
            }
        }

        Self {
            particles,
            interaction_matrix,
            rng_counter,
            last_tick: Instant::now(),
            frame_count: 0,
            fps: 0.0,
            fps_update_time: Instant::now(),
            fps_frame_count: 0,
            paused: false,
            show_trails: true,
            trail_alpha: 20,
            zoom: 1.0,
        }
    }

    fn randomize_matrix(&mut self) {
        for row in &mut self.interaction_matrix {
            for cell in row.iter_mut() {
                *cell = hash_f32(self.rng_counter) * 2.0 - 1.0;
                self.rng_counter += 1;
            }
        }
    }

    fn symmetrize_matrix(&mut self) {
        for i in 0..NUM_SPECIES {
            for j in (i + 1)..NUM_SPECIES {
                let avg = (self.interaction_matrix[i][j] + self.interaction_matrix[j][i]) / 2.0;
                self.interaction_matrix[i][j] = avg;
                self.interaction_matrix[j][i] = avg;
            }
        }
    }

    fn tick(&mut self) {
        if self.paused {
            self.last_tick = Instant::now();
            return;
        }

        let now = Instant::now();
        self.fps_frame_count += 1;
        let fps_elapsed = now.duration_since(self.fps_update_time).as_secs_f32();
        if fps_elapsed >= 0.5 {
            self.fps = self.fps_frame_count as f32 / fps_elapsed;
            self.fps_frame_count = 0;
            self.fps_update_time = now;
        }
        self.last_tick = now;
        self.frame_count += 1;

        let count = self.particles.len();
        let mut forces = vec![(0.0f32, 0.0f32); count];

        for i in 0..count {
            let px = self.particles[i].x;
            let py = self.particles[i].y;
            let si = self.particles[i].species;

            for j in (i + 1)..count {
                let qx = self.particles[j].x;
                let qy = self.particles[j].y;
                let sj = self.particles[j].species;

                let mut dx = qx - px;
                let mut dy = qy - py;

                if dx > W / 2.0 {
                    dx -= W;
                }
                if dx < -W / 2.0 {
                    dx += W;
                }
                if dy > H / 2.0 {
                    dy -= H;
                }
                if dy < -H / 2.0 {
                    dy += H;
                }

                let dist_sq = dx * dx + dy * dy;
                let radius_sq = INTERACTION_RADIUS * INTERACTION_RADIUS;

                if dist_sq > radius_sq || dist_sq < 1.0 {
                    continue;
                }

                let dist = dist_sq.sqrt();
                let norm_dist = dist / INTERACTION_RADIUS;

                let attraction_ij = self.interaction_matrix[si][sj];
                let attraction_ji = self.interaction_matrix[sj][si];

                let force_magnitude = if norm_dist < 0.3 {
                    (norm_dist / 0.3) - 1.0
                } else if norm_dist < 0.7 {
                    (norm_dist - 0.3) / 0.4
                } else {
                    1.0 - (norm_dist - 0.7) / 0.3
                };

                let inv_dist = 1.0 / dist;
                let nx = dx * inv_dist;
                let ny = dy * inv_dist;

                let fi = force_magnitude * attraction_ij * FORCE_SCALE;
                let fj = force_magnitude * attraction_ji * FORCE_SCALE;

                forces[i].0 += nx * fi;
                forces[i].1 += ny * fi;
                forces[j].0 -= nx * fj;
                forces[j].1 -= ny * fj;
            }
        }

        for (i, particle) in self.particles.iter_mut().enumerate() {
            particle.vx = (particle.vx + forces[i].0 * DT) * FRICTION;
            particle.vy = (particle.vy + forces[i].1 * DT) * FRICTION;

            particle.x += particle.vx;
            particle.y += particle.vy;

            if particle.x < 0.0 {
                particle.x += W;
            }
            if particle.x >= W {
                particle.x -= W;
            }
            if particle.y < 0.0 {
                particle.y += H;
            }
            if particle.y >= H {
                particle.y -= H;
            }
        }
    }
}

struct SimPainter {
    state: Rc<RefCell<SimState>>,
}

impl Painter for SimPainter {
    fn paint(&self, rect: Rect, commands: &mut CommandList) {
        let mut state = self.state.borrow_mut();
        state.tick();

        if state.show_trails {
            commands.fill_rect(rect, Color::rgba(10, 10, 14, state.trail_alpha));
        } else {
            commands.fill_rect(rect, Color::rgb(10, 10, 14));
        }

        let zoom = state.zoom;
        let cx = W / 2.0;
        let cy = H / 2.0;

        for particle in &state.particles {
            let px = (particle.x - cx) * zoom + cx + rect.x;
            let py = (particle.y - cy) * zoom + cy + rect.y;
            let radius = PARTICLE_RADIUS * zoom;

            if px + radius < rect.x || px - radius > rect.x + rect.width {
                continue;
            }
            if py + radius < rect.y || py - radius > rect.y + rect.height {
                continue;
            }

            let color = SPECIES_COLORS[particle.species];

            let speed = (particle.vx * particle.vx + particle.vy * particle.vy).sqrt();
            let glow = (speed * 15.0).min(60.0) as u8;

            if glow > 10 {
                let glow_size = radius * 3.0;
                commands.fill_rect(
                    Rect::new(
                        px - glow_size / 2.0,
                        py - glow_size / 2.0,
                        glow_size,
                        glow_size,
                    ),
                    Color::rgba(color.r, color.g, color.b, glow),
                );
            }

            commands.fill_rect(
                Rect::new(px - radius, py - radius, radius * 2.0, radius * 2.0),
                color,
            );
        }

        let matrix = &state.interaction_matrix;
        let cell_size = 14.0;
        let matrix_x = rect.x + rect.width - (NUM_SPECIES as f32 * cell_size) - 16.0;
        let matrix_y = rect.y + 16.0;

        commands.fill_rect(
            Rect::new(
                matrix_x - 4.0,
                matrix_y - 4.0,
                NUM_SPECIES as f32 * cell_size + 8.0,
                NUM_SPECIES as f32 * cell_size + 8.0,
            ),
            Color::rgba(0, 0, 0, 160),
        );

        for (i, row) in matrix.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                let color = if val > 0.0 {
                    let intensity = (val * 255.0).min(255.0) as u8;
                    Color::rgba(0, intensity, 60, 200)
                } else {
                    let intensity = (-val * 255.0).min(255.0) as u8;
                    Color::rgba(intensity, 0, 40, 200)
                };
                commands.fill_rect(
                    Rect::new(
                        matrix_x + j as f32 * cell_size,
                        matrix_y + i as f32 * cell_size,
                        cell_size - 1.0,
                        cell_size - 1.0,
                    ),
                    color,
                );
            }
        }

        for (idx, species_color) in SPECIES_COLORS.iter().enumerate() {
            commands.fill_rect(
                Rect::new(
                    matrix_x - 18.0,
                    matrix_y + idx as f32 * cell_size + 2.0,
                    10.0,
                    10.0,
                ),
                *species_color,
            );
            commands.fill_rect(
                Rect::new(
                    matrix_x + idx as f32 * cell_size + 2.0,
                    matrix_y - 18.0,
                    10.0,
                    10.0,
                ),
                *species_color,
            );
        }

        let hud_y = rect.y + rect.height - 36.0;
        commands.fill_rect(
            Rect::new(rect.x, hud_y, rect.width, 36.0),
            Color::rgba(0, 0, 0, 180),
        );

        let fps = state.fps;
        let particle_count = state.particles.len();
        let paused = state.paused;
        let trails = state.show_trails;

        let bar_w = 120.0;
        let bar_h = 8.0;
        let bar_x = rect.x + 16.0;
        let bar_y = hud_y + 14.0;
        commands.fill_rect(
            Rect::new(bar_x, bar_y, bar_w, bar_h),
            Color::rgba(40, 40, 50, 255),
        );
        let fps_frac = (fps / 120.0).min(1.0);
        let fps_color = if fps > 55.0 {
            Color::rgb(60, 220, 120)
        } else if fps > 30.0 {
            Color::rgb(255, 200, 40)
        } else {
            Color::rgb(255, 60, 80)
        };
        commands.fill_rect(Rect::new(bar_x, bar_y, bar_w * fps_frac, bar_h), fps_color);

        let status_indicators = [
            (
                bar_x + bar_w + 20.0,
                if paused {
                    Color::rgb(255, 60, 80)
                } else {
                    Color::rgb(60, 220, 120)
                },
            ),
            (
                bar_x + bar_w + 40.0,
                if trails {
                    Color::rgb(60, 140, 255)
                } else {
                    Color::rgb(60, 60, 70)
                },
            ),
        ];
        for (sx, sc) in &status_indicators {
            commands.fill_rect(Rect::new(*sx, hud_y + 12.0, 12.0, 12.0), *sc);
        }

        drop(state);

        let _ = (fps, particle_count, paused, trails);
    }
}

struct SimHandler {
    state: Rc<RefCell<SimState>>,
}

impl EventHandler for SimHandler {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool {
        if !event.state.is_pressed() {
            return false;
        }
        match event.key {
            Key::Space => {
                self.state.borrow_mut().paused = !self.state.borrow().paused;
                ctx.request_redraw();
                true
            }
            Key::R => {
                self.state.borrow_mut().randomize_matrix();
                ctx.request_redraw();
                true
            }
            Key::M => {
                self.state.borrow_mut().symmetrize_matrix();
                ctx.request_redraw();
                true
            }
            Key::T => {
                let mut s = self.state.borrow_mut();
                s.show_trails = !s.show_trails;
                ctx.request_redraw();
                true
            }
            Key::P => {
                let mut s = self.state.borrow_mut();
                s.trail_alpha = (s.trail_alpha + 5).min(80);
                ctx.request_redraw();
                true
            }
            Key::O => {
                let mut s = self.state.borrow_mut();
                s.trail_alpha = s.trail_alpha.saturating_sub(5).max(5);
                ctx.request_redraw();
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
        if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
            let mut s = self.state.borrow_mut();
            let mx = event.position.x;
            let my = event.position.y;
            for _ in 0..30 {
                let species = (simple_hash(s.rng_counter) as usize) % NUM_SPECIES;
                s.rng_counter += 1;
                let angle = hash_f32(s.rng_counter) * std::f32::consts::TAU;
                s.rng_counter += 1;
                let spread = hash_f32(s.rng_counter) * 30.0;
                s.rng_counter += 1;

                s.particles.push(Particle {
                    x: mx + angle.cos() * spread,
                    y: my + angle.sin() * spread,
                    vx: 0.0,
                    vy: 0.0,
                    species,
                });
            }
            ctx.request_redraw();
            return true;
        }
        false
    }

    fn handle_scroll(&mut self, event: &ScrollEvent, ctx: &mut EventContext) -> bool {
        let mut s = self.state.borrow_mut();
        s.zoom = (s.zoom + event.delta_y * 0.05).clamp(0.3, 5.0);
        ctx.request_redraw();
        true
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ThemeManager::new(Theme::generated_default());

    let state = Rc::new(RefCell::new(SimState::new()));

    let painter_state = state.clone();
    let handler_state = state.clone();

    App::new()
        .name("Particle Life")
        .power_policy(PowerPolicy::Adaptive)
        .theme_manager(manager)
        .window(
            WindowConfig::new("main")
                .title("Velox — Particle Life Simulation")
                .size(W as u32, H as u32)
                .min_size(640, 480),
        )
        .setup(move |scene| {
            let root = scene.tree_mut().insert(None);
            scene.tree_mut().set_rect(root, Rect::new(0.0, 0.0, W, H));

            scene.tree_mut().set_painter(
                root,
                SimPainter {
                    state: painter_state,
                },
            );
            scene.tree_mut().set_event_handler(
                root,
                SimHandler {
                    state: handler_state,
                },
            );
            scene.focus_mut().request_focus(root);
        })
        .run()
}
