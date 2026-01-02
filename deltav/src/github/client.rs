//! GitHub API client implementation.

use super::types::*;
use crate::schema::GitHubConfig;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};

/// Client for interacting with GitHub Enterprise API.
pub struct GitHubClient {
    client: Client,
    config: GitHubConfig,
}

impl GitHubClient {
    /// Create a new GitHub client.
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
    fn api_url(&self) -> String {
        self.config.api_url()
    }

    /// Fetch all repositories for an organization that match the configured pattern.
    pub fn fetch_repos(&self, org: &str) -> Result<Vec<Repository>> {
        let org_config = self
            .config
            .organisations
            .iter()
            .find(|o| o.name == org)
            .context("Organization not found in config")?;

        let pattern =
            regex::Regex::new(&org_config.repo_pattern).context("Invalid repo pattern regex")?;

        let mut all_repos = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/orgs/{}/repos?per_page=100&page={}",
                self.api_url(),
                org,
                page
            );

            let response: Vec<Repository> = self
                .client
                .get(&url)
                .send()
                .context("Failed to fetch repositories")?
                .json()
                .context("Failed to parse repository response")?;

            if response.is_empty() {
                break;
            }

            for repo in response {
                if pattern.is_match(&repo.name) {
                    all_repos.push(repo);
                }
            }

            page += 1;
        }

        Ok(all_repos)
    }

    /// Fetch issues for a repository within a date range.
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
            for issue in response {
                if issue.pull_request.is_none() {
                    all_issues.push(issue);
                }
            }

            page += 1;
        }

        Ok(all_issues)
    }

    /// Fetch pull requests for a repository.
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
            if let Some(since) = since {
                let mut found_older = false;
                for pr in response {
                    if pr.updated_at >= since {
                        all_prs.push(pr);
                    } else {
                        found_older = true;
                        break;
                    }
                }
                if found_older {
                    break;
                }
            } else {
                all_prs.extend(response);
            }

            page += 1;
        }

        Ok(all_prs)
    }

    /// Fetch events for an issue (for label change history).
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

    /// Test the connection to GitHub Enterprise.
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
}

#[cfg(test)]
mod tests {
    // Integration tests would go here, but require a real GitHub instance
}
