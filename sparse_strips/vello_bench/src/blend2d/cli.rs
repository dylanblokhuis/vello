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
    /// Calls per test (0 = auto)
    #[arg(long, default_value_t = 0)]
    pub quantity: u32,
    /// Number of sizes from the default ladder (1..=6)
    #[arg(long = "size-count", default_value_t = 6, value_parser = clap::value_parser!(u32).range(1..=6))]
    pub size_count: u32,
    /// Repeat count when measuring tests
    #[arg(long, default_value_t = 1)]
    pub repeat: u32,
    /// Composition operator to benchmark ("all" or name)
    #[arg(long = "comp-op")]
    pub comp_op: Option<String>,
    /// Explicit list of sizes to use (comma separated)
    #[arg(long = "sizes")]
    pub size_list: Option<String>,
    /// Style filter (comma separated, supports -name shorthand)
    #[arg(long = "styles")]
    pub style_list: Option<String>,
    /// Test filter (comma separated, supports -name shorthand)
    #[arg(long = "tests")]
    pub test_list: Option<String>,
    /// Thread counts for Vello CPU backends
    #[arg(long = "threads", value_delimiter = ',', default_values_t = vec![0, 2, 4, 8])]
    pub threads: Vec<u16>,
    /// Save final surfaces for the two largest sizes
    #[arg(long = "save-images")]
    pub save_images: bool,
    /// Save overview image composed of all sizes per test
    #[arg(long = "save-overview")]
    pub save_overview: bool,
    /// Enable additional styles (gradients and textures)
    #[arg(long = "deep")]
    pub deep: bool,
    /// Output JSON path
    #[arg(long = "json-out", default_value = "results.json")]
    pub json_path: String,
}
