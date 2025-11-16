use std::{path::Path, process::Command};

use anyhow::{Context, Result};

#[derive(Debug)]
pub struct ProviderOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(bin_path: &Path, prompt: &str, constitution: &str) -> Result<ProviderOutput> {
    let review_prompt = format!("Review: {prompt}");
    let output = Command::new(bin_path)
        .arg("-p")
        .arg("--output-format")
        .arg("json")
        .arg("--max-turns")
        .arg("1")
        .arg("--append-system-prompt")
        .arg(constitution)
        .arg(review_prompt)
        .output()
        .context("failed to spawn `claude` CLI")?;

    Ok(ProviderOutput {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
