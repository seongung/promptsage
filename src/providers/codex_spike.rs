use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};

use super::claude_spike::ProviderOutput;

pub fn run(
    bin_path: &Path,
    prompt: &str,
    constitution: &str,
    working_dir: Option<&Path>,
) -> Result<ProviderOutput> {
    let review_block = format!("{constitution}\n\nReview: {prompt}");
    let working_dir_str: PathBuf = working_dir
        .map(|dir| dir.to_path_buf())
        .unwrap_or(std::env::current_dir().context("failed to determine current directory")?);

    let output = Command::new(bin_path)
        .arg("exec")
        .arg("--json")
        .arg("--sandbox")
        .arg("read-only")
        .arg("--ask-for-approval")
        .arg("on-request")
        .arg("--cd")
        .arg(working_dir_str)
        .arg(review_block)
        .output()
        .context("failed to spawn `codex` CLI")?;

    Ok(ProviderOutput {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
