use vello_cpu::{
    kurbo::{Affine, BezPath, Point, Rect, Size, Shape},
    peniko::{
        color::{AlphaColor, Srgb},
        BlendMode, Brush, ColorStop, Extend, Fill, Gradient, ImageBrush, ImageQuality, ImageSampler,
    },
    RenderContext, RenderSettings,
};
use vello_common::paint::{Image as PaintImage, ImageSource};
use vello_common::pixmap::Pixmap;

use crate::blend2d::{
    backend::{Backend, BackendRun, BenchAssets, BenchParams, BenchRandom, TimerGuard},
    shapes,
    tests::{RenderOp, ShapeKind, StyleKind, TestKind},
};

const COORD_SEED: u64 = 0x19AE0DDAE3FA7391;
const COLOR_SEED: u64 = 0x94BD7A499AD10011;
const EXTRA_SEED: u64 = 0x1ABD9CC9CAF0F123;

pub struct VelloBackend {
    name: String,
    settings: RenderSettings,
    surface: Pixmap,
    width: u16,
    height: u16,
    coord_rng: BenchRandom,
    color_rng: BenchRandom,
    extra_rng: BenchRandom,
    sprite_cursor: usize,
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
        Self {
            name,
            settings,
            surface: Pixmap::new(width_u16, height_u16),
            width: width_u16,
            height: height_u16,
            coord_rng: BenchRandom::new(COORD_SEED),
            color_rng: BenchRandom::new(COLOR_SEED),
            extra_rng: BenchRandom::new(EXTRA_SEED),
            sprite_cursor: 0,
        }
    }

    fn reset_state(&mut self) {
        self.coord_rng.rewind();
        self.color_rng.rewind();
        self.extra_rng.rewind();
        self.sprite_cursor = 0;
    }

    fn next_sprite_index(&mut self) -> usize {
        let idx = self.sprite_cursor;
        self.sprite_cursor = (self.sprite_cursor + 1) % 4;
        idx
    }

    fn setup_context(&self, comp_op: Option<BlendMode>, stroke_width: f64) -> RenderContext {
        let mut ctx = RenderContext::new_with(self.width, self.height, self.settings);
        if let Some(mode) = comp_op {
            ctx.set_blend_mode(mode);
        }
        let mut stroke = ctx.stroke().clone();
        stroke.width = stroke_width;
        ctx.set_stroke(stroke);
        ctx
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

    fn image_brush(
        &mut self,
        rect: Rect,
        style: StyleKind,
        size: u32,
        assets: &BenchAssets<'_>,
    ) -> (Brush<PaintImage, Gradient>, Option<Affine>) {
        let sprite = assets
            .sprites
            .sprite(self.next_sprite_index(), size.max(1));
        let quality = if matches!(style, StyleKind::PatternNearest) {
            ImageQuality::Low
        } else {
            ImageQuality::Medium
        };
        let mut brush = ImageBrush {
            image: ImageSource::Pixmap(sprite),
            sampler: ImageSampler::default(),
        };
        brush.sampler.x_extend = Extend::Repeat;
        brush.sampler.y_extend = Extend::Repeat;
        brush.sampler.quality = quality;
        (Brush::Image(brush), Some(Affine::translate((-rect.x0, -rect.y0))))
    }

    fn gradient_brush(&mut self, gradient: Gradient) -> Brush<PaintImage, Gradient> {
        Brush::Gradient(gradient)
    }

    fn make_brush(
        &mut self,
        rect: Rect,
        style: StyleKind,
        shape_size: u32,
        assets: &BenchAssets<'_>,
    ) -> (Brush<PaintImage, Gradient>, Option<Affine>) {
        match style {
            StyleKind::Solid => (Brush::Solid(self.random_color()), None),
            StyleKind::LinearPad | StyleKind::LinearRepeat | StyleKind::LinearReflect => {
                let start = Point::new(rect.x0 + rect.width() * 0.2, rect.y0 + rect.height() * 0.2);
                let end = Point::new(rect.x0 + rect.width() * 0.8, rect.y0 + rect.height() * 0.8);
                let mut gradient = Gradient::new_linear(start, end);
                gradient.stops.extend([
                    ColorStop {
                        offset: 0.0,
                        color: self.random_color().into(),
                    },
                    ColorStop {
                        offset: 0.5,
                        color: self.random_color().into(),
                    },
                    ColorStop {
                        offset: 1.0,
                        color: self.random_color().into(),
                    },
                ]);
                gradient.extend = match style {
                    StyleKind::LinearRepeat => Extend::Repeat,
                    StyleKind::LinearReflect => Extend::Reflect,
                    _ => Extend::Pad,
                };
                (self.gradient_brush(gradient), None)
            }
            StyleKind::RadialPad | StyleKind::RadialRepeat | StyleKind::RadialReflect => {
                let center = Point::new(rect.x0 + rect.width() * 0.5, rect.y0 + rect.height() * 0.5);
                let radius = ((rect.width() + rect.height()) * 0.25) as f32;
                let mut gradient = Gradient::new_radial(center, radius);
                gradient.stops.extend([
                    ColorStop {
                        offset: 0.0,
                        color: self.random_color().into(),
                    },
                    ColorStop {
                        offset: 0.5,
                        color: self.random_color().into(),
                    },
                    ColorStop {
                        offset: 1.0,
                        color: self.random_color().into(),
                    },
                ]);
                gradient.extend = match style {
                    StyleKind::RadialRepeat => Extend::Repeat,
                    StyleKind::RadialReflect => Extend::Reflect,
                    _ => Extend::Pad,
                };
                (self.gradient_brush(gradient), None)
            }
            StyleKind::Conic => {
                let center = Point::new(rect.x0 + rect.width() * 0.5, rect.y0 + rect.height() * 0.5);
                let mut gradient = Gradient::new_sweep(center, 0.0, std::f32::consts::TAU);
                gradient.stops.extend([
                    ColorStop {
                        offset: 0.0,
                        color: self.random_color().into(),
                    },
                    ColorStop {
                        offset: 0.33,
                        color: self.random_color().into(),
                    },
                    ColorStop {
                        offset: 0.66,
                        color: self.random_color().into(),
                    },
                    ColorStop {
                        offset: 1.0,
                        color: self.random_color().into(),
                    },
                ]);
                (self.gradient_brush(gradient), None)
            }
            StyleKind::PatternNearest | StyleKind::PatternBilinear => {
                self.image_brush(rect, style, shape_size, assets)
            }
        }
    }

    fn apply_brush(
        &mut self,
        ctx: &mut RenderContext,
        rect: Rect,
        style: StyleKind,
        shape_size: u32,
        assets: &BenchAssets<'_>,
    ) -> bool {
        let (brush, transform) = self.make_brush(rect, style, shape_size, assets);
        ctx.set_paint(brush);
        if let Some(t) = transform {
            ctx.set_paint_transform(t);
            true
        } else {
            false
        }
    }

    fn finish_brush(transform_used: bool, ctx: &mut RenderContext) {
        if transform_used {
            ctx.reset_paint_transform();
        }
    }

    fn render_rect_aligned(
        &mut self,
        ctx: &mut RenderContext,
        params: &BenchParams,
        assets: &BenchAssets<'_>,
    ) {
        let bounds_x = (self.width as i32 - params.shape_size as i32).max(1);
        let bounds_y = (self.height as i32 - params.shape_size as i32).max(1);
        for _ in 0..params.quantity {
            let x = self.coord_rng.next_i32(0, bounds_x);
            let y = self.coord_rng.next_i32(0, bounds_y);
            let rect = Rect::from_origin_size((x as f64, y as f64), (params.shape_size as f64, params.shape_size as f64));
            let transform_used = self.apply_brush(ctx, rect, params.style, params.shape_size, assets);
            match params.test.render_op() {
                RenderOp::Stroke => ctx.stroke_rect(&rect),
                _ => ctx.fill_rect(&rect),
            }
            Self::finish_brush(transform_used, ctx);
        }
    }

    fn render_rect_floating(
        &mut self,
        ctx: &mut RenderContext,
        params: &BenchParams,
        assets: &BenchAssets<'_>,
    ) {
        let bounds = Size::new(
            (self.width - params.shape_size as u16) as f64,
            (self.height - params.shape_size as u16) as f64,
        );
        let size = params.shape_size as f64;
        for _ in 0..params.quantity {
            let x = self.coord_rng.next_f64(0.0, bounds.width);
            let y = self.coord_rng.next_f64(0.0, bounds.height);
            let rect = Rect::from_origin_size((x, y), (size, size));
            let transform_used = self.apply_brush(ctx, rect, params.style, params.shape_size, assets);
            match params.test.render_op() {
                RenderOp::Stroke => ctx.stroke_rect(&rect),
                _ => ctx.fill_rect(&rect),
            }
            Self::finish_brush(transform_used, ctx);
        }
    }

    fn render_rect_rotated(
        &mut self,
        ctx: &mut RenderContext,
        params: &BenchParams,
        assets: &BenchAssets<'_>,
    ) {
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
            let transform_used = self.apply_brush(ctx, rect, params.style, params.shape_size, assets);
            let previous = *ctx.transform();
            let rotation = rotate_about(center, angle);
            ctx.set_transform(rotation * previous);
            match params.test.render_op() {
                RenderOp::Stroke => ctx.stroke_rect(&rect),
                _ => ctx.fill_rect(&rect),
            }
            ctx.set_transform(previous);
            Self::finish_brush(transform_used, ctx);
            angle += 0.01;
        }
    }

    fn render_round(
        &mut self,
        ctx: &mut RenderContext,
        params: &BenchParams,
        assets: &BenchAssets<'_>,
        rotate: bool,
    ) {
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
            let transform_used = self.apply_brush(ctx, rect, params.style, params.shape_size, assets);
            let path = rect.to_rounded_rect(radius).to_path(0.25);
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
            Self::finish_brush(transform_used, ctx);
            angle += 0.01;
        }
    }

    fn render_polygon(
        &mut self,
        ctx: &mut RenderContext,
        params: &BenchParams,
        assets: &BenchAssets<'_>,
        complexity: u32,
    ) {
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
            let rect = Rect::from_origin_size((base_x, base_y), (size, size));
            let transform_used = self.apply_brush(ctx, rect, params.style, params.shape_size, assets);
            match params.test.render_op() {
                RenderOp::Stroke => ctx.stroke_path(&path),
                RenderOp::FillEvenOdd => {
                    ctx.set_fill_rule(Fill::EvenOdd);
                    ctx.fill_path(&path);
                    ctx.set_fill_rule(Fill::NonZero);
                }
                _ => ctx.fill_path(&path),
            }
            Self::finish_brush(transform_used, ctx);
        }
    }

    fn render_shape(
        &mut self,
        ctx: &mut RenderContext,
        params: &BenchParams,
        assets: &BenchAssets<'_>,
        kind: ShapeKind,
    ) {
        let bounds = Size::new(
            (self.width - params.shape_size as u16) as f64,
            (self.height - params.shape_size as u16) as f64,
        );
        let path = shapes::scaled_path(kind, params.shape_size as f64);
        for _ in 0..params.quantity {
            let base_x = self.coord_rng.next_f64(0.0, bounds.width);
            let base_y = self.coord_rng.next_f64(0.0, bounds.height);
            let rect = Rect::from_origin_size((base_x, base_y), (params.shape_size as f64, params.shape_size as f64));
            let transform_used = self.apply_brush(ctx, rect, params.style, params.shape_size, assets);
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
            Self::finish_brush(transform_used, ctx);
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

    fn run(&mut self, assets: &BenchAssets<'_>, params: &BenchParams) -> BackendRun {
        self.reset_state();
        let mut ctx = self.setup_context(params.comp_op.mode, params.stroke_width);
        let timer = TimerGuard::start();
        match params.test {
            TestKind::FillRectA | TestKind::StrokeRectA => {
                self.render_rect_aligned(&mut ctx, params, assets)
            }
            TestKind::FillRectU | TestKind::StrokeRectU => {
                self.render_rect_floating(&mut ctx, params, assets)
            }
            TestKind::FillRectRot | TestKind::StrokeRectRot => {
                self.render_rect_rotated(&mut ctx, params, assets)
            }
            TestKind::FillRoundU | TestKind::StrokeRoundU => {
                self.render_round(&mut ctx, params, assets, false)
            }
            TestKind::FillRoundRot | TestKind::StrokeRoundRot => {
                self.render_round(&mut ctx, params, assets, true)
            }
            TestKind::FillTriangle | TestKind::StrokeTriangle => {
                self.render_polygon(&mut ctx, params, assets, 3)
            }
            TestKind::FillPolyNZ10 | TestKind::FillPolyEO10 | TestKind::StrokePoly10 => {
                self.render_polygon(&mut ctx, params, assets, 10)
            }
            TestKind::FillPolyNZ20 | TestKind::FillPolyEO20 | TestKind::StrokePoly20 => {
                self.render_polygon(&mut ctx, params, assets, 20)
            }
            TestKind::FillPolyNZ40 | TestKind::FillPolyEO40 | TestKind::StrokePoly40 => {
                self.render_polygon(&mut ctx, params, assets, 40)
            }
            TestKind::FillButterfly | TestKind::StrokeButterfly => {
                self.render_shape(&mut ctx, params, assets, ShapeKind::Butterfly)
            }
            TestKind::FillFish | TestKind::StrokeFish => {
                self.render_shape(&mut ctx, params, assets, ShapeKind::Fish)
            }
            TestKind::FillDragon | TestKind::StrokeDragon => {
                self.render_shape(&mut ctx, params, assets, ShapeKind::Dragon)
            }
            TestKind::FillWorld | TestKind::StrokeWorld => {
                self.render_shape(&mut ctx, params, assets, ShapeKind::World)
            }
        }
        ctx.flush();
        ctx.render_to_pixmap(&mut self.surface);
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
