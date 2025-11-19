use std::fmt;

use vello_common::peniko::{BlendMode, Compose, Mix};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Butterfly,
    Fish,
    Dragon,
    World,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderOp {
    FillNonZero,
    FillEvenOdd,
    Stroke,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TestKind {
    FillRectA,
    FillRectU,
    FillRectRot,
    FillRoundU,
    FillRoundRot,
    FillTriangle,
    FillPolyNZ10,
    FillPolyEO10,
    FillPolyNZ20,
    FillPolyEO20,
    FillPolyNZ40,
    FillPolyEO40,
    FillButterfly,
    FillFish,
    FillDragon,
    FillWorld,
    StrokeRectA,
    StrokeRectU,
    StrokeRectRot,
    StrokeRoundU,
    StrokeRoundRot,
    StrokeTriangle,
    StrokePoly10,
    StrokePoly20,
    StrokePoly40,
    StrokeButterfly,
    StrokeFish,
    StrokeDragon,
    StrokeWorld,
}

impl TestKind {
    pub const ALL: &[TestKind] = &[
        TestKind::FillRectA,
        TestKind::FillRectU,
        TestKind::FillRectRot,
        TestKind::FillRoundU,
        TestKind::FillRoundRot,
        TestKind::FillTriangle,
        TestKind::FillPolyNZ10,
        TestKind::FillPolyEO10,
        TestKind::FillPolyNZ20,
        TestKind::FillPolyEO20,
        TestKind::FillPolyNZ40,
        TestKind::FillPolyEO40,
        TestKind::FillButterfly,
        TestKind::FillFish,
        TestKind::FillDragon,
        TestKind::FillWorld,
        TestKind::StrokeRectA,
        TestKind::StrokeRectU,
        TestKind::StrokeRectRot,
        TestKind::StrokeRoundU,
        TestKind::StrokeRoundRot,
        TestKind::StrokeTriangle,
        TestKind::StrokePoly10,
        TestKind::StrokePoly20,
        TestKind::StrokePoly40,
        TestKind::StrokeButterfly,
        TestKind::StrokeFish,
        TestKind::StrokeDragon,
        TestKind::StrokeWorld,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TestKind::FillRectA => "FillRectA",
            TestKind::FillRectU => "FillRectU",
            TestKind::FillRectRot => "FillRectRot",
            TestKind::FillRoundU => "FillRoundU",
            TestKind::FillRoundRot => "FillRoundRot",
            TestKind::FillTriangle => "FillTriangle",
            TestKind::FillPolyNZ10 => "FillPolyNZi10",
            TestKind::FillPolyEO10 => "FillPolyEOi10",
            TestKind::FillPolyNZ20 => "FillPolyNZi20",
            TestKind::FillPolyEO20 => "FillPolyEOi20",
            TestKind::FillPolyNZ40 => "FillPolyNZi40",
            TestKind::FillPolyEO40 => "FillPolyEOi40",
            TestKind::FillButterfly => "FillButterfly",
            TestKind::FillFish => "FillFish",
            TestKind::FillDragon => "FillDragon",
            TestKind::FillWorld => "FillWorld",
            TestKind::StrokeRectA => "StrokeRectA",
            TestKind::StrokeRectU => "StrokeRectU",
            TestKind::StrokeRectRot => "StrokeRectRot",
            TestKind::StrokeRoundU => "StrokeRoundU",
            TestKind::StrokeRoundRot => "StrokeRoundRot",
            TestKind::StrokeTriangle => "StrokeTriangle",
            TestKind::StrokePoly10 => "StrokePoly10",
            TestKind::StrokePoly20 => "StrokePoly20",
            TestKind::StrokePoly40 => "StrokePoly40",
            TestKind::StrokeButterfly => "StrokeButterfly",
            TestKind::StrokeFish => "StrokeFish",
            TestKind::StrokeDragon => "StrokeDragon",
            TestKind::StrokeWorld => "StrokeWorld",
        }
    }

    pub fn render_op(self) -> RenderOp {
        match self {
            TestKind::FillRectA
            | TestKind::FillRectU
            | TestKind::FillRectRot
            | TestKind::FillRoundU
            | TestKind::FillRoundRot
            | TestKind::FillTriangle
            | TestKind::FillPolyNZ10
            | TestKind::FillPolyNZ20
            | TestKind::FillPolyNZ40
            | TestKind::FillButterfly
            | TestKind::FillFish
            | TestKind::FillDragon
            | TestKind::FillWorld => RenderOp::FillNonZero,
            TestKind::FillPolyEO10 | TestKind::FillPolyEO20 | TestKind::FillPolyEO40 => {
                RenderOp::FillEvenOdd
            }
            _ => RenderOp::Stroke,
        }
    }

    pub fn polygon_complexity(self) -> Option<u32> {
        match self {
            TestKind::FillTriangle | TestKind::StrokeTriangle => Some(3),
            TestKind::FillPolyNZ10 | TestKind::FillPolyEO10 | TestKind::StrokePoly10 => Some(10),
            TestKind::FillPolyNZ20 | TestKind::FillPolyEO20 | TestKind::StrokePoly20 => Some(20),
            TestKind::FillPolyNZ40 | TestKind::FillPolyEO40 | TestKind::StrokePoly40 => Some(40),
            _ => None,
        }
    }

    pub fn shape(self) -> Option<ShapeKind> {
        match self {
            TestKind::FillButterfly | TestKind::StrokeButterfly => Some(ShapeKind::Butterfly),
            TestKind::FillFish | TestKind::StrokeFish => Some(ShapeKind::Fish),
            TestKind::FillDragon | TestKind::StrokeDragon => Some(ShapeKind::Dragon),
            TestKind::FillWorld | TestKind::StrokeWorld => Some(ShapeKind::World),
            _ => None,
        }
    }
}

