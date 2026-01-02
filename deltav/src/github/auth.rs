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
/// Tries multiple sources in priority order until a valid token is found.
///
/// # Arguments
///
/// * `explicit_token` - Token provided via CLI `--token` argument (highest priority).
/// * `hostname` - GitHub Enterprise hostname for gh CLI auth lookup (e.g., "github.mycompany.com").
///   Pass `None` for public GitHub.
///
/// # Returns
///
/// The resolved token string, or an error if no token could be found.
///
/// # Priority Order
///
/// 1. Explicit token (from CLI --token argument)
/// 2. `GITHUB_TOKEN` environment variable
/// 3. gh CLI authentication (if available)
///
/// # Errors
///
/// Returns an error if no token is found from any source.
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
/// Checks if the `gh` CLI is installed and authenticated, then retrieves
/// the stored token for the specified hostname.
///
/// # Arguments
///
/// * `hostname` - GitHub Enterprise hostname (e.g., "github.mycompany.com").
///   Pass `None` for public GitHub.
///
/// # Returns
///
/// * `Ok(Some(token))` - Successfully retrieved token from gh CLI.
/// * `Ok(None)` - gh CLI not installed or not authenticated for this host.
/// * `Err(_)` - Unexpected failure running gh commands.
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

/// Extract hostname from a GitHub URL for use with the gh CLI.
///
/// Public GitHub URLs return `None` because the gh CLI defaults to github.com.
/// Enterprise URLs return the hostname portion for use with `--hostname`.
///
/// # Arguments
///
/// * `enterprise_url` - Full GitHub URL (e.g., "https://github.mycompany.com").
///
/// # Returns
///
/// * `None` - For public GitHub URLs (https://github.com).
/// * `Some(hostname)` - For GitHub Enterprise URLs.
///
/// # Examples
///
/// ```
/// use deltav::github::extract_hostname;
///
/// // Public GitHub returns None
/// assert_eq!(extract_hostname("https://github.com"), None);
/// assert_eq!(extract_hostname("https://github.com/"), None);
///
/// // Enterprise URLs return the hostname
/// assert_eq!(
///     extract_hostname("https://github.mycompany.com"),
///     Some("github.mycompany.com".to_string())
/// );
/// ```
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
