//! GitHub API client implementation.
//!
//! This module provides the [`GitHubClient`] for interacting with GitHub's REST API.
//! It supports both public GitHub and GitHub Enterprise instances.

use super::types::*;
use crate::schema::GitHubConfig;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};

/// Client for interacting with GitHub REST API.
///
/// Supports both public GitHub (api.github.com) and GitHub Enterprise instances.
/// All API calls use token-based authentication and handle pagination automatically.
pub struct GitHubClient {
    /// The underlying HTTP client with authentication headers.
    client: Client,

    /// GitHub configuration (enterprise URL, organizations, etc.).
    config: GitHubConfig,
}

impl GitHubClient {
    /// Create a new GitHub client.
    ///
    /// Initializes the HTTP client with authentication headers for the GitHub API.
    ///
    /// # Arguments
    ///
    /// * `config` - GitHub configuration including enterprise URL and organization settings.
    /// * `token` - Personal access token for API authentication.
    ///
    /// # Returns
    ///
    /// A configured `GitHubClient` ready to make API calls, or an error if
    /// the HTTP client could not be initialized.
    ///
    /// # Errors
    ///
    /// Returns an error if the token format is invalid or the HTTP client
    /// cannot be created.
    pub fn new(config: GitHubConfig, token: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("token {}", token)).context("Invalid token format")?,
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github.v3+json"),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("deltav"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(GitHubClient { client, config })
    }

    /// Get the API base URL.
    ///
    /// # Returns
    ///
    /// The base URL for API requests (e.g., "https://api.github.com" or
    /// "https://github.enterprise.com/api/v3").
    fn api_url(&self) -> String {
        self.config.api_url()
    }

    /// Fetch all repositories for an organization or user that match the configured pattern.
    ///
    /// Retrieves repositories from the specified organization or user and filters them
    /// using the regex pattern configured for that organization.
    ///
    /// # Arguments
    ///
    /// * `org` - Organization or user name (must be configured in `GitHubConfig`).
    ///
    /// # Returns
    ///
    /// A vector of repositories matching the configured pattern.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The organization/user is not found in the configuration.
    /// - The repo pattern regex is invalid.
    /// - The API request fails.
    pub fn fetch_repos(&self, org: &str) -> Result<Vec<Repository>> {
        let org_config = self
            .config
            .organisations
            .iter()
            .find(|o| o.name == org)
            .context("Organization not found in config")?;

        let pattern =
            regex::Regex::new(&org_config.repo_pattern).context("Invalid repo pattern regex")?;

        // Use different endpoint for users vs organizations
        let endpoint = if org_config.is_user { "users" } else { "orgs" };

        let mut all_repos = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/{}/{}/repos?per_page=100&page={}",
                self.api_url(),
                endpoint,
                org,
                page
            );

            let response = self
                .client
                .get(&url)
                .send()
                .context("Failed to fetch repositories")?;

            // Check for error responses and provide helpful debugging info
            let status = response.status();
            if !status.is_success() {
                let body = response.text().unwrap_or_else(|_| "<no body>".to_string());
                anyhow::bail!(
                    "GitHub API error ({}): {}\nURL: {}\nHint: If '{}' is a user account, set is_user = true in your config",
                    status,
                    body,
                    url,
                    org
                );
            }

            let repos: Vec<Repository> = response
                .json()
                .context("Failed to parse repository response")?;

            if repos.is_empty() {
                break;
            }

            all_repos.extend(
                repos
                    .into_iter()
                    .filter(|repo| pattern.is_match(&repo.name)),
            );

            page += 1;
        }

        Ok(all_repos)
    }

    /// Fetch issues for a repository within a date range.
    ///
    /// Retrieves issues from the specified repository, optionally filtering
    /// by update date. Pull requests are automatically filtered out (use
    /// [`fetch_pull_requests`](Self::fetch_pull_requests) instead).
    ///
    /// # Arguments
    ///
    /// * `org` - Organization name.
    /// * `repo` - Repository name.
    /// * `since` - Only return issues updated after this date (optional).
    /// * `state` - Filter by issue state (open, closed, or all).
    ///
    /// # Returns
    ///
    /// A vector of issues matching the criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub fn fetch_issues(
        &self,
        org: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        state: IssueState,
    ) -> Result<Vec<Issue>> {
        let mut all_issues = Vec::new();
        let mut page = 1;

        loop {
            let mut url = format!(
                "{}/repos/{}/{}/issues?per_page=100&page={}&state={}",
                self.api_url(),
                org,
                repo,
                page,
                state.as_str()
            );

            if let Some(since) = since {
                url.push_str(&format!("&since={}", since.to_rfc3339()));
            }

            let response: Vec<Issue> = self
                .client
                .get(&url)
                .send()
                .context("Failed to fetch issues")?
                .json()
                .context("Failed to parse issues response")?;

            if response.is_empty() {
                break;
            }

            // Filter out pull requests (GitHub API returns PRs in issues endpoint)
            all_issues.extend(
                response
                    .into_iter()
                    .filter(|issue| issue.pull_request.is_none()),
            );

            page += 1;
        }

        Ok(all_issues)
    }

    /// Fetch pull requests for a repository.
    ///
    /// Retrieves pull requests from the specified repository, sorted by
    /// update date (newest first). Pagination stops when PRs older than
    /// the `since` date are encountered.
    ///
    /// # Arguments
    ///
    /// * `org` - Organization name.
    /// * `repo` - Repository name.
    /// * `state` - Filter by PR state (open, closed, or all).
    /// * `since` - Only return PRs updated after this date (optional).
    ///
    /// # Returns
    ///
    /// A vector of pull requests matching the criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub fn fetch_pull_requests(
        &self,
        org: &str,
        repo: &str,
        state: PullRequestState,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PullRequest>> {
        let mut all_prs = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/repos/{}/{}/pulls?per_page=100&page={}&state={}&sort=updated&direction=desc",
                self.api_url(),
                org,
                repo,
                page,
                state.as_str()
            );

            let response: Vec<PullRequest> = self
                .client
                .get(&url)
                .send()
                .context("Failed to fetch pull requests")?
                .json()
                .context("Failed to parse pull requests response")?;

            if response.is_empty() {
                break;
            }

            // If we have a since filter, stop when we hit older PRs
            let Some(since) = since else {
                all_prs.extend(response);
                page += 1;
                continue;
            };

            let response_len = response.len();
            let prev_len = all_prs.len();
            all_prs.extend(response.into_iter().take_while(|pr| pr.updated_at >= since));
            // If we didn't take all PRs, we hit an older one - stop paginating
            if all_prs.len() - prev_len < response_len {
                break;
            }

            page += 1;
        }

        Ok(all_prs)
    }

    /// Fetch events for an issue (for label change history).
    ///
    /// Retrieves the timeline of events for an issue, including label changes,
    /// assignment changes, and milestone updates.
    ///
    /// # Arguments
    ///
    /// * `org` - Organization name.
    /// * `repo` - Repository name.
    /// * `issue_number` - Issue number within the repository.
    ///
    /// # Returns
    ///
    /// A vector of all events for the issue.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub fn fetch_issue_events(
        &self,
        org: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Vec<IssueEvent>> {
        let mut all_events = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/repos/{}/{}/issues/{}/events?per_page=100&page={}",
                self.api_url(),
                org,
                repo,
                issue_number,
                page
            );

            let response: Vec<IssueEvent> = self
                .client
                .get(&url)
                .send()
                .context("Failed to fetch issue events")?
                .json()
                .context("Failed to parse issue events response")?;

            if response.is_empty() {
                break;
            }

            all_events.extend(response);
            page += 1;
        }

        Ok(all_events)
    }

    /// Fetch a single issue by number.
    ///
    /// # Arguments
    ///
    /// * `org` - Organization name.
    /// * `repo` - Repository name.
    /// * `number` - Issue number within the repository.
    ///
    /// # Returns
    ///
    /// The requested issue.
    ///
    /// # Errors
    ///
    /// Returns an error if the issue is not found or the API request fails.
    pub fn fetch_issue(&self, org: &str, repo: &str, number: u64) -> Result<Issue> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.api_url(),
            org,
            repo,
            number
        );

        let issue: Issue = self
            .client
            .get(&url)
            .send()
            .context("Failed to fetch issue")?
            .json()
            .context("Failed to parse issue response")?;

        Ok(issue)
    }

    /// Test the connection to GitHub.
    ///
    /// Fetches the authenticated user's information to verify that the
    /// token is valid and the API is reachable.
    ///
    /// # Returns
    ///
    /// The authenticated user's information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API is unreachable.
    pub fn test_connection(&self) -> Result<User> {
        let url = format!("{}/user", self.api_url());

        let user: User = self
            .client
            .get(&url)
            .send()
            .context("Failed to connect to GitHub")?
            .json()
            .context("Failed to parse user response")?;

        Ok(user)
    }

    /// Get rate limit status.
    ///
    /// Retrieves the current API rate limit status for the authenticated user.
    ///
    /// # Returns
    ///
    /// Rate limit information including remaining requests and reset time.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub fn rate_limit(&self) -> Result<RateLimit> {
        let url = format!("{}/rate_limit", self.api_url());

        let response: RateLimitResponse = self
            .client
            .get(&url)
            .send()
            .context("Failed to fetch rate limit")?
            .json()
            .context("Failed to parse rate limit response")?;

        Ok(response.rate)
    }

    /// Fetch releases for a repository.
    ///
    /// Retrieves published releases from a repository, excluding drafts.
    /// Results are returned in reverse chronological order (newest first).
    ///
    /// See: <https://docs.github.com/en/rest/releases/releases#list-releases>
    ///
    /// # Arguments
    ///
    /// * `org` - Organization or user name.
    /// * `repo` - Repository name.
    /// * `per_page` - Maximum number of releases to fetch (default 30, max 100).
    ///
    /// # Returns
    ///
    /// A vector of releases, excluding drafts.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub fn fetch_releases(&self, org: &str, repo: &str, per_page: Option<u32>) -> Result<Vec<Release>> {
        let per_page = per_page.unwrap_or(30).min(100);
        let url = format!(
            "{}/repos/{}/{}/releases?per_page={}",
            self.api_url(),
            org,
            repo,
            per_page
        );

        let releases: Vec<Release> = self
            .client
            .get(&url)
            .send()
            .with_context(|| format!("Failed to fetch releases from {}/{}", org, repo))?
            .json()
            .context("Failed to parse releases response")?;

        // Filter out drafts - only return published releases
        Ok(releases.into_iter().filter(|r| !r.draft).collect())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would go here, but require a real GitHub instance
}