impl fmt::Display for TestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CompOpInfo {
    pub name: &'static str,
    pub mode: Option<BlendMode>,
}

const fn compose(compose: Compose) -> Option<BlendMode> {
    Some(BlendMode::new(Mix::Normal, compose))
}

const fn mix(mix: Mix) -> Option<BlendMode> {
    Some(BlendMode::new(mix, Compose::SrcOver))
}

pub const COMP_OPS: [CompOpInfo; 29] = [
    CompOpInfo {
        name: "SrcOver",
        mode: compose(Compose::SrcOver),
    },
    CompOpInfo {
        name: "SrcCopy",
        mode: compose(Compose::Copy),
    },
    CompOpInfo {
        name: "SrcIn",
        mode: compose(Compose::SrcIn),
    },
    CompOpInfo {
        name: "SrcOut",
        mode: compose(Compose::SrcOut),
    },
    CompOpInfo {
        name: "SrcAtop",
        mode: compose(Compose::SrcAtop),
    },
    CompOpInfo {
        name: "DstOver",
        mode: compose(Compose::DestOver),
    },
    CompOpInfo {
        name: "DstCopy",
        mode: compose(Compose::Dest),
    },
    CompOpInfo {
        name: "DstIn",
        mode: compose(Compose::DestIn),
    },
    CompOpInfo {
        name: "DstOut",
        mode: compose(Compose::DestOut),
    },
    CompOpInfo {
        name: "DstAtop",
        mode: compose(Compose::DestAtop),
    },
    CompOpInfo {
        name: "Xor",
        mode: compose(Compose::Xor),
    },
    CompOpInfo {
        name: "Clear",
        mode: compose(Compose::Clear),
    },
    CompOpInfo {
        name: "Plus",
        mode: compose(Compose::Plus),
    },
    CompOpInfo {
        name: "Minus",
        mode: None,
    },
    CompOpInfo {
        name: "Modulate",
        mode: mix(Mix::Multiply),
    },
    CompOpInfo {
        name: "Multiply",
        mode: mix(Mix::Multiply),
    },
    CompOpInfo {
        name: "Screen",
        mode: mix(Mix::Screen),
    },
    CompOpInfo {
        name: "Overlay",
        mode: mix(Mix::Overlay),
    },
    CompOpInfo {
        name: "Darken",
        mode: mix(Mix::Darken),
    },
    CompOpInfo {
        name: "Lighten",
        mode: mix(Mix::Lighten),
    },
    CompOpInfo {
        name: "ColorDodge",
        mode: mix(Mix::ColorDodge),
    },
    CompOpInfo {
        name: "ColorBurn",
        mode: mix(Mix::ColorBurn),
    },
    CompOpInfo {
        name: "LinearBurn",
        mode: None,
    },
    CompOpInfo {
        name: "LinearLight",
        mode: None,
    },
    CompOpInfo {
        name: "PinLight",
        mode: None,
    },
    CompOpInfo {
        name: "HardLight",
        mode: mix(Mix::HardLight),
    },
    CompOpInfo {
        name: "SoftLight",
        mode: mix(Mix::SoftLight),
    },
    CompOpInfo {
        name: "Difference",
        mode: mix(Mix::Difference),
    },
    CompOpInfo {
        name: "Exclusion",
        mode: mix(Mix::Exclusion),
    },
];

pub const BENCH_SHAPE_SIZES: [u32; 6] = [8, 16, 32, 64, 128, 256];

pub fn find_test(name: &str) -> Option<TestKind> {
    TestKind::ALL
        .iter()
        .copied()
        .find(|test| test.name().eq_ignore_ascii_case(name))
}

pub fn find_comp_op(name: &str) -> Option<usize> {
    COMP_OPS
        .iter()
        .position(|info| info.name.eq_ignore_ascii_case(name))
}
