//! GitHub API response types.
//!
//! These types correspond to objects returned by the GitHub REST API v3.
//! See the [GitHub REST API documentation](https://docs.github.com/en/rest) for details.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A GitHub repository.
///
/// Corresponds to the repository object from the GitHub REST API.
/// See: <https://docs.github.com/en/rest/repos/repos#get-a-repository>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique identifier for the repository.
    pub id: u64,

    /// Short name of the repository (e.g., "deltav").
    pub name: String,

    /// Full name including owner (e.g., "org/deltav").
    pub full_name: String,

    /// Repository description, if set.
    pub description: Option<String>,

    /// Whether the repository is private.
    pub private: bool,

    /// URL to the repository on GitHub.
    pub html_url: String,

    /// When the repository was created.
    pub created_at: DateTime<Utc>,

    /// When the repository was last updated.
    pub updated_at: DateTime<Utc>,

    /// When the repository was last pushed to.
    pub pushed_at: Option<DateTime<Utc>>,
}

/// A GitHub issue.
///
/// Corresponds to the issue object from the GitHub REST API.
/// See: <https://docs.github.com/en/rest/issues/issues#get-an-issue>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Unique identifier for the issue.
    pub id: u64,

    /// Issue number within the repository.
    pub number: u64,

    /// Issue title.
    pub title: String,

    /// Issue body/description in Markdown.
    pub body: Option<String>,

    /// Current state ("open" or "closed").
    pub state: String,

    /// URL to the issue on GitHub.
    pub html_url: String,

    /// URL to the repository API endpoint (e.g., "https://api.github.com/repos/owner/repo").
    pub repository_url: String,

    /// User who created the issue.
    pub user: User,

    /// Labels attached to the issue.
    pub labels: Vec<Label>,

    /// Users assigned to the issue.
    pub assignees: Vec<User>,

    /// Milestone the issue belongs to, if any.
    pub milestone: Option<Milestone>,

    /// When the issue was created.
    pub created_at: DateTime<Utc>,

    /// When the issue was last updated.
    pub updated_at: DateTime<Utc>,

    /// When the issue was closed, if closed.
    pub closed_at: Option<DateTime<Utc>>,

    /// Present if this is actually a pull request (issues API returns PRs too).
    pub pull_request: Option<PullRequestRef>,
}

impl Issue {
    /// Check if the issue has a specific label.
    ///
    /// # Arguments
    ///
    /// * `label_name` - Exact label name to search for.
    ///
    /// # Returns
    ///
    /// `true` if the issue has a label with the exact name.
    pub fn has_label(&self, label_name: &str) -> bool {
        self.labels.iter().any(|l| l.name == label_name)
    }

    /// Check if the issue has any label matching a prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Label name prefix to search for.
    ///
    /// # Returns
    ///
    /// `true` if any label starts with the given prefix.
    pub fn has_label_prefix(&self, prefix: &str) -> bool {
        self.labels.iter().any(|l| l.name.starts_with(prefix))
    }

    /// Get the size label (XS, S, M, L, XL) if present.
    ///
    /// Recognizes both bare size labels ("M", "XL") and prefixed labels ("size:M").
    ///
    /// # Returns
    ///
    /// The size label string if found, or `None` if no size label is present.
    pub fn size_label(&self) -> Option<&str> {
        const SIZES: [&str; 5] = ["XS", "S", "M", "L", "XL"];

        self.labels.iter().find_map(|label| {
            let upper = label.name.to_uppercase();
            let is_size = SIZES.contains(&upper.as_str());
            let is_prefixed_size = label
                .name
                .strip_prefix("size:")
                .map(|s| SIZES.contains(&s.trim().to_uppercase().as_str()))
                .unwrap_or(false);

            (is_size || is_prefixed_size).then_some(label.name.as_str())
        })
    }

    /// Check if the issue is closed.
    ///
    /// # Returns
    ///
    /// `true` if the issue state is "closed".
    pub fn is_closed(&self) -> bool {
        self.state == "closed"
    }

