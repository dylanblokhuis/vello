use vello_common::pixmap::Pixmap;
use vello_cpu::{
    RenderContext, RenderSettings,
    kurbo::{Affine, BezPath, Point, Rect, Shape, Size},
    peniko::{
        BlendMode, Brush, Fill,
        color::{AlphaColor, Srgb},
    },
};

use crate::{
    backend::{Backend, BackendRun, BenchParams, BenchRandom, TimerGuard},
    shapes,
    tests::{RenderOp, ShapeKind, TestKind},
};

const COORD_SEED: u64 = 0x19AE0DDAE3FA7391;
const COLOR_SEED: u64 = 0x94BD7A499AD10011;
const EXTRA_SEED: u64 = 0x1ABD9CC9CAF0F123;

pub struct VelloBackend {
    name: String,
    settings: RenderSettings,
    ctx: RenderContext,
    surface: Pixmap,
    width: u16,
    height: u16,
    coord_rng: BenchRandom,
    color_rng: BenchRandom,
    extra_rng: BenchRandom,
}

impl VelloBackend {
    pub fn new(width: u32, height: u32, threads: u16) -> Self {
        let mut settings = RenderSettings::default();
        settings.num_threads = threads;
        let width_u16 = width as u16;
        let height_u16 = height as u16;
        let name = if threads == 0 {
            "Vello CPU ST".to_string()
        } else {
            format!("Vello CPU {}T", threads)
        };
        let mut ctx = RenderContext::new_with(width_u16, height_u16, settings);
        let mut stroke = ctx.stroke().clone();
        stroke.width = 2.0;
        ctx.set_stroke(stroke);
        Self {
            name,
            settings,
            ctx,
            surface: Pixmap::new(width_u16, height_u16),
            width: width_u16,
            height: height_u16,
            coord_rng: BenchRandom::new(COORD_SEED),
            color_rng: BenchRandom::new(COLOR_SEED),
            extra_rng: BenchRandom::new(EXTRA_SEED),
        }
    }

    fn reset_state(&mut self) {
        self.coord_rng.rewind();
        self.color_rng.rewind();
        self.extra_rng.rewind();
    }

    fn ensure_context(&mut self, screen_w: u32, screen_h: u32) {
        let target_w = screen_w as u16;
        let target_h = screen_h as u16;
        if target_w != self.width || target_h != self.height {
            self.width = target_w;
            self.height = target_h;
            self.ctx = RenderContext::new_with(self.width, self.height, self.settings);
            self.surface = Pixmap::new(self.width, self.height);
        }
    }

    fn prepare_context(&mut self, params: &BenchParams) {
        self.ensure_context(
            params.screen_size.width as u32,
            params.screen_size.height as u32,
        );
        self.ctx.reset();
        let mut stroke = self.ctx.stroke().clone();
        stroke.width = params.stroke_width;
        self.ctx.set_stroke(stroke);
        if let Some(mode) = params.comp_op.mode {
            self.ctx.set_blend_mode(mode);
        } else {
            self.ctx.set_blend_mode(BlendMode::default());
        }
    }

    fn random_color(&mut self) -> AlphaColor<Srgb> {
        let value = self.color_rng.next_color();
        let components = [
            ((value >> 16) & 0xFF) as f32 / 255.0,
            ((value >> 8) & 0xFF) as f32 / 255.0,
            (value & 0xFF) as f32 / 255.0,
            ((value >> 24) & 0xFF) as f32 / 255.0,
        ];
        AlphaColor::new(components)
    }

    fn render_rect_aligned(&mut self, params: &BenchParams) {
        let bounds_x = (self.width as i32 - params.shape_size as i32).max(1);
        let bounds_y = (self.height as i32 - params.shape_size as i32).max(1);
        for _ in 0..params.quantity {
            let x = self.coord_rng.next_i32(0, bounds_x);
            let y = self.coord_rng.next_i32(0, bounds_y);
            let rect = Rect::from_origin_size(
                (x as f64, y as f64),
                (params.shape_size as f64, params.shape_size as f64),
            );
            let color = self.random_color();
            {
                let ctx = &mut self.ctx;
                ctx.set_paint(Brush::Solid(color));
                match params.test.render_op() {
                    RenderOp::Stroke => ctx.stroke_rect(&rect),
                    _ => ctx.fill_rect(&rect),
                }
            }
        }
    }

