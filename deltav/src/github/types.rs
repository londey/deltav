//! GitHub API response types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub html_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: Option<DateTime<Utc>>,
}

/// A GitHub issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub user: User,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub milestone: Option<Milestone>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    /// Present if this is actually a pull request.
    pub pull_request: Option<PullRequestRef>,
}

impl Issue {
    /// Check if the issue has a specific label.
    pub fn has_label(&self, label_name: &str) -> bool {
        self.labels.iter().any(|l| l.name == label_name)
    }

    /// Check if the issue has any label matching a prefix.
    pub fn has_label_prefix(&self, prefix: &str) -> bool {
        self.labels.iter().any(|l| l.name.starts_with(prefix))
    }

    /// Get the size label (XS, S, M, L, XL) if present.
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
    pub fn is_closed(&self) -> bool {
        self.state == "closed"
    }

    /// Check if the issue is open.
    pub fn is_open(&self) -> bool {
        self.state == "open"
    }

    /// Get assigned usernames.
    pub fn assignee_logins(&self) -> Vec<&str> {
        self.assignees.iter().map(|u| u.login.as_str()).collect()
    }
}

/// Reference to a pull request (used in Issue when it's actually a PR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestRef {
    pub url: String,
    pub html_url: String,
}

/// A GitHub pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub user: User,
    pub labels: Vec<Label>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub base: BranchRef,
    pub head: BranchRef,
}

impl PullRequest {
    /// Check if the PR was merged.
    pub fn is_merged(&self) -> bool {
        self.merged_at.is_some()
    }

    /// Check if the PR is to the main/master branch.
    pub fn is_to_main(&self) -> bool {
        matches!(self.base.ref_name.as_str(), "main" | "master")
    }

    /// Calculate cycle time (created to merged) if merged.
    pub fn cycle_time_hours(&self) -> Option<f64> {
        let merged = self.merged_at?;
        let duration = merged - self.created_at;
        Some(duration.num_minutes() as f64 / 60.0)
    }
}

/// A branch reference in a PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// A GitHub label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

/// A GitHub user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub login: String,
    pub html_url: String,
    #[serde(rename = "type")]
    pub user_type: Option<String>,
}

/// A GitHub milestone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub due_on: Option<DateTime<Utc>>,
}

/// An issue event (for tracking label changes, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueEvent {
    pub id: u64,
    pub event: String,
    pub created_at: DateTime<Utc>,
    pub actor: Option<User>,
    pub label: Option<Label>,
    pub assignee: Option<User>,
    pub milestone: Option<Milestone>,
}

impl IssueEvent {
    /// Check if this is a label event.
    pub fn is_label_event(&self) -> bool {
        matches!(self.event.as_str(), "labeled" | "unlabeled")
    }

    /// Check if this is an assignment event.
    pub fn is_assignment_event(&self) -> bool {
        matches!(self.event.as_str(), "assigned" | "unassigned")
    }
}

/// Issue state filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
    All,
}

impl IssueState {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueState::Open => "open",
            IssueState::Closed => "closed",
            IssueState::All => "all",
        }
    }
}

/// Pull request state filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Closed,
    All,
}

impl PullRequestState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PullRequestState::Open => "open",
            PullRequestState::Closed => "closed",
            PullRequestState::All => "all",
        }
    }
}

/// Rate limit information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    pub reset: u64,
}

impl RateLimit {
    /// Check if we're close to the rate limit.
    pub fn is_low(&self) -> bool {
        self.remaining < 100
    }

    /// Time until reset as a human-readable string.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResponse {
    pub rate: RateLimit,
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
