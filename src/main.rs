use anyhow::Result;
use clap::Parser;
use radial::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    radial::run(cli)
}
