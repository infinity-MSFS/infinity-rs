use std::{fs, path::PathBuf};

const CURRENT_FILE: &str = "current.txt";

/// Returns the path to the resolved MSFS 2024 SDK root.
///
/// Resolution order:
/// 1. `MSFS2024_SDK` env var (full SDK install — backward compat).
/// 2. `INFINITY_MSFS_SDK_CACHE/<current>` where `<current>` is read from
///    `current.txt` inside the cache directory.
/// 3. Default platform cache:
///    - Windows: `%LOCALAPPDATA%\infinity-msfs\sdk\msfs2024\<current>`
///    - Linux/macOS: `$XDG_CACHE_HOME/infinity-msfs/sdk/msfs2024/<current>`
///      (falls back to `$HOME/.cache/...`).
///
/// Run `infinity-msfs sdk install` to download the SDK directly from
/// `sdk.flightsimulator.com`.
pub fn sdk_path() -> Result<String, String> {
    if let Ok(sdk) = std::env::var("MSFS2024_SDK") {
        return Ok(sdk);
    }

    let base = cache_base().ok_or_else(|| {
        "could not determine SDK cache directory.\n\
         Set INFINITY_MSFS_SDK_CACHE or MSFS2024_SDK to a valid path."
            .to_string()
    })?;

    let version = current_version(&base).ok_or_else(|| {
        "no MSFS 2024 SDK installed.\n\
         Run `infinity-msfs sdk install` to fetch it from Microsoft,\n\
         or set MSFS2024_SDK to an existing SDK installation."
            .to_string()
    })?;

    let path = base.join(&version);
    if !path.exists() {
        return Err(format!(
            "current SDK version {version} not found at {}; \
             run `infinity-msfs sdk install --force`",
            path.display()
        ));
    }
    Ok(path.to_string_lossy().into_owned())
}

/// Returns the platform-default SDK cache base (the directory that holds
/// `current.txt` and the per-version subdirectories).
pub fn default_sdk_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA").ok().map(|d| {
            PathBuf::from(d)
                .join("infinity-msfs")
                .join("sdk")
                .join("msfs2024")
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_CACHE_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".cache"))
            })
            .map(|base| base.join("infinity-msfs").join("sdk").join("msfs2024"))
    }
}

/// Returns the cache base, honoring the `INFINITY_MSFS_SDK_CACHE` override.
pub fn cache_base() -> Option<PathBuf> {
    if let Ok(base) = std::env::var("INFINITY_MSFS_SDK_CACHE") {
        return Some(PathBuf::from(base));
    }
    default_sdk_cache_dir()
}

/// Reads the version recorded in `<base>/current.txt`, if any.
pub fn current_version(base: &std::path::Path) -> Option<String> {
    fs::read_to_string(base.join(CURRENT_FILE))
        .ok()
        .map(|s| s.trim().to_string())
}

/// Writes `version` into `<base>/current.txt`, creating `base` if needed.
pub fn write_current_version(base: &std::path::Path, version: &str) -> std::io::Result<()> {
    fs::create_dir_all(base)?;
    fs::write(base.join(CURRENT_FILE), version)
}
