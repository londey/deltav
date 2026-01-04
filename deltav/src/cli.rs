//! Command-line interface definitions.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "deltav",
    author = "Nicholas",
    version,
    about = "DevSecOps metrics aggregator for GitHub Enterprise",
    long_about = "deltav tracks delivery metrics for systems engineering projects \
                  using data from GitHub Enterprise. It generates weekly reports \
                  showing team velocity, CSCI completion, and external dependency status."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Generate a stub project.toml with example values.
    Init {
        /// Output file path (default: project.toml in current directory).
        #[arg(short, long, default_value = "project.toml")]
        output: PathBuf,
    },

    /// Output JSON schema for project.toml (for editor autocomplete).
    Schema {
        /// Output file path (default: stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate a project.toml file.
    Validate {
        /// Path to project.toml.
        #[arg(default_value = "project.toml")]
        config: PathBuf,
    },

    /// Generate a weekly report.
    Report {
        /// Path to project.toml.
        #[arg(short, long, default_value = "project.toml")]
        config: PathBuf,

        /// Week to report on (ISO week format: YYYY-Www, e.g., 2026-W02).
        /// Defaults to the current week.
        #[arg(short, long)]
        week: Option<String>,

        /// Output format.
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: OutputFormat,

        /// Output directory (for 'all' format) or file path.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// GitHub personal access token (or set GITHUB_TOKEN env var).
        #[arg(long)]
        token: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Self-contained Markdown with embedded images as data URLs.
    Markdown,
    /// Self-contained HTML for viewing in a browser.
    Html,
    /// PDF for formal reporting.
    Pdf,
    /// Generate all formats.
    All,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Markdown => "md",
            OutputFormat::Html => "html",
            OutputFormat::Pdf => "pdf",
            OutputFormat::All => "", // Not used directly
        }
    }
}

/// Parse an ISO week string (YYYY-Www) into year and week number.
pub fn parse_iso_week(s: &str) -> anyhow::Result<(i32, u32)> {
    // Format: 2026-W02
    let parts: Vec<&str> = s.split("-W").collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid ISO week format. Expected YYYY-Www (e.g., 2026-W02)");
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid year in ISO week"))?;

    let week: u32 = parts[1]
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid week number in ISO week"))?;

    if !(1..=53).contains(&week) {
        anyhow::bail!("Week number must be between 1 and 53");
    }

    Ok((year, week))
}

/// Get the current ISO week.
pub fn current_iso_week() -> (i32, u32) {
    use chrono::{Datelike, Local};

    let today = Local::now().date_naive();
    let week = today.iso_week();
    (week.year(), week.week())
}

/// Format an ISO week as a string.
pub fn format_iso_week(year: i32, week: u32) -> String {
    format!("{}-W{:02}", year, week)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso_week() {
        let (year, week) = parse_iso_week("2026-W02").unwrap();
        assert_eq!(year, 2026);
        assert_eq!(week, 2);
    }

    #[test]
    fn test_parse_iso_week_single_digit() {
        let (year, week) = parse_iso_week("2026-W2").unwrap();
        assert_eq!(year, 2026);
        assert_eq!(week, 2);
    }

    #[test]
    fn test_parse_iso_week_invalid() {
        assert!(parse_iso_week("2026-02").is_err());
        assert!(parse_iso_week("invalid").is_err());
        assert!(parse_iso_week("2026-W54").is_err());
    }

    #[test]
    fn test_format_iso_week() {
        assert_eq!(format_iso_week(2026, 2), "2026-W02");
        assert_eq!(format_iso_week(2026, 52), "2026-W52");
    }
}
