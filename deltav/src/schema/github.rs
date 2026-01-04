//! GitHub Enterprise configuration schema.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// GitHub Enterprise configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GitHubConfig {
    /// Base URL of the GitHub Enterprise instance.
    ///
    /// Example: `https://github.mycompany.com`
    pub enterprise_url: String,

    /// Organizations and repository filters.
    #[serde(default)]
    pub organisations: Vec<Organisation>,

    /// GitHub Projects (project boards) to track.
    #[serde(default)]
    pub projects: Vec<GitHubProject>,

    /// Non-project work that consumes team capacity.
    #[serde(default)]
    pub distractions: Vec<Distraction>,

    /// Repositories to track for deliveries (releases/tags).
    ///
    /// Releases from these repositories will appear in the weekly
    /// "Deliveries" section of the report.
    #[serde(default)]
    pub delivery_repos: Vec<DeliveryRepo>,
}

/// A repository to track for deliveries via releases/tags.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeliveryRepo {
    /// Organization containing the repository.
    pub org: String,

    /// Repository name.
    pub repo: String,

    /// Human-readable name for the deliverable.
    ///
    /// If not provided, uses the repository name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl DeliveryRepo {
    /// Get the full repository path (org/repo).
    pub fn full_repo(&self) -> String {
        format!("{}/{}", self.org, self.repo)
    }

    /// Get the display name for this deliverable.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.repo)
    }
}

impl GitHubConfig {
    /// Get the API base URL for this instance.
    ///
    /// Handles both GitHub Enterprise (appends /api/v3) and public GitHub.com
    /// (uses api.github.com).
    pub fn api_url(&self) -> String {
        let base = self.enterprise_url.trim_end_matches('/');
        if base == "https://github.com" || base == "http://github.com" {
            "https://api.github.com".to_string()
        } else {
            format!("{}/api/v3", base)
        }
    }

    /// Check if this config points to public GitHub.com.
    pub fn is_public_github(&self) -> bool {
        let base = self.enterprise_url.trim_end_matches('/');
        base == "https://github.com" || base == "http://github.com"
    }

    /// Get the GraphQL API URL for this instance.
    pub fn graphql_url(&self) -> String {
        format!("{}/api/graphql", self.enterprise_url.trim_end_matches('/'))
    }

    /// Check if a repository matches any organisation filter.
    pub fn matches_repo(&self, org: &str, repo: &str) -> bool {
        self.organisations.iter().any(|o| o.matches(org, repo))
    }

    /// Get all organisation names.
    pub fn org_names(&self) -> Vec<&str> {
        self.organisations.iter().map(|o| o.name.as_str()).collect()
    }
}

/// An organization or user with repository filtering.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Organisation {
    /// Organization or user name (as it appears in GitHub URLs).
    pub name: String,

    /// Regex pattern to filter repositories.
    ///
    /// Only repositories whose names match this pattern will be included.
    /// Use `.*` to include all repositories.
    pub repo_pattern: String,

    /// Set to true if this is a GitHub user account rather than an organization.
    ///
    /// User accounts use a different API endpoint (`/users/{name}/repos`)
    /// than organizations (`/orgs/{name}/repos`).
    #[serde(default)]
    pub is_user: bool,
}

impl Organisation {
    /// Check if a repository matches this organisation's filter.
    pub fn matches(&self, org: &str, repo: &str) -> bool {
        if org != self.name {
            return false;
        }

        match regex::Regex::new(&self.repo_pattern) {
            Ok(re) => re.is_match(repo),
            Err(_) => false, // Invalid regex matches nothing
        }
    }

    /// Get all repos in this org (requires API call - this just validates the pattern).
    pub fn validate_pattern(&self) -> Result<(), regex::Error> {
        regex::Regex::new(&self.repo_pattern)?;
        Ok(())
    }
}

/// A GitHub Project (the project board feature) to track.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GitHubProject {
    /// Organization containing the project.
    pub org: String,

    /// Project number (from the URL).
    pub project_number: u32,

    /// Human-readable name for reporting.
    pub name: String,
}

impl GitHubProject {
    /// Get the URL for this project.
    pub fn url(&self, base_url: &str) -> String {
        format!(
            "{}/orgs/{}/projects/{}",
            base_url.trim_end_matches('/'),
            self.org,
            self.project_number
        )
    }
}

/// Non-project work that consumes team capacity.
///
/// Used to track "distractions" - work the team does that isn't
/// part of the main project but still consumes effort.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Distraction {
    /// Organization containing the repository.
    pub org: String,

    /// Repository name.
    pub repo: String,

    /// Optional label filter (only count issues with this label).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Human-readable name for reporting.
    pub name: String,
}

impl Distraction {
    /// Get the full repository path (org/repo).
    pub fn full_repo(&self) -> String {
        format!("{}/{}", self.org, self.repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_url() {
        let config = GitHubConfig {
            enterprise_url: "https://github.mycompany.com".to_string(),
            organisations: vec![],
            projects: vec![],
            distractions: vec![],
            delivery_repos: vec![],
        };
        assert_eq!(config.api_url(), "https://github.mycompany.com/api/v3");
    }

    #[test]
    fn test_api_url_trailing_slash() {
        let config = GitHubConfig {
            enterprise_url: "https://github.mycompany.com/".to_string(),
            organisations: vec![],
            projects: vec![],
            distractions: vec![],
            delivery_repos: vec![],
        };
        assert_eq!(config.api_url(), "https://github.mycompany.com/api/v3");
    }

    #[test]
    fn test_api_url_public_github() {
        let config = GitHubConfig {
            enterprise_url: "https://github.com".to_string(),
            organisations: vec![],
            projects: vec![],
            distractions: vec![],
            delivery_repos: vec![],
        };
        assert_eq!(config.api_url(), "https://api.github.com");
        assert!(config.is_public_github());
    }

    #[test]
    fn test_api_url_public_github_trailing_slash() {
        let config = GitHubConfig {
            enterprise_url: "https://github.com/".to_string(),
            organisations: vec![],
            projects: vec![],
            distractions: vec![],
            delivery_repos: vec![],
        };
        assert_eq!(config.api_url(), "https://api.github.com");
        assert!(config.is_public_github());
    }

    #[test]
    fn test_org_matches() {
        let org = Organisation {
            name: "my-org".to_string(),
            repo_pattern: "^project-.*".to_string(),
            is_user: false,
        };
        assert!(org.matches("my-org", "project-foo"));
        assert!(org.matches("my-org", "project-bar-baz"));
        assert!(!org.matches("my-org", "other-repo"));
        assert!(!org.matches("other-org", "project-foo"));
    }

    #[test]
    fn test_org_matches_all() {
        let org = Organisation {
            name: "my-org".to_string(),
            repo_pattern: ".*".to_string(),
            is_user: false,
        };
        assert!(org.matches("my-org", "anything"));
        assert!(org.matches("my-org", ""));
    }

    #[test]
    fn test_project_url() {
        let project = GitHubProject {
            org: "my-org".to_string(),
            project_number: 42,
            name: "Test Project".to_string(),
        };
        assert_eq!(
            project.url("https://github.mycompany.com"),
            "https://github.mycompany.com/orgs/my-org/projects/42"
        );
    }
}
