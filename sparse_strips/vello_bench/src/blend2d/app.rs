use std::{collections::HashSet, fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use vello_common::pixmap::Pixmap;
use vello_cpu::peniko::color::PremulRgba8;

use crate::blend2d::{
    backend::{Backend, BenchParams},
    backend_vello_cpu,
    cli::Blend2dArgs,
    json::{JsonRecord, JsonWriter},
    sprites,
    tests::{self, BENCH_SHAPE_SIZES, COMP_OPS, CompOpInfo, TestKind},
};

const SOLID_STYLE: &str = "Solid";
const DEFAULT_COMP_OP: &CompOpInfo = &COMP_OPS[0];
const TABLE_BORDER: &str = "+--------------------+-------------+---------------+----------+----------+----------+----------+----------+----------+";

pub fn run(args: Blend2dArgs) -> Result<()> {
    let config = BenchmarkConfig::from_args(args)?;
    BenchRunner::new(config).run()
}

struct BenchmarkConfig {
    width: u32,
    height: u32,
    quantity: u32,
    min_runs: u32,
    sizes: Vec<u32>,
    tests: Vec<TestKind>,
    threads: Vec<u16>,
    save_images: bool,
    save_overview: bool,
    json_path: PathBuf,
}

impl BenchmarkConfig {
    fn from_args(args: Blend2dArgs) -> Result<Self> {
        let mut threads = args.threads;
        threads.sort_unstable();
        threads.dedup();
        if threads.is_empty() {
            threads.push(0);
        }

        let sizes = if let Some(list) = args.size_list.as_deref() {
            parse_sizes(list)?
        } else {
            let count = usize::try_from(args.size_count).unwrap_or(BENCH_SHAPE_SIZES.len());
            BENCH_SHAPE_SIZES[..count.min(BENCH_SHAPE_SIZES.len())].to_vec()
        };

        let test_items: Vec<_> = tests::TestKind::ALL
            .iter()
            .map(|test| (test.name(), *test))
            .collect();

        let tests = parse_toggle_list(
            args.test_list.as_deref(),
            &test_items,
            &tests::TestKind::ALL,
        )?;

        Ok(Self {
            width: args.width,
            height: args.height,
            quantity: args.quantity,
            min_runs: args.min_runs.max(1),
            sizes,
            tests,
            threads,
            save_images: args.save_images,
            save_overview: args.save_overview,
            json_path: PathBuf::from(args.json_path),
        })
    }
}

struct BenchRunner {
    config: BenchmarkConfig,
    json: JsonWriter,
}

impl BenchRunner {
    fn new(config: BenchmarkConfig) -> Self {
        let json = JsonWriter::new(
            config.width,
            config.height,
            config.quantity,
            config.min_runs,
            config.sizes.clone(),
        );
        Self { config, json }
    }

    fn run(mut self) -> Result<()> {
        if self.config.save_images || self.config.save_overview {
            fs::create_dir_all("images").ok();
        }
        let mut backends = backend_vello_cpu::create_backends(
            self.config.width,
            self.config.height,
            &self.config.threads,
        );
        for backend in backends.iter_mut() {
            self.run_backend(backend.as_mut())?;
        }
        self.json.write(&self.config.json_path)
    }

    fn run_backend(&mut self, backend: &mut dyn Backend) -> Result<()> {
        let mut records = Vec::new();
        let mut params = BenchParams {
            screen_size: vello_common::kurbo::Size::new(
                self.config.width as f64,
                self.config.height as f64,
            ),
            test: TestKind::FillRectA,
            comp_op: DEFAULT_COMP_OP,
            shape_size: self.config.sizes[0],
            quantity: self.config.quantity,
            stroke_width: 2.0,
        };

        let mut totals = vec![0.0; self.config.sizes.len()];

        println!("{}", TABLE_BORDER);
        println!(
            "|{:<20}| {:<11} | {:<13} | {:<9}| {:<9}| {:<9}| {:<9}| {:<9}| {:<9}|",
            truncate(backend.name(), 20),
            truncate(DEFAULT_COMP_OP.name, 11),
            truncate(SOLID_STYLE, 13),
            "8x8",
            "16x16",
            "32x32",
            "64x64",
            "128x128",
            "256x256",
        );
        println!("{}", TABLE_BORDER);

        for &test in &self.config.tests {
            params.test = test;
            let mut cpms_values = Vec::new();
            let mut overview = self.maybe_create_overview();

            for (index, &size) in self.config.sizes.iter().enumerate() {
                params.shape_size = size;
                let (duration, used_quantity) = run_single_test(
                    backend,
                    &mut params,
                    self.config.quantity,
                    self.config.min_runs,
                );
                let cpms = if duration == 0 {
                    0.0
                } else {
                    used_quantity as f64 * 1000.0 / duration as f64
                };
                totals[index] += cpms;
                cpms_values.push(format_cpms(cpms));
                if let Some(ref mut pixmap) = overview {
                    copy_into_overview(pixmap, index, backend.surface(), self.config.width);
                }
                if self.config.save_images && index + 2 >= self.config.sizes.len() {
                    let suffix = (b'A' + index as u8) as char;
                    let file = format!(
                        "images/{}-{}-{}-{}-{}.png",
                        test.name(),
                        DEFAULT_COMP_OP.name,
                        SOLID_STYLE,
                        suffix,
                        backend.name()
                    );
                    save_surface(backend.surface(), &sanitize(&file))?;
                }
            }

            if let Some(pixmap) = overview {
                let file = format!(
                    "images/{}-{}-{}-{}.png",
                    test.name(),
                    DEFAULT_COMP_OP.name,
                    SOLID_STYLE,
                    backend.name()
                );
                save_surface(&pixmap, &sanitize(&file))?;
            }

            print_row(test.name(), DEFAULT_COMP_OP.name, SOLID_STYLE, &cpms_values);
            records.push(JsonRecord {
                test_name: test.name().to_string(),
                comp_op: DEFAULT_COMP_OP.name.to_string(),
                style: SOLID_STYLE.to_string(),
                rcpms: cpms_values,
            });
        }

        let total_strings: Vec<String> = totals.iter().map(|value| format_cpms(*value)).collect();
        print_row("Total", DEFAULT_COMP_OP.name, SOLID_STYLE, &total_strings);
        println!("{}", TABLE_BORDER);

        self.json
            .push_run(backend.name().to_string(), None, records);
        Ok(())
    }

    fn maybe_create_overview(&self) -> Option<Pixmap> {
        if !self.config.save_overview {
            return None;
        }
        let width = 1 + ((self.config.width + 1) * self.config.sizes.len() as u32);
        let height = self.config.height + 2;
        let mut pixmap = Pixmap::new(width as u16, height as u16);
        sprites::clear_pixmap(
            &mut pixmap,
            PremulRgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );
        Some(pixmap)
    }
}

fn run_single_test(
    backend: &mut dyn Backend,
    params: &mut BenchParams,
    configured_quantity: u32,
    min_runs: u32,
) -> (u64, u32) {
    const INITIAL_QUANTITY: u32 = 25;
    const MIN_DURATION_US: u64 = 1000;

    let mut quantity = if configured_quantity == 0 {
        INITIAL_QUANTITY
    } else {
        configured_quantity
    };
    let mut best = u64::MAX;
    let mut attempts = 0;

    let required_runs = min_runs.max(1);

    if configured_quantity == 0 {
        loop {
            params.quantity = quantity;
            let run = backend.run(params);
            best = run.duration_us;
            if run.duration_us >= MIN_DURATION_US || quantity > 1_000_000 {
                attempts = 1;
                break;
            }
            if run.duration_us < 100 {
                quantity *= 10;
            } else if run.duration_us < 500 {
                quantity *= 3;
            } else {
                quantity *= 2;
            }
        }
    }

    while attempts < required_runs {
        params.quantity = quantity;
        let run = backend.run(params);
        if run.duration_us < best {
            best = run.duration_us;
        }
        attempts += 1;
    }

    (best, quantity)
}

fn parse_sizes(list: &str) -> Result<Vec<u32>> {
    let mut sizes = Vec::new();
    for part in list.split(',') {
        if part.trim().is_empty() {
            continue;
        }
        let value: u32 = part
            .trim()
            .parse()
            .with_context(|| format!("Invalid size '{part}'"))?;
        sizes.push(value);
    }
    if sizes.is_empty() {
        return Err(anyhow!("No sizes provided"));
    }
    Ok(sizes)
}

fn parse_toggle_list<T: Copy + Eq + std::hash::Hash>(
    list: Option<&str>,
    items: &[(&str, T)],
    default_items: &[T],
) -> Result<Vec<T>> {
    if let Some(list) = list {
        let mut include_mode = None;
        let mut result = HashSet::new();
        for raw in list.split(',') {
            let token = raw.trim();
            if token.is_empty() {
                continue;
            }
            let (mode, name) = if let Some(rest) = token.strip_prefix('-') {
                (-1, rest)
            } else {
                (1, token)
            };
            if include_mode.is_none() {
                include_mode = Some(mode);
            } else if include_mode != Some(mode) {
                return Err(anyhow!("Cannot mix additive and subtractive entries"));
            }
            let value = items
                .iter()
                .find(|(label, _)| label.eq_ignore_ascii_case(name))
                .ok_or_else(|| anyhow!("Unknown entry '{name}'"))?
                .1;
            result.insert(value);
        }
        match include_mode.unwrap_or(1) {
            1 => Ok(result.into_iter().collect()),
            -1 => {
                let mut base: HashSet<T> = default_items.iter().copied().collect();
                for item in result {
                    base.remove(&item);
                }
                Ok(base.into_iter().collect())
            }
            _ => unreachable!(),
        }
    } else {
        Ok(default_items.to_vec())
    }
}

fn format_cpms(value: f64) -> String {
    if value <= 0.1 {
        format!("{value:.4}")
    } else if value <= 1.0 {
        format!("{value:.3}")
    } else if value < 10.0 {
        format!("{value:.2}")
    } else if value < 100.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    }
}

fn print_row(test: &str, comp: &str, style: &str, values: &[String]) {
    let mut cols = values.to_vec();
    while cols.len() < 6 {
        cols.push(String::from("-"));
    }
    println!(
        "|{:<20}| {:<11} | {:<13} | {:<9}| {:<9}| {:<9}| {:<9}| {:<9}| {:<9}|",
        truncate(test, 20),
        truncate(comp, 11),
        truncate(style, 13),
        cols[0],
        cols[1],
        cols[2],
        cols[3],
        cols[4],
        cols[5]
    );
}

fn truncate(input: &str, max: usize) -> String {
    if input.len() <= max {
        input.to_string()
    } else {
        input[..max].to_string()
    }
}

fn copy_into_overview(target: &mut Pixmap, index: usize, surface: &Pixmap, width: u32) {
    let x = 1 + index as i32 * (width as i32 + 1);
    sprites::blit(surface, target, x, 1);
}

fn save_surface(surface: &Pixmap, path: &str) -> Result<()> {
    let png = surface.clone().into_png().context("encode png")?;
    fs::write(path, png).with_context(|| format!("write {path}"))
}

fn sanitize(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '/' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
