//! External dependencies schema.

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// External dependencies and prerequisites.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Dependencies {
    /// External dependencies from other teams/projects.
    #[serde(default)]
    pub external: Vec<ExternalDependency>,
}

impl Dependencies {
    /// Get dependencies that are overdue (RC or final).
    pub fn overdue(&self, as_of: NaiveDate) -> Vec<&ExternalDependency> {
        self.external
            .iter()
            .filter(|d| d.is_rc_overdue(as_of) || d.is_final_overdue(as_of))
            .collect()
    }

    /// Get dependencies with upcoming RC deadlines.
    pub fn upcoming_rc(&self, as_of: NaiveDate, within_days: i64) -> Vec<&ExternalDependency> {
        let cutoff = as_of + chrono::Duration::days(within_days);
        self.external
            .iter()
            .filter(|d| d.rc_due >= as_of && d.rc_due <= cutoff)
            .collect()
    }

    /// Get dependencies with upcoming final deadlines.
    pub fn upcoming_final(&self, as_of: NaiveDate, within_days: i64) -> Vec<&ExternalDependency> {
        let cutoff = as_of + chrono::Duration::days(within_days);
        self.external
            .iter()
            .filter(|d| d.final_due >= as_of && d.final_due <= cutoff)
            .collect()
    }

    /// Find a dependency by ID.
    pub fn by_id(&self, id: &str) -> Option<&ExternalDependency> {
        self.external.iter().find(|d| d.id == id)
    }
}

/// An external dependency from another team or project.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExternalDependency {
    /// Human-readable name.
    pub name: String,

    /// Unique identifier (e.g., "ICD-001").
    pub id: String,

    /// Team or entity responsible for this dependency.
    pub owner: String,

    /// Due date for Release Candidate version.
    pub rc_due: NaiveDate,

    /// Due date for final/approved version.
    pub final_due: NaiveDate,

    /// Optional GitHub issue tracking this dependency (org/repo#number format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking_issue: Option<String>,
}

impl ExternalDependency {
    /// Check if RC is overdue.
    pub fn is_rc_overdue(&self, as_of: NaiveDate) -> bool {
        as_of > self.rc_due
    }

    /// Check if final is overdue.
    pub fn is_final_overdue(&self, as_of: NaiveDate) -> bool {
        as_of > self.final_due
    }

    /// Days until RC due (negative if overdue).
    pub fn days_until_rc(&self, as_of: NaiveDate) -> i64 {
        (self.rc_due - as_of).num_days()
    }

    /// Days until final due (negative if overdue).
    pub fn days_until_final(&self, as_of: NaiveDate) -> i64 {
        (self.final_due - as_of).num_days()
    }

    /// Get the status of this dependency.
    pub fn status(
        &self,
        as_of: NaiveDate,
        rc_received: bool,
        final_received: bool,
    ) -> DependencyStatus {
        if final_received {
            DependencyStatus::Complete
        } else if rc_received {
            if self.is_final_overdue(as_of) {
                DependencyStatus::FinalOverdue
            } else {
                DependencyStatus::RcReceived
            }
        } else if self.is_rc_overdue(as_of) {
            DependencyStatus::RcOverdue
        } else {
            DependencyStatus::Pending
        }
    }

    /// Parse tracking issue into (org, repo, number) if present.
    pub fn parse_tracking_issue(&self) -> Option<(&str, &str, u32)> {
        let issue = self.tracking_issue.as_ref()?;

        // Format: org/repo#number
        let parts: Vec<&str> = issue.splitn(2, '#').collect();
        if parts.len() != 2 {
            return None;
        }

        let number: u32 = parts[1].parse().ok()?;
        let repo_parts: Vec<&str> = parts[0].splitn(2, '/').collect();
        if repo_parts.len() != 2 {
            return None;
        }

        Some((repo_parts[0], repo_parts[1], number))
    }
}

/// Status of an external dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyStatus {
    /// Waiting for RC, not yet due.
    Pending,
    /// RC due date has passed without receipt.
    RcOverdue,
    /// RC received, waiting for final.
    RcReceived,
    /// Final due date has passed without receipt.
    FinalOverdue,
    /// Final version received.
    Complete,
}

impl DependencyStatus {
    /// Human-readable status string.
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencyStatus::Pending => "Pending",
            DependencyStatus::RcOverdue => "RC Overdue",
            DependencyStatus::RcReceived => "RC Received",
            DependencyStatus::FinalOverdue => "Final Overdue",
            DependencyStatus::Complete => "Complete",
        }
    }

    /// Symbol for compact display.
    pub fn symbol(&self) -> &'static str {
        match self {
            DependencyStatus::Pending => "○",
            DependencyStatus::RcOverdue => "✗",
            DependencyStatus::RcReceived => "◐",
            DependencyStatus::FinalOverdue => "✗",
            DependencyStatus::Complete => "✓",
        }
    }

    /// Is this status problematic?
    pub fn is_at_risk(&self) -> bool {
        matches!(
            self,
            DependencyStatus::RcOverdue | DependencyStatus::FinalOverdue
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dependency() -> ExternalDependency {
        ExternalDependency {
            name: "Test ICD".to_string(),
            id: "ICD-001".to_string(),
            owner: "Other Team".to_string(),
            rc_due: NaiveDate::from_ymd_opt(2025, 8, 1).unwrap(),
            final_due: NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
            tracking_issue: Some("my-org/my-repo#42".to_string()),
        }
    }

    #[test]
    fn test_overdue_checks() {
        let dep = sample_dependency();

        let before_rc = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();
        let between = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        let after_final = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        assert!(!dep.is_rc_overdue(before_rc));
        assert!(dep.is_rc_overdue(between));
        assert!(!dep.is_final_overdue(between));
        assert!(dep.is_final_overdue(after_final));
    }

    #[test]
    fn test_status() {
        let dep = sample_dependency();
        let after_rc = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();

        assert_eq!(
            dep.status(after_rc, false, false),
            DependencyStatus::RcOverdue
        );
        assert_eq!(
            dep.status(after_rc, true, false),
            DependencyStatus::RcReceived
        );
        assert_eq!(dep.status(after_rc, true, true), DependencyStatus::Complete);
    }

    #[test]
    fn test_parse_tracking_issue() {
        let dep = sample_dependency();
        let parsed = dep.parse_tracking_issue().unwrap();
        assert_eq!(parsed, ("my-org", "my-repo", 42));
    }

    #[test]
    fn test_parse_tracking_issue_invalid() {
        let mut dep = sample_dependency();
        dep.tracking_issue = Some("invalid-format".to_string());
        assert!(dep.parse_tracking_issue().is_none());
    }
}
