//! Deliverables schema - documents, CSCIs, and demonstrations.

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// All project deliverables.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Deliverables {
    /// Document deliverables (SRS, SDD, etc.).
    #[serde(default)]
    pub documents: Vec<Document>,

    /// Computer Software Configuration Items.
    #[serde(default)]
    pub csci: Vec<Csci>,

    /// Demonstrations and reviews.
    #[serde(default)]
    pub demonstrations: Vec<Demonstration>,
}

impl Deliverables {
    /// Get all deliverable IDs.
    pub fn all_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        ids.extend(self.documents.iter().map(|d| d.id.as_str()));
        ids.extend(self.csci.iter().map(|c| c.id.as_str()));
        ids.extend(self.demonstrations.iter().map(|d| d.id.as_str()));
        ids
    }

    /// Find a document by ID.
    pub fn document_by_id(&self, id: &str) -> Option<&Document> {
        self.documents.iter().find(|d| d.id == id)
    }

    /// Find a CSCI by ID.
    pub fn csci_by_id(&self, id: &str) -> Option<&Csci> {
        self.csci.iter().find(|c| c.id == id)
    }

    /// Find a demonstration by ID.
    pub fn demonstration_by_id(&self, id: &str) -> Option<&Demonstration> {
        self.demonstrations.iter().find(|d| d.id == id)
    }

    /// Get upcoming milestones within a number of days.
    pub fn upcoming_milestones(&self, as_of: NaiveDate, within_days: i64) -> Vec<Milestone> {
        let cutoff = as_of + chrono::Duration::days(within_days);
        let mut milestones = Vec::new();

        for doc in &self.documents {
            if doc.due_date >= as_of && doc.due_date <= cutoff {
                milestones.push(Milestone {
                    id: doc.id.clone(),
                    name: doc.name.clone(),
                    date: doc.due_date,
                    kind: MilestoneKind::Document,
                });
            }
        }

        for csci in &self.csci {
            if csci.target_date >= as_of && csci.target_date <= cutoff {
                milestones.push(Milestone {
                    id: csci.id.clone(),
                    name: csci.name.clone(),
                    date: csci.target_date,
                    kind: MilestoneKind::Csci,
                });
            }
        }

        for demo in &self.demonstrations {
            if demo.start_date >= as_of && demo.start_date <= cutoff {
                milestones.push(Milestone {
                    id: demo.id.clone(),
                    name: demo.name.clone(),
                    date: demo.start_date,
                    kind: MilestoneKind::Demonstration,
                });
            }
        }

        milestones.sort_by_key(|m| m.date);
        milestones
    }
}

/// A milestone for timeline reporting.
#[derive(Debug, Clone)]
pub struct Milestone {
    pub id: String,
    pub name: String,
    pub date: NaiveDate,
    pub kind: MilestoneKind,
}

impl Milestone {
    /// Days until this milestone from a given date.
    pub fn days_until(&self, from: NaiveDate) -> i64 {
        (self.date - from).num_days()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneKind {
    Document,
    Csci,
    Demonstration,
}

/// A document deliverable.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Document {
    /// Human-readable document name.
    pub name: String,

    /// Unique identifier (e.g., "SRS-001").
    pub id: String,

    /// Target delivery date.
    pub due_date: NaiveDate,

    /// Optional label used to track document status in GitHub issues.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_label: Option<String>,

    /// IDs of documents this depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

impl Document {
    /// Check if this document is overdue.
    pub fn is_overdue(&self, as_of: NaiveDate) -> bool {
        as_of > self.due_date
    }

    /// Days until due (negative if overdue).
    pub fn days_until_due(&self, as_of: NaiveDate) -> i64 {
        (self.due_date - as_of).num_days()
    }
}

/// A Computer Software Configuration Item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Csci {
    /// Human-readable CSCI name.
    pub name: String,

    /// Unique identifier (e.g., "CSCI-FCU").
    pub id: String,

    /// Target delivery date.
    pub target_date: NaiveDate,

    /// Repositories that comprise this CSCI (org/repo format).
    pub repos: Vec<String>,

    /// Label indicating a ticket is integration-ready (Tier 1).
    pub tier1_label: String,

    /// Label indicating a ticket has passed HIL testing (Tier 2).
    pub tier2_label: String,
}

impl Csci {
    /// Check if a repository belongs to this CSCI.
    pub fn contains_repo(&self, org: &str, repo: &str) -> bool {
        let full = format!("{}/{}", org, repo);
        self.repos.iter().any(|r| r == &full)
    }

    /// Parse repos into (org, repo) pairs.
    pub fn repo_pairs(&self) -> Vec<(&str, &str)> {
        self.repos
            .iter()
            .filter_map(|r| {
                let parts: Vec<&str> = r.splitn(2, '/').collect();
                if parts.len() == 2 {
                    Some((parts[0], parts[1]))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if this CSCI is overdue.
    pub fn is_overdue(&self, as_of: NaiveDate) -> bool {
        as_of > self.target_date
    }

    /// Days until target (negative if overdue).
    pub fn days_until_target(&self, as_of: NaiveDate) -> i64 {
        (self.target_date - as_of).num_days()
    }
}

/// A demonstration or review milestone.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Demonstration {
    /// Human-readable name.
    pub name: String,

    /// Unique identifier (e.g., "DEMO-PDR").
    pub id: String,

    /// First day of the demonstration.
    pub start_date: NaiveDate,

    /// Last day of the demonstration.
    pub end_date: NaiveDate,

    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Demonstration {
    /// Duration in days (inclusive).
    pub fn duration_days(&self) -> i64 {
        (self.end_date - self.start_date).num_days() + 1
    }

    /// Check if a date falls within this demonstration.
    pub fn contains_date(&self, date: NaiveDate) -> bool {
        date >= self.start_date && date <= self.end_date
    }

    /// Days until start (negative if past).
    pub fn days_until_start(&self, as_of: NaiveDate) -> i64 {
        (self.start_date - as_of).num_days()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csci_repo_pairs() {
        let csci = Csci {
            name: "Test".to_string(),
            id: "CSCI-TEST".to_string(),
            target_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            repos: vec!["org-a/repo-1".to_string(), "org-b/repo-2".to_string()],
            tier1_label: "ready".to_string(),
            tier2_label: "done".to_string(),
        };

        let pairs = csci.repo_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("org-a", "repo-1"));
        assert_eq!(pairs[1], ("org-b", "repo-2"));
    }

    #[test]
    fn test_document_overdue() {
        let doc = Document {
            name: "Test Doc".to_string(),
            id: "DOC-001".to_string(),
            due_date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            status_label: None,
            depends_on: vec![],
        };

        let before = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let after = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        assert!(!doc.is_overdue(before));
        assert!(doc.is_overdue(after));
    }

    #[test]
    fn test_demonstration_contains_date() {
        let demo = Demonstration {
            name: "PDR".to_string(),
            id: "DEMO-PDR".to_string(),
            start_date: NaiveDate::from_ymd_opt(2025, 10, 14).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 10, 16).unwrap(),
            description: None,
        };

        let during = NaiveDate::from_ymd_opt(2025, 10, 15).unwrap();
        let before = NaiveDate::from_ymd_opt(2025, 10, 13).unwrap();
        let after = NaiveDate::from_ymd_opt(2025, 10, 17).unwrap();

        assert!(demo.contains_date(during));
        assert!(!demo.contains_date(before));
        assert!(!demo.contains_date(after));
    }
}
