//! Schema definitions for deltav project configuration.
//!
//! This module defines the structure of `project.toml` files that describe
//! a systems engineering project for metrics tracking.

mod project;
mod team;
mod github;
mod deliverables;
mod dependencies;
mod sizing;

pub use project::Project;
pub use team::{Team, TeamMember, Leave};
pub use github::{GitHubConfig, Organisation, GitHubProject, Distraction};
pub use deliverables::{Deliverables, Document, Csci, Demonstration};
pub use dependencies::{Dependencies, ExternalDependency};
pub use sizing::{Sizing, TShirtSize};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Root configuration structure for a deltav project.
///
/// This is the top-level structure that gets serialized to/from `project.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectConfig {
    /// Core project metadata
    pub project: Project,

    /// Team composition and capacity
    pub team: Team,

    /// GitHub Enterprise configuration
    pub github: GitHubConfig,

    /// Project deliverables (documents, CSCIs, demonstrations)
    pub deliverables: Deliverables,

    /// External dependencies and prerequisites
    pub dependencies: Dependencies,

    /// T-shirt sizing definitions for effort estimation
    #[serde(default)]
    pub sizing: Sizing,
}

impl ProjectConfig {
    /// Generate a stub configuration with example values and comments.
    pub fn stub() -> Self {
        use chrono::NaiveDate;

        ProjectConfig {
            project: Project {
                name: "Project Name".to_string(),
                start_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2026, 12, 15).unwrap(),
                backlog_completeness: 0.85,
            },
            team: Team {
                members: vec![
                    TeamMember {
                        name: "Alice Chen".to_string(),
                        github: "achen".to_string(),
                        capacity: 1.0,
                    },
                    TeamMember {
                        name: "Bob Martinez".to_string(),
                        github: "bmartinez".to_string(),
                        capacity: 0.8,
                    },
                ],
                leave: vec![
                    Leave {
                        github: "achen".to_string(),
                        start: NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
                        end: NaiveDate::from_ymd_opt(2026, 1, 24).unwrap(),
                        reason: Some("PTO".to_string()),
                    },
                ],
            },
            github: GitHubConfig {
                enterprise_url: "https://github.mycompany.com".to_string(),
                organisations: vec![
                    Organisation {
                        name: "my-org".to_string(),
                        repo_pattern: "^project-.*".to_string(),
                    },
                ],
                projects: vec![
                    GitHubProject {
                        org: "my-org".to_string(),
                        project_number: 1,
                        name: "Project Board".to_string(),
                    },
                ],
                distractions: vec![
                    Distraction {
                        org: "my-org".to_string(),
                        repo: "legacy-system".to_string(),
                        label: Some("unplanned".to_string()),
                        name: "Legacy Support".to_string(),
                    },
                ],
            },
            deliverables: Deliverables {
                documents: vec![
                    Document {
                        name: "Software Requirements Specification".to_string(),
                        id: "SRS-001".to_string(),
                        due_date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
                        status_label: Some("doc:srs".to_string()),
                        depends_on: vec![],
                    },
                    Document {
                        name: "Software Design Document".to_string(),
                        id: "SDD-001".to_string(),
                        due_date: NaiveDate::from_ymd_opt(2025, 11, 15).unwrap(),
                        status_label: Some("doc:sdd".to_string()),
                        depends_on: vec!["SRS-001".to_string()],
                    },
                ],
                csci: vec![
                    Csci {
                        name: "Flight Control Unit".to_string(),
                        id: "CSCI-FCU".to_string(),
                        target_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
                        repos: vec!["my-org/project-fcu".to_string()],
                        tier1_label: "integration-ready".to_string(),
                        tier2_label: "hil-passed".to_string(),
                    },
                ],
                demonstrations: vec![
                    Demonstration {
                        name: "Preliminary Design Review".to_string(),
                        id: "DEMO-PDR".to_string(),
                        start_date: NaiveDate::from_ymd_opt(2025, 10, 14).unwrap(),
                        end_date: NaiveDate::from_ymd_opt(2025, 10, 16).unwrap(),
                        description: Some("PDR demonstration milestone".to_string()),
                    },
                ],
            },
            dependencies: Dependencies {
                external: vec![
                    ExternalDependency {
                        name: "Interface Control Document".to_string(),
                        id: "ICD-001".to_string(),
                        owner: "External Team".to_string(),
                        rc_due: NaiveDate::from_ymd_opt(2025, 8, 1).unwrap(),
                        final_due: NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                        tracking_issue: Some("my-org/project-integration#42".to_string()),
                    },
                ],
            },
            sizing: Sizing::default(),
        }
    }

    /// Load configuration from a TOML file.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file.
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Generate JSON schema for editor autocomplete.
    pub fn json_schema() -> schemars::schema::RootSchema {
        schemars::schema_for!(ProjectConfig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_serializes() {
        let stub = ProjectConfig::stub();
        let toml = toml::to_string_pretty(&stub).unwrap();
        assert!(toml.contains("Project Name"));
    }

    #[test]
    fn test_roundtrip() {
        let stub = ProjectConfig::stub();
        let toml = toml::to_string_pretty(&stub).unwrap();
        let parsed: ProjectConfig = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.project.name, stub.project.name);
    }

    #[test]
    fn test_schema_generation() {
        let schema = ProjectConfig::json_schema();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("ProjectConfig"));
    }
}
