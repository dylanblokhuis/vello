use std::sync::LazyLock;

use vello_common::kurbo::{Affine, BezPath, PathEl, Point};

use crate::blend2d::{
    generated::shapes as data,
    tests::ShapeKind,
};

fn build_path(commands: &str, vertices: &[f64]) -> BezPath {
    let mut path = BezPath::new();
    let mut index = 0;
    for cmd in commands.chars() {
        match cmd {
            'M' => {
                let p = Point::new(vertices[index], vertices[index + 1]);
                path.push(PathEl::MoveTo(p));
                index += 2;
            }
            'L' => {
                let p = Point::new(vertices[index], vertices[index + 1]);
                path.push(PathEl::LineTo(p));
                index += 2;
            }
            'Q' => {
                let p0 = Point::new(vertices[index], vertices[index + 1]);
                let p1 = Point::new(vertices[index + 2], vertices[index + 3]);
                path.push(PathEl::QuadTo(p0, p1));
                index += 4;
            }
            'C' => {
                let p0 = Point::new(vertices[index], vertices[index + 1]);
                let p1 = Point::new(vertices[index + 2], vertices[index + 3]);
                let p2 = Point::new(vertices[index + 4], vertices[index + 5]);
                path.push(PathEl::CurveTo(p0, p1, p2));
                index += 6;
            }
            'Z' => path.push(PathEl::ClosePath),
            _ => {}
        }
    }
    path
}

static BUTTERFLY: LazyLock<BezPath> = LazyLock::new(|| {
    build_path(data::shapes::BUTTERFLY_COMMANDS, data::shapes::BUTTERFLY_VERTICES)
});
static FISH: LazyLock<BezPath> = LazyLock::new(|| {
    build_path(data::shapes::FISH_COMMANDS, data::shapes::FISH_VERTICES)
});
static DRAGON: LazyLock<BezPath> = LazyLock::new(|| {
    build_path(data::shapes::DRAGON_COMMANDS, data::shapes::DRAGON_VERTICES)
});
static WORLD: LazyLock<BezPath> = LazyLock::new(|| {
    build_path(data::shapes::WORLD_COMMANDS, data::shapes::WORLD_VERTICES)
});

fn base_path(kind: ShapeKind) -> &'static BezPath {
    match kind {
        ShapeKind::Butterfly => &BUTTERFLY,
        ShapeKind::Fish => &FISH,
        ShapeKind::Dragon => &DRAGON,
        ShapeKind::World => &WORLD,
    }
}

pub fn scaled_path(kind: ShapeKind, size: f64) -> BezPath {
    let mut path = base_path(kind).clone();
    path.apply_affine(Affine::scale(size));
    path
}