    fn render_rect_floating(&mut self, params: &BenchParams) {
        let bounds = Size::new(
            (self.width - params.shape_size as u16) as f64,
            (self.height - params.shape_size as u16) as f64,
        );
        let size = params.shape_size as f64;
        for _ in 0..params.quantity {
            let x = self.coord_rng.next_f64(0.0, bounds.width);
            let y = self.coord_rng.next_f64(0.0, bounds.height);
            let rect = Rect::from_origin_size((x, y), (size, size));
            let color = self.random_color();
            {
                let ctx = &mut self.ctx;
                ctx.set_paint(Brush::Solid(color));
                match params.test.render_op() {
                    RenderOp::Stroke => ctx.stroke_rect(&rect),
                    _ => ctx.fill_rect(&rect),
                }
            }
        }
    }

    fn render_rect_rotated(&mut self, params: &BenchParams) {
        let bounds = Size::new(
            (self.width - params.shape_size as u16) as f64,
            (self.height - params.shape_size as u16) as f64,
        );
        let mut angle = 0.0;
        let center = Point::new(self.width as f64 / 2.0, self.height as f64 / 2.0);
        for _ in 0..params.quantity {
            let size = params.shape_size as f64;
            let x = self.coord_rng.next_f64(0.0, bounds.width);
            let y = self.coord_rng.next_f64(0.0, bounds.height);
            let rect = Rect::from_origin_size((x, y), (size, size));
            let color = self.random_color();
            {
                let ctx = &mut self.ctx;
                ctx.set_paint(Brush::Solid(color));
                let previous = *ctx.transform();
                let rotation = rotate_about(center, angle);
                ctx.set_transform(rotation * previous);
                match params.test.render_op() {
                    RenderOp::Stroke => ctx.stroke_rect(&rect),
                    _ => ctx.fill_rect(&rect),
                }
                ctx.set_transform(previous);
            }
            angle += 0.01;
        }
    }

    fn render_round(&mut self, params: &BenchParams, rotate: bool) {
        let bounds = Size::new(
            (self.width - params.shape_size as u16) as f64,
            (self.height - params.shape_size as u16) as f64,
        );
        let mut angle = 0.0;
        let center = Point::new(self.width as f64 / 2.0, self.height as f64 / 2.0);
        for _ in 0..params.quantity {
            let size = params.shape_size as f64;
            let x = self.coord_rng.next_f64(0.0, bounds.width);
            let y = self.coord_rng.next_f64(0.0, bounds.height);
            let radius = self.extra_rng.next_f64(4.0, 40.0);
            let rect = Rect::from_origin_size((x, y), (size, size));
            let color = self.random_color();
            let path = rect.to_rounded_rect(radius).to_path(0.25);
            {
                let ctx = &mut self.ctx;
                ctx.set_paint(Brush::Solid(color));
                let previous = *ctx.transform();
                if rotate {
                    let rotation = rotate_about(center, angle);
                    ctx.set_transform(rotation * previous);
                }
                match params.test.render_op() {
                    RenderOp::Stroke => ctx.stroke_path(&path),
                    _ => ctx.fill_path(&path),
                }
                if rotate {
                    ctx.set_transform(previous);
                }
            }
            angle += 0.01;
        }
    }

    fn render_polygon(&mut self, params: &BenchParams, complexity: u32) {
        let bounds = Size::new(
            (self.width - params.shape_size as u16) as f64,
            (self.height - params.shape_size as u16) as f64,
        );
        let size = params.shape_size as f64;
        for _ in 0..params.quantity {
            let base_x = self.coord_rng.next_f64(0.0, bounds.width);
            let base_y = self.coord_rng.next_f64(0.0, bounds.height);
            let mut path = BezPath::new();
            for i in 0..complexity {
                let px = self.coord_rng.next_f64(base_x, base_x + size);
                let py = self.coord_rng.next_f64(base_y, base_y + size);
                if i == 0 {
                    path.move_to((px, py));
                } else {
                    path.line_to((px, py));
                }
            }
            path.close_path();
            let color = self.random_color();
            {
                let ctx = &mut self.ctx;
                ctx.set_paint(Brush::Solid(color));
                match params.test.render_op() {
                    RenderOp::Stroke => ctx.stroke_path(&path),
                    RenderOp::FillEvenOdd => {
                        ctx.set_fill_rule(Fill::EvenOdd);
                        ctx.fill_path(&path);
                        ctx.set_fill_rule(Fill::NonZero);
                    }
                    _ => ctx.fill_path(&path),
                }
            }
        }
    }