    /// Check if the issue is open.
    ///
    /// # Returns
    ///
    /// `true` if the issue state is "open".
    pub fn is_open(&self) -> bool {
        self.state == "open"
    }

    /// Get assigned usernames.
    ///
    /// # Returns
    ///
    /// A vector of login names for all assignees.
    pub fn assignee_logins(&self) -> Vec<&str> {
        self.assignees.iter().map(|u| u.login.as_str()).collect()
    }

    /// Extract org and repo from repository_url.
    ///
    /// Parses the repository API URL to extract the organization and repository names.
    ///
    /// # Returns
    ///
    /// A tuple of (org, repo) if the URL can be parsed, or `None` otherwise.
    ///
    /// # Example
    ///
    /// For `repository_url = "https://api.github.com/repos/myorg/myrepo"`,
    /// returns `Some(("myorg", "myrepo"))`.
    pub fn org_repo(&self) -> Option<(&str, &str)> {
        let parts: Vec<_> = self.repository_url.split('/').collect();
        if parts.len() >= 2 {
            let repo = parts[parts.len() - 1];
            let org = parts[parts.len() - 2];
            Some((org, repo))
        } else {
            None
        }
    }
}

/// Reference to a pull request (used in Issue when it's actually a PR).
///
/// When fetching issues, GitHub includes this field for issues that are
/// actually pull requests, allowing callers to distinguish between them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestRef {
    /// API URL for the pull request.
    pub url: String,

    /// Web URL for the pull request.
    pub html_url: String,
}

/// A GitHub pull request.
///
/// Corresponds to the pull request object from the GitHub REST API.
/// See: <https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// Unique identifier for the pull request.
    pub id: u64,

    /// Pull request number within the repository.
    pub number: u64,

    /// Pull request title.
    pub title: String,

    /// Pull request body/description in Markdown.
    pub body: Option<String>,

    /// Current state ("open", "closed").
    pub state: String,

    /// URL to the pull request on GitHub.
    pub html_url: String,

    /// User who created the pull request.
    pub user: User,

    /// Labels attached to the pull request.
    pub labels: Vec<Label>,

    /// When the pull request was created.
    pub created_at: DateTime<Utc>,

    /// When the pull request was last updated.
    pub updated_at: DateTime<Utc>,

    /// When the pull request was closed, if closed.
    pub closed_at: Option<DateTime<Utc>>,

    /// When the pull request was merged, if merged.
    pub merged_at: Option<DateTime<Utc>>,

    /// The base branch (target of the merge).
    pub base: BranchRef,

    /// The head branch (source of the merge).
    pub head: BranchRef,
}

impl PullRequest {
    /// Check if the PR was merged.
    ///
    /// # Returns
    ///
    /// `true` if the pull request has been merged.
    pub fn is_merged(&self) -> bool {
        self.merged_at.is_some()
    }

    /// Check if the PR is to the main/master branch.
    ///
    /// # Returns
    ///
    /// `true` if the base branch is "main" or "master".
    pub fn is_to_main(&self) -> bool {
        matches!(self.base.ref_name.as_str(), "main" | "master")
    }

    /// Calculate cycle time (created to merged) if merged.
    ///
    /// Cycle time measures how long it took from PR creation to merge,
    /// which is a key DevOps metric for delivery speed.
    ///
    /// # Returns
    ///
    /// The number of hours from creation to merge, or `None` if not merged.
    pub fn cycle_time_hours(&self) -> Option<f64> {
        let merged = self.merged_at?;
        let duration = merged - self.created_at;
        Some(duration.num_minutes() as f64 / 60.0)
    }
}

/// A branch reference in a PR.
///
/// Contains information about a branch involved in a pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchRef {
    /// Branch name (e.g., "main", "feature-branch").
    #[serde(rename = "ref")]
    pub ref_name: String,

    /// SHA of the commit at the tip of the branch.
    pub sha: String,
}

/// A GitHub label.
///
/// Labels are used to categorize issues and pull requests.
/// See: <https://docs.github.com/en/rest/issues/labels>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    /// Unique identifier for the label.
    pub id: u64,

    /// Label name (e.g., "bug", "enhancement", "M").
    pub name: String,

    /// Hex color code without the leading `#` (e.g., "ff0000").
    pub color: String,

    /// Label description, if set.
    pub description: Option<String>,
}

