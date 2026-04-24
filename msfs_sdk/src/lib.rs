use std::path::PathBuf;

/// Version of the pinned SDK headers archive.
/// Update this constant when a new pinned release is published.
pub const SDK_HEADERS_VERSION: &str = "2024.1.0";

/// URL of the pinned SDK headers archive.
/// Override at runtime with the `INFINITY_MSFS_SDK_URL` environment variable.
pub const SDK_HEADERS_URL: &str =
    "https://cdn.infinity-simulations.com/msfs-sdk-headers-v2024.1.0.tar.gz";

/// Returns the path to the MSFS SDK root, checking in this order:
///
/// 1. `MSFS2024_SDK` env var (full SDK install on Windows, backward compat)
/// 2. `INFINITY_MSFS_SDK_CACHE` env var (explicit cache directory override)
/// 3. Default platform cache location:
///    - Linux/macOS: `$XDG_CACHE_HOME/infinity-msfs/sdk/<version>` or
///      `~/.cache/infinity-msfs/sdk/<version>`
///    - Windows:    `%LOCALAPPDATA%\infinity-msfs\sdk\<version>`
///
/// Run `infinity-msfs setup` to download the pinned headers automatically.
pub fn msfs_sdk_path() -> Result<String, String> {
    // Legacy: full MSFS SDK installation on Windows.
    if let Ok(sdk) = std::env::var("MSFS2024_SDK") {
        return Ok(sdk);
    }

    let version = SDK_HEADERS_VERSION;

    // Explicit cache directory override.
    if let Ok(base) = std::env::var("INFINITY_MSFS_SDK_CACHE") {
        let path = PathBuf::from(base).join(version);
        if path.exists() {
            return Ok(path.to_string_lossy().into_owned());
        }
    }

    // Default platform cache location.
    if let Some(base) = default_sdk_cache_dir() {
        let path = base.join(version);
        if path.exists() {
            return Ok(path.to_string_lossy().into_owned());
        }
    }

    Err(format!(
        "MSFS SDK headers not found (version {version}).\n\
         Run `infinity-msfs setup` to download the pinned headers automatically,\n\
         or set MSFS2024_SDK to the path of your MSFS SDK installation."
    ))
}

/// Returns the base directory used to store cached pinned SDK headers.
///
/// The version sub-directory is **not** appended; callers should join
/// [`SDK_HEADERS_VERSION`] themselves.
pub fn default_sdk_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|d| PathBuf::from(d).join("infinity-msfs").join("sdk"))
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
            .map(|base| base.join("infinity-msfs").join("sdk"))
    }
}
