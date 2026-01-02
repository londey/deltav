//! GitHub API client for fetching issues, PRs, and projects.
//!
//! This module handles communication with GitHub Enterprise instances.

pub mod client;
pub mod types;

pub use client::GitHubClient;
pub use types::*;