    fn render_shape(&mut self, params: &BenchParams, kind: ShapeKind) {
        let bounds = Size::new(
            (self.width - params.shape_size as u16) as f64,
            (self.height - params.shape_size as u16) as f64,
        );
        let path = shapes::scaled_path(kind, params.shape_size as f64);
        for _ in 0..params.quantity {
            let base_x = self.coord_rng.next_f64(0.0, bounds.width);
            let base_y = self.coord_rng.next_f64(0.0, bounds.height);
            let color = self.random_color();
            {
                let ctx = &mut self.ctx;
                ctx.set_paint(Brush::Solid(color));
                let previous = *ctx.transform();
                ctx.set_transform(Affine::translate((base_x, base_y)) * previous);
                match params.test.render_op() {
                    RenderOp::Stroke => ctx.stroke_path(&path),
                    RenderOp::FillEvenOdd => {
                        ctx.set_fill_rule(Fill::EvenOdd);
                        ctx.fill_path(&path);
                        ctx.set_fill_rule(Fill::NonZero);
                    }
                    _ => ctx.fill_path(&path),
                }
                ctx.set_transform(previous);
            }
        }
    }
}

fn rotate_about(center: Point, angle: f64) -> Affine {
    Affine::translate((center.x, center.y))
        * Affine::rotate(angle)
        * Affine::translate((-center.x, -center.y))
}

impl Backend for VelloBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&mut self, params: &BenchParams) -> BackendRun {
        self.reset_state();
        self.prepare_context(params);
        let timer = TimerGuard::start();
        match params.test {
            TestKind::FillRectA | TestKind::StrokeRectA => self.render_rect_aligned(params),
            TestKind::FillRectU | TestKind::StrokeRectU => self.render_rect_floating(params),
            TestKind::FillRectRot | TestKind::StrokeRectRot => self.render_rect_rotated(params),
            TestKind::FillRoundU | TestKind::StrokeRoundU => self.render_round(params, false),
            TestKind::FillRoundRot | TestKind::StrokeRoundRot => self.render_round(params, true),
            TestKind::FillTriangle | TestKind::StrokeTriangle => self.render_polygon(params, 3),
            TestKind::FillPolyNZ10 | TestKind::FillPolyEO10 | TestKind::StrokePoly10 => {
                self.render_polygon(params, 10)
            }
            TestKind::FillPolyNZ20 | TestKind::FillPolyEO20 | TestKind::StrokePoly20 => {
                self.render_polygon(params, 20)
            }
            TestKind::FillPolyNZ40 | TestKind::FillPolyEO40 | TestKind::StrokePoly40 => {
                self.render_polygon(params, 40)
            }
            TestKind::FillButterfly | TestKind::StrokeButterfly => {
                self.render_shape(params, ShapeKind::Butterfly)
            }
            TestKind::FillFish | TestKind::StrokeFish => {
                self.render_shape(params, ShapeKind::Fish)
            }
            TestKind::FillDragon | TestKind::StrokeDragon => {
                self.render_shape(params, ShapeKind::Dragon)
            }
            TestKind::FillWorld | TestKind::StrokeWorld => {
                self.render_shape(params, ShapeKind::World)
            }
        }
        self.ctx.flush();
        self.ctx.render_to_pixmap(&mut self.surface);
        BackendRun {
            duration_us: timer.elapsed_us(),
        }
    }

    fn surface(&self) -> &Pixmap {
        &self.surface
    }
}

pub fn create_backends(width: u32, height: u32, thread_counts: &[u16]) -> Vec<Box<dyn Backend>> {
    thread_counts
        .iter()
        .copied()
        .map(|count| Box::new(VelloBackend::new(width, height, count)) as Box<dyn Backend>)
        .collect()
}
