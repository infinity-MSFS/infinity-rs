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
    #[command(alias = "list-projects")]
    Projects(ProjectsArgs),
}

#[derive(Debug, Args, Clone)]
pub struct BuildArgs {
    #[arg(long)]
    pub release: bool,

    /// Stream subprocess output directly instead of the compact progress UI.
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Override the single legacy `[build].package`. Ignored when
    /// `[[packages]]` is set and `--only` is preferred there.
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,

    /// In multi-package mode, restrict the build to the named packages.
    /// May be passed multiple times. Matched against the `package` field
    /// of each `[[packages]]` entry.
    #[arg(long = "only")]
    pub only: Vec<String>,

    #[arg(long = "no-wasm-opt")]
    pub no_wasm_opt: bool,
}

#[derive(Debug, Args, Clone)]
pub struct ProjectsArgs {
    /// Override the single legacy `[build].package`. Ignored when
    /// `[[packages]]` is set and `--only` is preferred there.
    #[arg(short = 'p', long = "package")]
    pub package: Option<String>,

    /// In multi-package mode, restrict the list to the named packages.
    /// May be passed multiple times. Matched against the `package` field
    /// of each `[[packages]]` entry.
    #[arg(long = "only")]
    pub only: Vec<String>,
}
