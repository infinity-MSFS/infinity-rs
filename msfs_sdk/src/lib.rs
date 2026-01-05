pub fn msfs_sdk_path() -> Result<String, &'static str> {
    if let Ok(sdk) = std::env::var("MSFS2024_SDK") {
        return Ok(sdk);
    }
    Err("MSFS2024_SDK environment variable is not set")
}