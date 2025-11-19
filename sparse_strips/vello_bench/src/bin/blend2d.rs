#![allow(missing_docs, reason = "CLI wrapper for blend2d bench")]

fn main() -> anyhow::Result<()> {
    vello_bench::blend2d::run_cli()
}
