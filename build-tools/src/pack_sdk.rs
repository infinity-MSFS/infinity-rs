use crate::cli::PackSdkArgs;
use anyhow::{Context, Result, bail};
use console::style;
use flate2::{Compression, write::GzEncoder};
use indicatif::{ProgressBar, ProgressStyle};
use msfs_sdk::SDK_HEADERS_VERSION;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tar::Builder;

/// Files and directories (relative to the SDK root) packed into the
/// redistributable archive.  Each entry is (sdk-relative source path,
/// archive path).
///
/// Directories are packed recursively; single files are packed as-is.
///
/// The archive contains everything required for both WASM gauge builds and
/// Windows native builds that use SimConnect.  The SimConnect headers are
/// needed on every platform (bindgen runs unconditionally when the
/// `simconnect` feature is enabled), and the SimConnect lib/dll are needed
/// at link time for native Windows builds.
const SDK_ENTRIES: &[(&str, &str)] = &[
    ("WASM/wasi-sysroot", "WASM/wasi-sysroot"),
    ("WASM/include", "WASM/include"),
    (
        "WASM/src/MSFS/Render/nanovg.cpp",
        "WASM/src/MSFS/Render/nanovg.cpp",
    ),
    // Required to mark the resulting .wasm as an MSFS 2024 module.
    // Without it, runtime features like SimConnect fall back to 2020
    // semantics.
    ("WASM/WasmVersions", "WASM/WasmVersions"),
    ("SimConnect SDK/include", "SimConnect SDK/include"),
    ("SimConnect SDK/lib", "SimConnect SDK/lib"),
];

pub fn run_pack_sdk(args: PackSdkArgs) -> Result<()> {
    let sdk = PathBuf::from(&args.sdk_path);
    if !sdk.exists() {
        bail!("SDK path does not exist: {}", sdk.display());
    }

    // Validate that the expected subtrees are actually present.
    for (rel, _) in SDK_ENTRIES {
        let candidate = sdk.join(rel);
        if !candidate.exists() {
            bail!(
                "expected SDK path not found: {}\n\
                 Make sure '{}' points to the MSFS SDK root (the directory that\n\
                 contains the 'WASM' and 'SimConnect SDK' subdirectories).",
                candidate.display(),
                args.sdk_path,
            );
        }
    }

    let archive_name = format!("msfs-sdk-headers-v{SDK_HEADERS_VERSION}.tar.gz");
    let output = args.output.unwrap_or_else(|| {
        // Use the workspace/project `target/` directory if we can find it,
        // otherwise fall back to the current directory.
        crate::util::find_project_root()
            .map(|root| root.join("target").join(&archive_name))
            .unwrap_or_else(|_| PathBuf::from(&archive_name))
    });

    // Count files up front so we can show a meaningful progress bar.
    print!("{} Scanning SDK tree…", style("→").cyan().bold(),);
    let files = collect_files(&sdk)?;
    println!(
        "\r{} Found {} files to pack",
        style("→").cyan().bold(),
        style(files.len()).bold(),
    );

    println!("  SDK:    {}", style(sdk.display()).dim(),);
    println!("  Output: {}", style(output.display()).dim(),);

    let bar = ProgressBar::new(files.len() as u64);
    bar.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/blue}] \
             {pos}/{len} {msg}",
        )
        .expect("valid pack progress template")
        .progress_chars("=> "),
    );
    bar.enable_steady_tick(Duration::from_millis(120));

    let out_file = fs::File::create(&output)
        .with_context(|| format!("failed to create output file {}", output.display()))?;
    let gz = GzEncoder::new(out_file, Compression::best());
    let mut tar = Builder::new(gz);

    for (src, archive_path) in &files {
        bar.set_message(archive_path.to_string());
        tar.append_path_with_name(src, archive_path)
            .with_context(|| format!("failed to pack {}", src.display()))?;
        bar.inc(1);
    }

    let gz = tar.into_inner().context("failed to finalize tar archive")?;
    gz.finish().context("failed to finalize gzip stream")?;

    bar.finish_and_clear();

    let size = fs::metadata(&output)
        .map(|m| format_bytes(m.len()))
        .unwrap_or_else(|_| "?".to_string());

    println!(
        "{} Packed {} files → {} ({})",
        style("✓").green().bold(),
        style(files.len()).bold(),
        style(output.display()).bold(),
        style(&size).dim(),
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walks all `SDK_ENTRIES` under `sdk_root` and returns a flat list of
/// `(absolute_source_path, archive_relative_path)` pairs.
fn collect_files(sdk_root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut out = Vec::new();

    for (rel_src, rel_archive) in SDK_ENTRIES {
        let src = sdk_root.join(rel_src);
        if src.is_file() {
            out.push((src, rel_archive.to_string()));
        } else if src.is_dir() {
            collect_dir(&src, rel_archive, &mut out)?;
        }
    }

    Ok(out)
}

fn collect_dir(dir: &Path, archive_prefix: &str, out: &mut Vec<(PathBuf, String)>) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .collect::<Result<_, _>>()
        .with_context(|| format!("failed to iterate directory {}", dir.display()))?;

    // Sort for deterministic archive ordering.
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let archive_path = format!("{archive_prefix}/{}", name.to_string_lossy());

        if path.is_dir() {
            collect_dir(&path, &archive_path, out)?;
        } else if path.is_file() {
            out.push((path, archive_path));
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
