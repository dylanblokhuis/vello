#![allow(unreachable_pub)]
mod app;
mod backend;
mod backend_vello_cpu;
pub mod cli;
mod generated;
mod json;
mod shapes;
mod sprites;
mod tests;

pub use cli::Blend2dArgs;

use anyhow::Result;
use clap::Parser;

pub fn run_cli() -> Result<()> {
    let args = Blend2dArgs::parse();
    run(args)
}

pub fn run(args: Blend2dArgs) -> Result<()> {
    app::run(args)
}
