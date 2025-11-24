use std::{collections::{HashMap, HashSet}, fs, path::{Path, PathBuf}};

use anyhow::{Context, Result, anyhow};
use owo_colors::OwoColorize;
use serde::Deserialize;
use vello_common::pixmap::Pixmap;
use vello_cpu::peniko::color::PremulRgba8;

use crate::{
    backend::{Backend, BenchParams},
    backend_vello_cpu,
    cli::Blend2dArgs,
    json::{JsonRecord, JsonWriter},
    tests::{self, BENCH_SHAPE_SIZES, COMP_OPS, CompOpInfo, TestKind},
};

const SOLID_STYLE: &str = "Solid";
const DEFAULT_COMP_OP: &CompOpInfo = &COMP_OPS[0];
const TABLE_BORDER: &str = "+--------------------+-------------+---------------+------------------+------------------+------------------+------------------+------------------+------------------+";

pub fn run(args: Blend2dArgs) -> Result<()> {
    let config = BenchmarkConfig::from_args(args)?;
    BenchRunner::new(config)?.run()
}

struct BenchmarkConfig {
    width: u32,
    height: u32,
    quantity: u32,
    min_runs: u32,
    sizes: Vec<u32>,
    tests: Vec<TestKind>,
    threads: Vec<u16>,
    preview: bool,
    baseline: Option<PathBuf>,
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
        let sizes = BENCH_SHAPE_SIZES.to_vec();

        let test_items: Vec<_> = tests::TestKind::ALL
            .iter()
            .map(|test| (test.name(), *test))
            .collect();

        let tests = parse_toggle_list(
            args.test_list.as_deref(),
            &test_items,
            &tests::TestKind::ALL,
        )?;

        let quantity = if args.preview { 10 } else { 0 };

        Ok(Self {
            width: args.width,
            height: args.height,
            quantity,
            min_runs: args.min_runs.max(1),
            sizes,
            tests,
            threads,
            preview: args.preview,
            baseline: args.baseline.map(PathBuf::from),
            json_path: PathBuf::from(args.json_path),
        })
    }
}

struct BenchRunner {
    config: BenchmarkConfig,
    json: JsonWriter,
    baseline: Option<Baseline>,
}

impl BenchRunner {
    fn new(config: BenchmarkConfig) -> Result<Self> {
        let json = JsonWriter::new(
            config.width,
            config.height,
            config.quantity,
            config.min_runs,
            config.sizes.clone(),
        );
        let baseline = config
            .baseline
            .as_ref()
            .map(|path| Baseline::load(path.as_path()))
            .transpose()?;
        Ok(Self { config, json, baseline })
    }

