use clap::Parser;

/// Blend2D compatible benchmarking harness for Vello CPU.
#[derive(Debug, Parser)]
#[command(author, version, about = None, long_about = None)]
pub struct Blend2dArgs {
    /// Canvas width
    #[arg(long, default_value_t = 512)]
    pub width: u32,
    /// Canvas height
    #[arg(long, default_value_t = 600)]
    pub height: u32,
    /// Minimum runs per benchmark (best result is used)
    #[arg(long = "min-runs", default_value_t = 50)]
    pub min_runs: u32,
    /// Test filter (comma separated, supports -name shorthand)
    #[arg(long = "tests")]
    pub test_list: Option<String>,
    /// Thread counts for Vello CPU backends
    #[arg(long = "threads", value_delimiter = ',', default_values_t = vec![0, 2, 4, 8])]
    pub threads: Vec<u16>,
    /// Generate overview images with a reduced quantity for a quicker preview run
    #[arg(long = "preview")]
    pub preview: bool,
    /// Compare results against an existing JSON baseline
    #[arg(long = "baseline")]
    pub baseline: Option<String>,
    /// Output JSON path
    #[arg(long = "json-out", default_value = "results.json")]
    pub json_path: String,
}
