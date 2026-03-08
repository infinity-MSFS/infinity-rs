use anyhow::{Context, Result, bail};
use std::{path::Path, process::Command};

pub fn run_script_list(root: &Path, label: &str, scripts: &[String]) -> Result<()> {
    for (i, script) in scripts.iter().enumerate() {
        run_script(root, &format!("{label}[{i}]"), script)?;
    }

    Ok(())
}

fn run_script(root: &Path, label: &str, script: &str) -> Result<()> {
    println!("[infinity-msfs] running script {label}: {script}");

    let mut cmd = shell_command(script);
    cmd.current_dir(root);

    let status = cmd
        .status()
        .with_context(|| format!("failed to start script {label}"))?;

    if !status.success() {
        bail!("script {label} failed with status {status}");
    }

    Ok(())
}

#[cfg(windows)]
fn shell_command(script: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(script);
    cmd
}

#[cfg(not(windows))]
fn shell_command(script: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script);
    cmd
}
