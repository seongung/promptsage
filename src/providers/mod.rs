pub mod claude;
pub mod claude_spike;
pub mod codex;
pub mod codex_spike;

use std::path::Path;
use std::process::ExitStatus;

pub struct ProviderRequest<'a> {
    pub binary: &'a Path,
    pub constitution: &'a str,
    pub prompt: &'a str,
    pub working_dir: Option<&'a Path>,
}

pub struct ProviderResponse {
    pub stdout: String,
    pub stderr: String,
    pub status: ExitStatus,
}

impl ProviderResponse {
    pub fn from_output(output: std::process::Output) -> Self {
        ProviderResponse {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            status: output.status,
        }
    }
}
