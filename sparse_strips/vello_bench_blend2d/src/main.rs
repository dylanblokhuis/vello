#![allow(unreachable_pub)]
mod app;
mod backend;
mod backend_vello_cpu;
pub mod cli;
mod generated_shapes;
mod json;
mod shapes;
mod tests;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Blend2dArgs::parse();
    app::run(args)
}
