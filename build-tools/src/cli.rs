use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

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
    /// Download the pinned MSFS SDK headers to the local cache.
    ///
    /// After running this command, `cargo build --target wasm32-wasip1` no
    /// longer requires the full MSFS SDK to be installed.  The headers are
    /// stored in the platform cache directory and are reused across projects.
    Setup(SetupArgs),
    /// Pack the MSFS SDK headers from a full SDK installation into a
    /// redistributable `.tar.gz` archive ready for upload.
    ///
    /// The archive contains exactly the WASM subtree required by the crate
    /// build script.  Upload it to your CDN/R2 bucket and point
    /// `SDK_HEADERS_URL` (in `msfs_sdk/src/lib.rs`) at the public URL.
    PackSdk(PackSdkArgs),
    /// Run pre-flight checks for the build environment: rust toolchain,
    /// `wasm32-wasip1` target, `wasm-opt` on PATH, cached SDK headers,
    /// and SimConnect lib presence (when relevant).
    Doctor,
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
pub struct PackSdkArgs {
    /// Path to the root of the MSFS SDK installation.
    /// Defaults to the `MSFS2024_SDK` environment variable.
    #[arg(long, env = "MSFS2024_SDK")]
    pub sdk_path: String,

    /// Where to write the output archive.
    /// Defaults to `msfs-sdk-headers-v<VERSION>.tar.gz` in the current directory.
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct SetupArgs {
    /// Re-download even if the pinned version is already cached.
    #[arg(long)]
    pub force: bool,

    /// Override the download URL (defaults to the built-in pinned URL).
    #[arg(long, env = "INFINITY_MSFS_SDK_URL")]
    pub url: Option<String>,
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