    fn run(mut self) -> Result<()> {
        if self.config.preview {
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
        let baseline_ref = self.baseline.as_ref();
        let mut baseline_totals = baseline_ref
            .map(|_| vec![BaselineSum::default(); self.config.sizes.len()]);

        println!("{}", TABLE_BORDER);
        println!(
            "|{:<20}| {:<11} | {:<13} | {:<18}| {:<18}| {:<18}| {:<18}| {:<18}| {:<18}|",
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
            let mut cpms_strings = Vec::new();
            let mut display_cells = Vec::new();
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
                let formatted = format_cpms(cpms);
                let baseline_entry = baseline_ref
                    .map(|baseline| baseline.lookup(backend.name(), test.name(), size));
                if let (Some(entry), Some(totals)) = (&baseline_entry, baseline_totals.as_mut()) {
                    totals[index].push(entry.clone());
                }
                if let Some(ref mut pixmap) = overview {
                    copy_into_overview(pixmap, index, backend.surface(), self.config.width);
                }
                cpms_strings.push(formatted.clone());
                display_cells.push(CellData::new(cpms, formatted, baseline_entry));
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

            print_row(test.name(), DEFAULT_COMP_OP.name, SOLID_STYLE, &display_cells);
            records.push(JsonRecord {
                test_name: test.name().to_string(),
                comp_op: DEFAULT_COMP_OP.name.to_string(),
                style: SOLID_STYLE.to_string(),
                rcpms: cpms_strings,
            });
        }

        let total_baseline_entries = baseline_totals
            .map(|entries| entries.into_iter().map(|entry| Some(entry.finish())).collect())
            .unwrap_or_else(|| vec![None; self.config.sizes.len()]);

        let total_cells: Vec<CellData> = totals
            .iter()
            .zip(total_baseline_entries.into_iter())
            .map(|(&value, baseline)| {
                CellData::new(value, format_cpms(value), baseline)
            })
            .collect();

        print_row("Total", DEFAULT_COMP_OP.name, SOLID_STYLE, &total_cells);
        println!("{}", TABLE_BORDER);

        self.json
            .push_run(backend.name().to_string(), None, records);
        Ok(())
    }

    fn maybe_create_overview(&self) -> Option<Pixmap> {
        if !self.config.preview {
            return None;
        }
        let width = 1 + ((self.config.width + 1) * self.config.sizes.len() as u32);
        let height = self.config.height + 2;
        let mut pixmap = Pixmap::new(width as u16, height as u16);
        clear_pixmap(
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

fn print_row(test: &str, comp: &str, style: &str, cells: &[CellData]) {
    let mut columns: Vec<String> = cells.iter().map(format_cell).collect();
    while columns.len() < BENCH_SHAPE_SIZES.len() {
        columns.push(String::from("-"));
    }

    println!(
        "|{:<20}| {:<11} | {:<13} | {:<18}| {:<18}| {:<18}| {:<18}| {:<18}| {:<18}|",
        truncate(test, 20),
        truncate(comp, 11),
        truncate(style, 13),
        columns[0].as_str(),
        columns[1].as_str(),
        columns[2].as_str(),
        columns[3].as_str(),
        columns[4].as_str(),
        columns[5].as_str()
    );
}

fn format_cell(cell: &CellData) -> String {
    let base = cell.formatted.clone();
    match &cell.baseline {
        None => base,
        Some(Ok(baseline)) => {
            if baseline.abs() <= f64::EPSILON {
                return format!("{base} {}", "(baseline 0)".red());
            }
            let diff = ((cell.raw - baseline) / baseline) * 100.0;
            let diff_text = format!("{diff:+.1}%");
            let colored = if diff >= 3.0 {
                diff_text.green().to_string()
            } else if diff <= -3.0 {
                diff_text.red().to_string()
            } else {
                diff_text.bright_black().to_string()
            };
            format!("{base} {colored}")
        }
        Some(Err(err)) => format!("{base} {}", format!("({err})").red()),
    }
}

#[derive(Clone)]
struct CellData {
    raw: f64,
    formatted: String,
    baseline: Option<Result<f64, String>>,
}

impl CellData {
    fn new(raw: f64, formatted: String, baseline: Option<Result<f64, String>>) -> Self {
        Self {
            raw,
            formatted,
            baseline,
        }
    }
}

#[derive(Clone, Default)]
struct BaselineSum {
    total: f64,
    has_value: bool,
    error: Option<String>,
}

impl BaselineSum {
    fn push(&mut self, entry: Result<f64, String>) {
        match entry {
            Ok(value) => {
                if self.error.is_none() {
                    self.total += value;
                    self.has_value = true;
                }
            }
            Err(err) => {
                if self.error.is_none() {
                    self.error = Some(err);
                }
            }
        }
    }

    fn finish(self) -> Result<f64, String> {
        if let Some(err) = self.error {
            Err(err)
        } else if self.has_value {
            Ok(self.total)
        } else {
            Err("missing baseline value".to_string())
        }
    }
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
    blit_surface(surface, target, x, 1);
}

fn blit_surface(src: &Pixmap, dst: &mut Pixmap, origin_x: i32, origin_y: i32) {
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    for y in 0..sh {
        let dy = origin_y + y;
        if dy < 0 || dy >= dh {
            continue;
        }
        let src_row = &src.data()[(y as usize) * (sw as usize)..][..sw as usize];
        let dst_row = &mut dst.data_mut()[(dy as usize) * (dw as usize)..][..dw as usize];
        for x in 0..sw {
            let dx = origin_x + x;
            if dx < 0 || dx >= dw {
                continue;
            }
            dst_row[dx as usize] = src_row[x as usize];
        }
    }
}

fn clear_pixmap(pixmap: &mut Pixmap, color: PremulRgba8) {
    for pixel in pixmap.data_mut() {
        *pixel = color;
    }
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

struct Baseline {
    entries: HashMap<(String, String, u32), f64>,
}

impl Baseline {
    fn load(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path)
            .with_context(|| format!("read baseline {}", path.display()))?;
        let root: BaselineRoot = serde_json::from_str(&data)
            .with_context(|| format!("parse baseline {}", path.display()))?;
        let sizes: Vec<u32> = root
            .options
            .sizes
            .iter()
            .map(|label| parse_size_label(label))
            .collect::<Result<_, _>>()?;

        let mut entries = HashMap::new();
        for run in root.runs {
            for record in run.records {
                for (idx, value_str) in record.rcpms.iter().enumerate() {
                    let Some(&size) = sizes.get(idx) else { continue };
                    if let Ok(value) = value_str.parse::<f64>() {
                        entries.insert((run.name.clone(), record.test_name.clone(), size), value);
                    }
                }
            }
        }

        Ok(Self { entries })
    }

    fn lookup(&self, backend: &str, test: &str, size: u32) -> Result<f64, String> {
        self.entries
            .get(&(backend.to_string(), test.to_string(), size))
            .copied()
            .ok_or_else(|| "missing baseline value".to_string())
    }
}

#[derive(Deserialize)]
struct BaselineRoot {
    options: BaselineOptions,
    runs: Vec<BaselineRun>,
}

#[derive(Deserialize)]
struct BaselineOptions {
    sizes: Vec<String>,
}

#[derive(Deserialize)]
struct BaselineRun {
    name: String,
    #[serde(default)]
    records: Vec<BaselineRecord>,
}

#[derive(Deserialize)]
struct BaselineRecord {
    #[serde(rename = "test")]
    test_name: String,
    rcpms: Vec<String>,
}

fn parse_size_label(label: &str) -> Result<u32> {
    let mut parts = label.split('x');
    let value = parts
        .next()
        .ok_or_else(|| anyhow!("invalid baseline size '{label}'"))?
        .trim()
        .parse()
        .with_context(|| format!("invalid baseline size '{label}'"))?;
    Ok(value)
}