/// A GitHub user.
///
/// Represents a GitHub user account (can be a person or bot).
/// See: <https://docs.github.com/en/rest/users/users>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier for the user.
    pub id: u64,

    /// Username/login (e.g., "octocat").
    pub login: String,

    /// URL to the user's GitHub profile.
    pub html_url: String,

    /// Account type (e.g., "User", "Bot", "Organization").
    #[serde(rename = "type")]
    pub user_type: Option<String>,
}

/// A GitHub milestone.
///
/// Milestones group issues and pull requests into larger goals.
/// See: <https://docs.github.com/en/rest/issues/milestones>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Unique identifier for the milestone.
    pub id: u64,

    /// Milestone number within the repository.
    pub number: u32,

    /// Milestone title.
    pub title: String,

    /// Milestone description, if set.
    pub description: Option<String>,

    /// Current state ("open" or "closed").
    pub state: String,

    /// Due date for the milestone, if set.
    pub due_on: Option<DateTime<Utc>>,
}

/// An issue event (for tracking label changes, etc.).
///
/// Events represent actions taken on an issue, such as labeling,
/// assignment changes, or milestone updates.
/// See: <https://docs.github.com/en/rest/issues/events>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueEvent {
    /// Unique identifier for the event.
    pub id: u64,

    /// Event type (e.g., "labeled", "unlabeled", "assigned", "closed").
    pub event: String,

    /// When the event occurred.
    pub created_at: DateTime<Utc>,

    /// User who triggered the event.
    pub actor: Option<User>,

    /// Label involved (for "labeled"/"unlabeled" events).
    pub label: Option<Label>,

    /// Assignee involved (for "assigned"/"unassigned" events).
    pub assignee: Option<User>,

    /// Milestone involved (for milestone events).
    pub milestone: Option<Milestone>,
}

impl IssueEvent {
    /// Check if this is a label event.
    ///
    /// # Returns
    ///
    /// `true` if the event is "labeled" or "unlabeled".
    pub fn is_label_event(&self) -> bool {
        matches!(self.event.as_str(), "labeled" | "unlabeled")
    }

    /// Check if this is an assignment event.
    ///
    /// # Returns
    ///
    /// `true` if the event is "assigned" or "unassigned".
    pub fn is_assignment_event(&self) -> bool {
        matches!(self.event.as_str(), "assigned" | "unassigned")
    }
}

/// Issue state filter for API queries.
///
/// Used to filter issues by their open/closed state when fetching from the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueState {
    /// Only open issues.
    Open,

    /// Only closed issues.
    Closed,

    /// Both open and closed issues.
    All,
}

impl IssueState {
    /// Convert to the API query parameter value.
    ///
    /// # Returns
    ///
    /// The string value expected by the GitHub API.
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueState::Open => "open",
            IssueState::Closed => "closed",
            IssueState::All => "all",
        }
    }
}

/// Pull request state filter for API queries.
///
/// Used to filter pull requests by their open/closed state when fetching from the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestState {
    /// Only open pull requests.
    Open,

    /// Only closed pull requests (includes merged).
    Closed,

    /// All pull requests regardless of state.
    All,
}

impl PullRequestState {
    /// Convert to the API query parameter value.
    ///
    /// # Returns
    ///
    /// The string value expected by the GitHub API.
    pub fn as_str(&self) -> &'static str {
        match self {
            PullRequestState::Open => "open",
            PullRequestState::Closed => "closed",
            PullRequestState::All => "all",
        }
    }
}

/// Rate limit information from the GitHub API.
///
/// GitHub enforces rate limits on API requests. This struct contains
/// the current rate limit status returned in API response headers.
/// See: <https://docs.github.com/en/rest/rate-limit>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum requests allowed per hour.
    pub limit: u32,

    /// Remaining requests in the current window.
    pub remaining: u32,

    /// Unix timestamp when the rate limit resets.
    pub reset: u64,
}

