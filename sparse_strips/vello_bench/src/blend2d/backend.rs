use std::time::Instant;

use rand::{rngs::StdRng, RngCore, SeedableRng};
use vello_common::{kurbo::Size, pixmap::Pixmap};

use crate::blend2d::{
    sprites::Sprites,
    tests::{CompOpInfo, StyleKind, TestKind},
};

#[derive(Clone, Debug)]
pub struct BenchParams {
    pub screen_size: Size,
    pub style: StyleKind,
    pub test: TestKind,
    pub comp_op: &'static CompOpInfo,
    pub shape_size: u32,
    pub quantity: u32,
    pub stroke_width: f64,
}

pub struct BenchAssets<'a> {
    pub sprites: &'a Sprites,
}

pub struct BackendRun {
    pub duration_us: u64,
}

pub trait Backend {
    fn name(&self) -> &str;
    fn supports_style(&self, _style: StyleKind) -> bool {
        true
    }
    fn supports_comp_op(&self, comp: &CompOpInfo) -> bool {
        comp.mode.is_some()
    }
    fn run(&mut self, assets: &BenchAssets<'_>, params: &BenchParams) -> BackendRun;
    fn surface(&self) -> &Pixmap;
}

pub struct BenchRandom {
    rng: StdRng,
    seed: u64,
}

impl BenchRandom {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    pub fn rewind(&mut self) {
        self.rng = StdRng::seed_from_u64(self.seed);
    }

    pub fn next_f64(&mut self, min: f64, max: f64) -> f64 {
        rand::Rng::random_range(&mut self.rng, min..max)
    }

    pub fn next_i32(&mut self, min: i32, max: i32) -> i32 {
        rand::Rng::random_range(&mut self.rng, min..max)
    }

    pub fn next_bool(&mut self) -> bool {
        self.rng.next_u32() & 1 == 1
    }

    pub fn next_color(&mut self) -> u32 {
        self.rng.next_u32()
    }
}

pub struct TimerGuard(Instant);

impl TimerGuard {
    pub fn start() -> Self {
        Self(Instant::now())
    }

    pub fn elapsed_us(&self) -> u64 {
        self.0.elapsed().as_micros() as u64
    }
}
