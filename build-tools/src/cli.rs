use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "infinity-msfs")]
#[command(version)]
#[command(about = "MSFS WASM build tooling for Infinity Rust projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Build(BuildArgs),
}

#[derive(Debug, Args, Clone)]
pub struct BuildArgs {
    #[arg(long)]
    pub release: bool,

    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,

    #[arg(long = "no-wasm-opt")]
    pub no_wasm_opt: bool,
}