impl RateLimit {
    /// Check if we're close to the rate limit.
    ///
    /// # Returns
    ///
    /// `true` if fewer than 100 requests remain.
    pub fn is_low(&self) -> bool {
        self.remaining < 100
    }

    /// Time until reset as a human-readable string.
    ///
    /// # Returns
    ///
    /// A string like "now", "45s", or "12m" indicating time until reset.
    pub fn reset_in(&self) -> String {
        let now = chrono::Utc::now().timestamp() as u64;
        if self.reset <= now {
            "now".to_string()
        } else {
            let seconds = self.reset - now;
            if seconds < 60 {
                format!("{}s", seconds)
            } else {
                format!("{}m", seconds / 60)
            }
        }
    }
}

/// Rate limit API response wrapper.
///
/// Wraps the rate limit information returned by the `/rate_limit` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResponse {
    /// The core rate limit information.
    pub rate: RateLimit,
}

/// A GitHub release.
///
/// Corresponds to the release object from the GitHub REST API.
/// See: <https://docs.github.com/en/rest/releases/releases#get-a-release>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    /// Unique identifier for the release.
    pub id: u64,

    /// Tag name for this release (e.g., "v1.0.0").
    pub tag_name: String,

    /// Human-readable release name.
    pub name: Option<String>,

    /// Release description in Markdown.
    pub body: Option<String>,

    /// Whether this is a draft release.
    pub draft: bool,

    /// Whether this is a pre-release.
    pub prerelease: bool,

    /// When the release was created.
    pub created_at: DateTime<Utc>,

    /// When the release was published.
    pub published_at: Option<DateTime<Utc>>,

    /// URL to the release on GitHub.
    pub html_url: String,

    /// User who created the release.
    pub author: User,
}

impl Release {
    /// Get the display name for this release.
    ///
    /// Returns the name if set, otherwise the tag name.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.tag_name)
    }

    /// Check if this release was published within a date range.
    pub fn published_in_range(
        &self,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> bool {
        if let Some(published) = self.published_at {
            let date = published.date_naive();
            date >= start && date <= end
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_has_label() {
        let issue = Issue {
            id: 1,
            number: 1,
            title: "Test".to_string(),
            body: None,
            state: "open".to_string(),
            html_url: "https://example.com".to_string(),
            repository_url: "https://api.github.com/repos/test-org/test-repo".to_string(),
            user: User {
                id: 1,
                login: "test".to_string(),
                html_url: "https://example.com".to_string(),
                user_type: None,
            },
            labels: vec![
                Label {
                    id: 1,
                    name: "bug".to_string(),
                    color: "red".to_string(),
                    description: None,
                },
                Label {
                    id: 2,
                    name: "M".to_string(),
                    color: "blue".to_string(),
                    description: None,
                },
            ],
            assignees: vec![],
            milestone: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
            pull_request: None,
        };

        assert!(issue.has_label("bug"));
        assert!(!issue.has_label("feature"));
        assert_eq!(issue.size_label(), Some("M"));
    }

    #[test]
    fn test_pr_cycle_time() {
        use chrono::Duration;

        let created = Utc::now() - Duration::hours(48);
        let merged = Utc::now();

        let pr = PullRequest {
            id: 1,
            number: 1,
            title: "Test".to_string(),
            body: None,
            state: "closed".to_string(),
            html_url: "https://example.com".to_string(),
            user: User {
                id: 1,
                login: "test".to_string(),
                html_url: "https://example.com".to_string(),
                user_type: None,
            },
            labels: vec![],
            created_at: created,
            updated_at: merged,
            closed_at: Some(merged),
            merged_at: Some(merged),
            base: BranchRef {
                ref_name: "main".to_string(),
                sha: "abc".to_string(),
            },
            head: BranchRef {
                ref_name: "feature".to_string(),
                sha: "def".to_string(),
            },
        };

        let cycle_time = pr.cycle_time_hours().unwrap();
        assert!((cycle_time - 48.0).abs() < 0.1);
    }
}
