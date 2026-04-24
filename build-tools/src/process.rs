use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn run_command(cmd: &mut Command, label: &str, verbose: bool) -> Result<()> {
    if verbose {
        let status = cmd
            .status()
            .with_context(|| format!("failed to start {label}"))?;

        if !status.success() {
            bail!("{label} failed with status {status}");
        }

        return Ok(());
    }

    let output = cmd
        .output()
        .with_context(|| format!("failed to start {label}"))?;

    let stdout = decode_output(&output.stdout);
    let stderr = decode_output(&output.stderr);

    if !output.status.success() {
        let mut message = format!("{label} failed with status {}", output.status);
        let rendered = render_command_output(&stdout, &stderr);
        if !rendered.is_empty() {
            message.push_str("\n\n");
            message.push_str(&rendered);
        }

        bail!(message);
    }

    Ok(())
}

fn decode_output(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn render_command_output(stdout: &str, stderr: &str) -> String {
    let mut sections = Vec::new();

    if !stderr.trim().is_empty() {
        sections.push(format!("stderr:\n{}", stderr.trim_end()));
    }

    if !stdout.trim().is_empty() {
        sections.push(format!("stdout:\n{}", stdout.trim_end()));
    }

    sections.join("\n\n")
}
