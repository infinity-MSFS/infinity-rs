mod build;
mod cargo_meta;
mod cli;
mod config;
mod process;
mod scripts;
mod ui;
mod util;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(err) = real_main() {
        ui::print_error(&err);
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(args) => build::run_build(args)?,
        Commands::Projects(args) => build::run_projects(args)?,
    }

    Ok(())
}
