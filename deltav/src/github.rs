//! GitHub API client for fetching issues, PRs, and projects.
//!
//! This module handles communication with GitHub Enterprise instances.

pub mod auth;
pub mod client;
pub mod types;

pub use auth::{extract_hostname, resolve_token};
pub use client::GitHubClient;
pub use types::*;
