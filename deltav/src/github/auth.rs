//! GitHub authentication and token resolution.
//!
//! Resolves GitHub tokens from multiple sources in priority order:
//! 1. CLI argument (--token)
//! 2. GITHUB_TOKEN environment variable
//! 3. gh CLI (if available and authenticated)

use anyhow::{Context, Result};
use std::process::Command;

/// Resolve a GitHub token from available sources.
///
/// Priority order:
/// 1. Explicit token (from CLI --token argument)
/// 2. GITHUB_TOKEN environment variable
/// 3. gh CLI authentication (if available)
///
/// For GitHub Enterprise, pass the hostname to check gh CLI auth for that specific host.
pub fn resolve_token(explicit_token: Option<&str>, hostname: Option<&str>) -> Result<String> {
    // 1. Explicit token takes precedence
    if let Some(token) = explicit_token {
        if !token.is_empty() {
            return Ok(token.to_string());
        }
    }

    // 2. Environment variable
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // 3. Try gh CLI
    if let Some(token) = try_gh_cli_token(hostname)? {
        return Ok(token);
    }

    anyhow::bail!(
        "No GitHub token found. Please provide one via:\n\
         - --token argument\n\
         - GITHUB_TOKEN environment variable\n\
         - gh auth login{}",
        hostname
            .map(|h| format!(" --hostname {}", h))
            .unwrap_or_default()
    )
}

/// Try to get a token from the gh CLI.
///
/// Returns Ok(None) if gh is not installed or not authenticated.
/// Returns Err only for unexpected failures.
fn try_gh_cli_token(hostname: Option<&str>) -> Result<Option<String>> {
    // First check if gh is available
    let gh_available = Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !gh_available {
        return Ok(None);
    }

    // Check auth status for the hostname
    let mut status_cmd = Command::new("gh");
    status_cmd.arg("auth").arg("status");
    if let Some(host) = hostname {
        status_cmd.arg("--hostname").arg(host);
    }

    let status = status_cmd
        .output()
        .context("Failed to run gh auth status")?;

    if !status.status.success() {
        // Not authenticated for this host
        return Ok(None);
    }

    // Get the token
    let mut token_cmd = Command::new("gh");
    token_cmd.arg("auth").arg("token");
    if let Some(host) = hostname {
        token_cmd.arg("--hostname").arg(host);
    }

    let output = token_cmd.output().context("Failed to run gh auth token")?;

    if output.status.success() {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !token.is_empty() {
            return Ok(Some(token));
        }
    }

    Ok(None)
}

/// Extract hostname from a GitHub URL.
///
/// Examples:
/// - "https://github.com" -> None (public GitHub, no hostname needed for gh)
/// - "https://github.mycompany.com" -> Some("github.mycompany.com")
pub fn extract_hostname(enterprise_url: &str) -> Option<String> {
    let url = enterprise_url.trim_end_matches('/');

    // Public GitHub doesn't need a hostname for gh CLI
    if url == "https://github.com" || url == "http://github.com" {
        return None;
    }

    // Extract hostname from URL
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hostname_enterprise() {
        assert_eq!(
            extract_hostname("https://github.mycompany.com"),
            Some("github.mycompany.com".to_string())
        );
        assert_eq!(
            extract_hostname("https://github.mycompany.com/"),
            Some("github.mycompany.com".to_string())
        );
    }

    #[test]
    fn test_extract_hostname_public() {
        assert_eq!(extract_hostname("https://github.com"), None);
        assert_eq!(extract_hostname("https://github.com/"), None);
    }

    #[test]
    fn test_resolve_token_explicit() {
        let token = resolve_token(Some("my-token"), None).unwrap();
        assert_eq!(token, "my-token");
    }

    #[test]
    fn test_resolve_token_empty_explicit_falls_through() {
        // Empty explicit token should fall through to env var
        std::env::set_var("GITHUB_TOKEN", "env-token");
        let token = resolve_token(Some(""), None).unwrap();
        assert_eq!(token, "env-token");
        std::env::remove_var("GITHUB_TOKEN");
    }
}
