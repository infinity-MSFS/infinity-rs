use crate::cli::SetupArgs;
use anyhow::{Context, Result, bail};
use console::style;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use msfs_sdk::{SDK_HEADERS_URL, SDK_HEADERS_VERSION, default_sdk_cache_dir};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};
use tar::Archive;

pub fn run_setup(args: SetupArgs) -> Result<()> {
    let url = args.url.as_deref().unwrap_or(SDK_HEADERS_URL).to_string();
    download_and_install(&url, args.force)
}

/// Ensures the pinned SDK headers are present in the cache, downloading them
/// if they are not.  Called automatically by `run_build` so users don't need
/// to run `infinity-msfs setup` manually.
///
/// Returns `Ok(())` immediately (without printing anything) if the headers are
/// already cached.
pub fn ensure_sdk_headers() -> Result<()> {
    let version = SDK_HEADERS_VERSION;

    // If MSFS2024_SDK is set the user has a full SDK; nothing to do.
    if std::env::var("MSFS2024_SDK").is_ok() {
        return Ok(());
    }

    let already_cached = default_sdk_cache_dir()
        .map(|base| base.join(version).exists())
        .unwrap_or(false);

    if already_cached {
        return Ok(());
    }

    let url =
        std::env::var("INFINITY_MSFS_SDK_URL").unwrap_or_else(|_| SDK_HEADERS_URL.to_string());

    download_and_install(&url, false)
}

fn download_and_install(url: &str, force: bool) -> Result<()> {
    let version = SDK_HEADERS_VERSION;

    let cache_base = default_sdk_cache_dir().context(
        "could not determine the SDK cache directory.\n\
         Set INFINITY_MSFS_SDK_CACHE to an explicit path and retry.",
    )?;
    let dest = cache_base.join(version);

    if dest.exists() && !force {
        println!(
            "{} SDK headers {} already cached at {}",
            style("✓").green().bold(),
            style(version).bold(),
            style(dest.display()).dim(),
        );
        println!("  Run with {} to re-download.", style("--force").yellow());
        return Ok(());
    }

    println!(
        "{} Downloading MSFS SDK headers {}",
        style("→").cyan().bold(),
        style(version).bold(),
    );
    println!("  URL: {}", style(url).dim());
    println!("  Dest: {}", style(dest.display()).dim());

    fs::create_dir_all(&cache_base)
        .with_context(|| format!("failed to create cache directory {}", cache_base.display()))?;

    let archive_bytes = download_with_progress(url)?;

    // If re-downloading, remove the old tree first.
    if dest.exists() {
        fs::remove_dir_all(&dest)
            .with_context(|| format!("failed to remove existing cache at {}", dest.display()))?;
    }

    println!("{} Extracting archive…", style("→").cyan().bold());

    extract_tar_gz(&archive_bytes, &dest)?;

    println!(
        "{} SDK headers {} installed to {}",
        style("✓").green().bold(),
        style(version).bold(),
        style(dest.display()).dim(),
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

/// Downloads the archive at `url` into memory, showing a progress bar.
fn download_with_progress(url: &str) -> Result<Vec<u8>> {
    let response = ureq::get(url)
        .call()
        .with_context(|| format!("HTTP request failed for {url}"))?;

    let content_length: Option<u64> = response
        .header("Content-Length")
        .and_then(|v| v.parse().ok());

    let bar = if let Some(total) = content_length {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "  {spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/blue}] \
                 {bytes}/{total_bytes} ({bytes_per_sec})",
            )
            .expect("valid download progress template")
            .progress_chars("=> "),
        );
        pb.enable_steady_tick(Duration::from_millis(120));
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template(
                "  {spinner:.cyan} [{elapsed_precise}] {bytes} downloaded",
            )
            .expect("valid download progress template"),
        );
        pb.enable_steady_tick(Duration::from_millis(120));
        pb
    };

    let mut reader = ProgressReader {
        inner: response.into_reader(),
        bar: bar.clone(),
    };

    let mut buf = match content_length {
        Some(len) => Vec::with_capacity(len as usize),
        None => Vec::new(),
    };
    reader
        .read_to_end(&mut buf)
        .context("error reading download stream")?;

    bar.finish_and_clear();
    Ok(buf)
}

struct ProgressReader<R: Read> {
    inner: R,
    bar: ProgressBar,
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.bar.inc(n as u64);
        Ok(n)
    }
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// Extracts a `.tar.gz` byte slice under `dest`.
///
/// Archive entries are extracted at their stored paths (e.g. `WASM/include/…`
/// or `SimConnect SDK/include/…`) relative to `dest`.  No leading component
/// is stripped — the `pack-sdk` command always writes canonical SDK-relative
/// paths.
fn extract_tar_gz(data: &[u8], dest: &Path) -> Result<()> {
    let gz = GzDecoder::new(data);
    let mut archive = Archive::new(gz);

    for entry in archive
        .entries()
        .context("failed to read archive entries")?
    {
        let mut entry = entry.context("corrupt archive entry")?;
        let rel = entry
            .path()
            .context("archive entry has no path")?
            .into_owned();

        // Guard against path-traversal attacks in the archive.
        let out_path = safe_join(dest, &rel)?;

        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&out_path)
                .with_context(|| format!("failed to create directory {}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            let mut out_file = fs::File::create(&out_path)
                .with_context(|| format!("failed to create file {}", out_path.display()))?;
            io::copy(&mut entry, &mut out_file)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
        }
    }

    Ok(())
}

/// Joins `base` and `rel` while ensuring the result stays inside `base`
/// (protection against path-traversal in archive entries).
fn safe_join(base: &Path, rel: &Path) -> Result<PathBuf> {
    use std::path::Component;
    for component in rel.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "archive entry '{}' would escape the destination directory",
                    rel.display()
                );
            }
        }
    }
    Ok(base.join(rel))
}
