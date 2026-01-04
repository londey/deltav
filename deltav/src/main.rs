//! deltav - DevSecOps metrics aggregator for GitHub Enterprise
//!
//! Tracks delivery metrics for systems engineering projects using data from
//! GitHub Enterprise. Generates weekly reports showing team velocity, CSCI
//! completion, and external dependency status.

#![deny(unsafe_code)]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::all))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::pedantic))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(missing_docs))]
// Allow some pedantic lints that are too strict for this project
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(unused_imports)]
#![allow(dead_code)]

mod cli;
mod github;
mod report;
mod schema;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command, OutputFormat};
use std::io::Write;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { output } => cmd_init(output),
        Command::Schema { output } => cmd_schema(output),
        Command::Validate { config } => cmd_validate(config),
        Command::Report {
            config,
            week,
            format,
            output,
            token,
        } => cmd_report(config, week, format, output, token),
    }
}

/// Generate a stub project.toml with example values.
///
/// Creates a template configuration file with placeholder values that users
/// can edit to match their project structure. Always writes directly to a file
/// to avoid encoding issues on Windows when piping stdout.
///
/// # Arguments
///
/// * `output` - File path to write to (defaults to "project.toml").
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
fn cmd_init(output: std::path::PathBuf) -> Result<()> {
    let stub = schema::ProjectConfig::stub();
    let toml = toml::to_string_pretty(&stub).context("Failed to serialize stub config")?;

    // Add helpful comments
    let commented = add_stub_comments(&toml);

    std::fs::write(&output, &commented)
        .with_context(|| format!("Failed to write to {}", output.display()))?;
    eprintln!("Created {}", output.display());

    Ok(())
}

/// Add helpful comments to the stub TOML.
///
/// Prepends a header comment with usage instructions to the generated TOML.
///
/// # Arguments
///
/// * `toml` - The TOML content to prepend comments to.
///
/// # Returns
///
/// The TOML content with a header comment block prepended.
fn add_stub_comments(toml: &str) -> String {
    let header = r#"# deltav Project Configuration
# =============================
#
# This file defines your project for metrics tracking.
# Edit the values below to match your project structure.
#
# For JSON schema (editor autocomplete): deltav schema > project.schema.json
#

"#;

    let mut out = String::from(header);
    out.push_str(toml);
    out
}

/// Output JSON schema for project.toml.
///
/// Generates a JSON schema that can be used by editors for autocomplete
/// and validation of project.toml files.
///
/// # Arguments
///
/// * `output` - Optional file path. If `None`, writes to stdout.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if the schema cannot be serialized or the file cannot be written.
fn cmd_schema(output: Option<std::path::PathBuf>) -> Result<()> {
    let schema = schema::ProjectConfig::json_schema();
    let json = serde_json::to_string_pretty(&schema).context("Failed to serialize schema")?;

    if let Some(path) = output {
        std::fs::write(&path, &json)
            .with_context(|| format!("Failed to write to {}", path.display()))?;
        eprintln!("Created {}", path.display());
    } else {
        println!("{}", json);
    }

    Ok(())
}

/// Validate a project.toml file.
///
/// Performs comprehensive validation of the configuration file including:
/// - TOML syntax and structure
/// - Regex pattern validity
/// - Date range consistency
/// - Backlog completeness bounds
/// - Duplicate deliverable ID detection
///
/// # Arguments
///
/// * `config` - Path to the project.toml file to validate.
///
/// # Returns
///
/// `Ok(())` if validation passes.
///
/// # Errors
///
/// Returns an error describing the first validation failure encountered.
fn cmd_validate(config: std::path::PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(&config)
        .with_context(|| format!("Failed to read {}", config.display()))?;

    let parsed: schema::ProjectConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", config.display()))?;

    // Validate regex patterns
    for org in &parsed.github.organisations {
        org.validate_pattern()
            .with_context(|| format!("Invalid repo_pattern for org '{}'", org.name))?;
    }

    // Validate date ranges
    if parsed.project.end_date <= parsed.project.start_date {
        anyhow::bail!("Project end_date must be after start_date");
    }

    // Validate backlog completeness
    if parsed.project.backlog_completeness <= 0.0 || parsed.project.backlog_completeness > 1.0 {
        anyhow::bail!("backlog_completeness must be between 0.0 (exclusive) and 1.0 (inclusive)");
    }

    // Check for duplicate IDs
    let mut ids = std::collections::HashSet::new();
    for id in parsed.deliverables.all_ids() {
        if !ids.insert(id) {
            anyhow::bail!("Duplicate deliverable ID: {}", id);
        }
    }

    eprintln!("✓ {} is valid", config.display());
    eprintln!("  Project: {}", parsed.project.name);
    eprintln!(
        "  Duration: {} to {} ({} days)",
        parsed.project.start_date,
        parsed.project.end_date,
        parsed.project.duration_days()
    );
    eprintln!("  Team: {} members", parsed.team.headcount());
    eprintln!("  Organisations: {}", parsed.github.org_names().join(", "));
    eprintln!("  CSCIs: {}", parsed.deliverables.csci.len());
    eprintln!("  Documents: {}", parsed.deliverables.documents.len());
    eprintln!(
        "  External dependencies: {}",
        parsed.dependencies.external.len()
    );

    Ok(())
}

/// Generate a weekly report.
///
/// Fetches data from GitHub and generates a report for the specified week.
/// Supports multiple output formats (Markdown, HTML, PDF).
///
/// # Arguments
///
/// * `config_path` - Path to the project.toml configuration file.
/// * `week` - ISO week string (e.g., "2026-W02"). Uses current week if `None`.
/// * `format` - Output format (Markdown, HTML, PDF, or All).
/// * `output` - Output path. If `None`, writes to stdout (except for "All" format).
/// * `token` - GitHub token. If `None`, resolves from env or gh CLI.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration cannot be loaded
/// - GitHub authentication fails
/// - API requests fail
/// - Output cannot be written
fn cmd_report(
    config_path: std::path::PathBuf,
    week: Option<String>,
    format: OutputFormat,
    output: Option<std::path::PathBuf>,
    token: Option<String>,
) -> Result<()> {
    // Load and validate config
    let config = schema::ProjectConfig::load(&config_path)
        .with_context(|| format!("Failed to load {}", config_path.display()))?;

    // Parse week or use current
    let (year, week_num) = if let Some(w) = week {
        cli::parse_iso_week(&w)?
    } else {
        cli::current_iso_week()
    };

    let week_str = cli::format_iso_week(year, week_num);
    eprintln!("Generating report for {}", week_str);

    // Calculate week boundaries
    let week_start = iso_week_to_date(year, week_num)?;
    let week_end = week_start + chrono::Duration::days(6);

    // Resolve GitHub token
    let hostname = github::extract_hostname(&config.github.enterprise_url);
    let resolved_token = github::resolve_token(token.as_deref(), hostname.as_deref())?;

    // Create GitHub client
    let client = github::GitHubClient::new(config.github.clone(), &resolved_token)?;

    // Test connection
    let user = client
        .test_connection()
        .context("Failed to connect to GitHub")?;
    eprintln!("Authenticated as: {}", user.login);

    // Fetch data and generate report
    let report_data =
        generate_report_from_github(&client, &config, &week_str, week_start, week_end)?;

    // Render based on format
    match format {
        OutputFormat::Markdown => {
            let md = report::markdown::render(&report_data);
            write_output(&md, output.as_ref(), "md")?;
        }
        OutputFormat::Html => {
            let html = report::html::render(&report_data);
            write_output(&html, output.as_ref(), "html")?;
        }
        OutputFormat::Pdf => {
            // Generate HTML first, then convert to PDF
            // For now, just output HTML with a note
            let html = report::html::render(&report_data);
            eprintln!("Note: PDF generation requires additional setup. Generating HTML instead.");
            write_output(&html, output.as_ref(), "html")?;
        }
        OutputFormat::All => {
            let base_path = output.unwrap_or_else(|| std::path::PathBuf::from("."));
            let base_name = format!("report-{}", week_str);

            let md = report::markdown::render(&report_data);
            let md_path = base_path.join(format!("{}.md", base_name));
            std::fs::write(&md_path, &md)?;
            eprintln!("Created {}", md_path.display());

            let html = report::html::render(&report_data);
            let html_path = base_path.join(format!("{}.html", base_name));
            std::fs::write(&html_path, &html)?;
            eprintln!("Created {}", html_path.display());
        }
    }

    Ok(())
}

/// Write output to file or stdout.
///
/// If a directory path is provided, creates a file named "report.{ext}" in that directory.
///
/// # Arguments
///
/// * `content` - The content to write.
/// * `output` - Optional output path (file or directory). Writes to stdout if `None`.
/// * `ext` - File extension to use when output is a directory.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
fn write_output(content: &str, output: Option<&std::path::PathBuf>, ext: &str) -> Result<()> {
    if let Some(path) = output {
        let final_path = if path.is_dir() {
            path.join(format!("report.{}", ext))
        } else {
            path.clone()
        };
        std::fs::write(&final_path, content)?;
        eprintln!("Created {}", final_path.display());
    } else {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(content.as_bytes())?;
    }
    Ok(())
}

/// Convert ISO week to Monday date.
///
/// Calculates the date of the Monday for the given ISO week number.
/// Uses the ISO 8601 week numbering system where week 1 contains January 4.
///
/// # Arguments
///
/// * `year` - ISO year (may differ from calendar year at year boundaries).
/// * `week` - ISO week number (1-52 or 1-53).
///
/// # Returns
///
/// The date of Monday for the specified week.
///
/// # Errors
///
/// Returns an error if the year is invalid.
fn iso_week_to_date(year: i32, week: u32) -> Result<chrono::NaiveDate> {
    use chrono::{Datelike, NaiveDate};

    // Find Jan 4 of the year (always in week 1)
    let jan4 = NaiveDate::from_ymd_opt(year, 1, 4).context("Invalid year")?;

    // Find Monday of week 1
    let week1_monday = jan4 - chrono::Duration::days(jan4.weekday().num_days_from_monday() as i64);

    // Add weeks
    let target = week1_monday + chrono::Duration::weeks((week - 1) as i64);

    Ok(target)
}

/// Generate a report by fetching data from GitHub.
///
/// Fetches issues from all configured organizations and repositories,
/// calculates metrics, and builds the complete report data structure.
///
/// # Arguments
///
/// * `client` - Authenticated GitHub API client.
/// * `config` - Project configuration.
/// * `week_str` - ISO week string for the report header (e.g., "2026-W02").
/// * `week_start` - First day (Monday) of the reporting week.
/// * `week_end` - Last day (Sunday) of the reporting week.
///
/// # Returns
///
/// Complete report data ready for rendering.
///
/// # Errors
///
/// Returns an error if GitHub API requests fail.
fn generate_report_from_github(
    client: &github::GitHubClient,
    config: &schema::ProjectConfig,
    week_str: &str,
    week_start: chrono::NaiveDate,
    week_end: chrono::NaiveDate,
) -> Result<report::ReportData> {
    use chrono::{Duration, TimeZone, Utc};
    use report::data::*;

    let as_of = week_end;

    // Calculate previous weeks for historical velocity (4-week rolling average)
    const VELOCITY_WEEKS: i64 = 4;
    let historical_start = week_start - Duration::weeks(VELOCITY_WEEKS);

    // Convert to UTC timestamps for API queries
    let historical_start_utc =
        Utc.from_utc_datetime(&historical_start.and_hms_opt(0, 0, 0).unwrap());
    let week_start_utc = Utc.from_utc_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap());

    // Previous week boundaries for comparison
    let prev_week_start = week_start - Duration::weeks(1);
    let prev_week_end = week_end - Duration::weeks(1);

    // Fetch issues from all configured organizations (including historical data)
    let mut all_issues: Vec<github::Issue> = Vec::new();
    let mut repos_fetched = 0;

    for org in &config.github.organisations {
        eprintln!("Fetching repositories for org: {}", org.name);
        let repos = client.fetch_repos(&org.name)?;
        eprintln!("  Found {} matching repositories", repos.len());

        for repo in &repos {
            eprintln!("  Fetching issues from {}/{}", org.name, repo.name);
            // Fetch issues updated since historical start (to get velocity history)
            let issues = client.fetch_issues(
                &org.name,
                &repo.name,
                Some(historical_start_utc),
                github::IssueState::All,
            )?;
            eprintln!("    Found {} issues", issues.len());
            all_issues.extend(issues);
            repos_fetched += 1;
        }
    }

    eprintln!(
        "Total: {} issues from {} repositories",
        all_issues.len(),
        repos_fetched
    );

    // Calculate ticket metrics for current week
    let (closed_this_week, opened_this_week, points_delivered) =
        calculate_ticket_metrics(&all_issues, week_start, week_end, &config.sizing);

    // Calculate previous week metrics for comparison
    let (closed_prev_week, _, _) =
        calculate_ticket_metrics(&all_issues, prev_week_start, prev_week_end, &config.sizing);

    // Calculate rolling velocity average (4-week)
    let velocity_avg = calculate_rolling_velocity(
        &all_issues,
        week_end,
        VELOCITY_WEEKS as usize,
        &config.sizing,
    );

    // Calculate expected velocity based on capacity ratio
    let nominal_capacity = config.team.nominal_capacity();
    let actual_capacity = config.team.average_capacity(week_start, week_end);
    let capacity_ratio = if nominal_capacity > 0.0 {
        actual_capacity / nominal_capacity
    } else {
        1.0
    };
    // Expected velocity = rolling average adjusted for capacity
    let expected_velocity = (velocity_avg * capacity_ratio).round() as u32;

    // Find blocked issues (issues with "blocked" label)
    let blocked_tickets = find_blocked_tickets(&all_issues);

    // Count open issues for backlog
    let open_issues = all_issues.iter().filter(|i| i.is_open()).count() as u32;

    // Fetch deliveries from releases
    let deliveries = fetch_deliveries(client, &config.github.delivery_repos, week_start, week_end)?;

    // Build meta
    let meta = ReportMeta {
        project_name: config.project.name.clone(),
        week: week_str.to_string(),
        week_start,
        week_end,
        generated_at: chrono::Utc::now(),
    };

    // Build weekly summary
    let weekly = WeeklySummary {
        deliveries,
        tickets: TicketSummary {
            closed: closed_this_week,
            closed_prev: closed_prev_week,
            opened: opened_this_week,
            net: closed_this_week as i32 - opened_this_week as i32,
            points_delivered,
            velocity_avg,
        },
        backlog: BacklogChange {
            start: open_issues + closed_this_week, // Estimate
            end: open_issues,
            new_work: opened_this_week,
            new_work_note: None,
        },
        capacity: CapacitySummary {
            nominal: nominal_capacity,
            actual: actual_capacity,
            leave: config
                .team
                .leave_in_range(week_start, week_end)
                .iter()
                .filter_map(|l| {
                    config.team.member_by_github(&l.github).map(|m| LeaveEntry {
                        name: m.name.clone(),
                        capacity_percent: 0,
                        reason: l.reason.clone(),
                    })
                })
                .collect(),
            expected_velocity,
            actual_velocity: points_delivered,
        },
        blocked: blocked_tickets,
        distractions: fetch_distractions(client, config, week_start_utc)?,
    };

    // Build project status
    let project = ProjectStatus {
        timeline: ProjectTimeline {
            days_elapsed: config.project.days_elapsed(as_of),
            total_days: config.project.duration_days(),
            percent_elapsed: config.project.percent_elapsed(as_of),
        },
        cscis: calculate_csci_status(
            &all_issues,
            &config.deliverables.csci,
            as_of,
            config.project.backlog_completeness,
        ),
        dependencies: fetch_dependency_status(client, &config.dependencies.external, as_of),
        documents: calculate_document_status(&all_issues, &config.deliverables.documents, as_of),
        milestones: config
            .deliverables
            .upcoming_milestones(as_of, 90)
            .iter()
            .map(|m| MilestoneStatus {
                id: m.id.clone(),
                name: m.name.clone(),
                date: m.date,
                days_until: m.days_until(as_of),
            })
            .collect(),
    };

    Ok(ReportData {
        meta,
        weekly,
        project,
    })
}

/// Calculate ticket metrics for the week.
///
/// Counts issues opened and closed within the week, and sums points
/// for closed issues based on their size labels.
///
/// # Arguments
///
/// * `issues` - All issues fetched from GitHub (may include issues outside the week).
/// * `week_start` - First day (Monday) of the reporting week.
/// * `week_end` - Last day (Sunday) of the reporting week.
/// * `sizing` - T-shirt sizing configuration for point calculations.
///
/// # Returns
///
/// A tuple of (closed_count, opened_count, points_delivered).
fn calculate_ticket_metrics(
    issues: &[github::Issue],
    week_start: chrono::NaiveDate,
    week_end: chrono::NaiveDate,
    sizing: &schema::Sizing,
) -> (u32, u32, u32) {
    let mut closed = 0u32;
    let mut opened = 0u32;
    let mut points = 0u32;

    for issue in issues {
        let created_date = issue.created_at.date_naive();
        let closed_date = issue.closed_at.map(|d| d.date_naive());

        // Count opened this week
        if created_date >= week_start && created_date <= week_end {
            opened += 1;
        }

        // Count closed this week and sum points
        let closed_in_range = closed_date
            .filter(|&d| d >= week_start && d <= week_end)
            .is_some();

        if closed_in_range {
            closed += 1;

            // Get points from size label
            let issue_points = issue
                .size_label()
                .and_then(|label| sizing.points_for(label))
                .unwrap_or(0);
            points += issue_points;
        }
    }

    (closed, opened, points)
}

/// Calculate rolling velocity average over N weeks.
///
/// Calculates the average story points delivered per week over the specified
/// number of weeks ending on the given date.
///
/// # Arguments
///
/// * `issues` - All issues to analyze.
/// * `end_date` - The end date of the most recent week.
/// * `num_weeks` - Number of weeks to include in the average.
/// * `sizing` - T-shirt sizing configuration for point calculations.
///
/// # Returns
///
/// The average points delivered per week. Returns 0.0 if no weeks have data.
fn calculate_rolling_velocity(
    issues: &[github::Issue],
    end_date: chrono::NaiveDate,
    num_weeks: usize,
    sizing: &schema::Sizing,
) -> f64 {
    use chrono::Duration;

    if num_weeks == 0 {
        return 0.0;
    }

    let mut total_points = 0u32;
    let mut weeks_with_data = 0usize;

    for week_offset in 0..num_weeks {
        let week_end = end_date - Duration::weeks(week_offset as i64);
        let week_start = week_end - Duration::days(6);

        let (_, _, points) = calculate_ticket_metrics(issues, week_start, week_end, sizing);

        // Only count weeks that have any closed issues
        if points > 0 {
            total_points += points;
            weeks_with_data += 1;
        }
    }

    if weeks_with_data > 0 {
        total_points as f64 / weeks_with_data as f64
    } else {
        0.0
    }
}

/// Find blocked tickets (issues with "blocked" label).
///
/// Identifies open issues that have a label starting with "blocked"
/// (e.g., "blocked", "blocked:external", "blocked-by-dependency").
///
/// # Arguments
///
/// * `issues` - All issues to search through.
///
/// # Returns
///
/// A vector of blocked ticket summaries for the report.
fn find_blocked_tickets(issues: &[github::Issue]) -> Vec<report::data::BlockedTicket> {
    issues
        .iter()
        .filter(|i| i.is_open() && i.has_label_prefix("blocked"))
        .map(|i| {
            let (org, repo) = i.org_repo().unwrap_or(("unknown", "unknown"));
            report::data::BlockedTicket {
                repo: format!("{}/{}", org, repo),
                number: i.number,
                title: i.title.clone(),
                blocked_on: i
                    .labels
                    .iter()
                    .find(|l| l.name.starts_with("blocked"))
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
            }
        })
        .collect()
}

/// Fetch deliveries from releases in configured delivery repositories.
///
/// Retrieves releases published during the reporting week from repositories
/// configured as delivery sources.
///
/// # Arguments
///
/// * `client` - Authenticated GitHub API client.
/// * `delivery_repos` - Delivery repository configurations.
/// * `week_start` - First day (Monday) of the reporting week.
/// * `week_end` - Last day (Sunday) of the reporting week.
///
/// # Returns
///
/// A vector of delivery items for releases published this week.
///
/// # Errors
///
/// Returns an error if GitHub API requests fail.
fn fetch_deliveries(
    client: &github::GitHubClient,
    delivery_repos: &[schema::DeliveryRepo],
    week_start: chrono::NaiveDate,
    week_end: chrono::NaiveDate,
) -> Result<Vec<report::data::DeliveryItem>> {
    use report::data::{DeliveryItem, DeliveryKind};

    let mut deliveries = Vec::new();

    for repo in delivery_repos {
        match client.fetch_releases(&repo.org, &repo.repo, Some(30)) {
            Ok(releases) => {
                for release in releases {
                    if release.published_in_range(week_start, week_end) {
                        deliveries.push(DeliveryItem {
                            id: release.tag_name.clone(),
                            name: format!(
                                "{} {}",
                                repo.display_name(),
                                release.display_name()
                            ),
                            kind: DeliveryKind::Release,
                            was_blocked: false,
                        });
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "  Warning: Could not fetch releases from {}/{}: {}",
                    repo.org, repo.repo, e
                );
            }
        }
    }

    Ok(deliveries)
}

/// Fetch distraction work from configured repositories.
///
/// Retrieves issues from configured "distraction" repositories that represent
/// non-project work (e.g., support tickets, maintenance, on-call work).
///
/// # Arguments
///
/// * `client` - Authenticated GitHub API client.
/// * `config` - Project configuration containing distraction repo settings.
/// * `since` - Only fetch issues updated after this timestamp.
///
/// # Returns
///
/// A vector of distraction summaries, one per configured distraction source.
///
/// # Errors
///
/// Returns an error if GitHub API requests fail.
fn fetch_distractions(
    client: &github::GitHubClient,
    config: &schema::ProjectConfig,
    since: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<report::data::DistractionSummary>> {
    let mut distractions = Vec::new();

    for distraction in &config.github.distractions {
        let issues = client.fetch_issues(
            &distraction.org,
            &distraction.repo,
            Some(since),
            github::IssueState::All,
        )?;

        // Filter by label if specified
        let filtered: Vec<_> = if let Some(ref label) = distraction.label {
            issues.into_iter().filter(|i| i.has_label(label)).collect()
        } else {
            issues
        };

        if !filtered.is_empty() {
            let assignees: Vec<String> = filtered
                .iter()
                .flat_map(|i| i.assignee_logins())
                .map(|s| s.to_string())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            distractions.push(report::data::DistractionSummary {
                name: distraction.name.clone(),
                ticket_count: filtered.len() as u32,
                estimated_hours: None,
                assignees,
            });
        }
    }

    Ok(distractions)
}

/// Fetch dependency status from GitHub tracking issues.
///
/// For dependencies with a `tracking_issue` configured, fetches the issue
/// and determines status based on labels:
/// - "rc-received" label → RC received
/// - "final-received" or closed issue → Final received
///
/// # Arguments
///
/// * `client` - Authenticated GitHub API client.
/// * `dependencies` - Dependency definitions from the project configuration.
/// * `as_of` - Reference date for calculating overdue status.
///
/// # Returns
///
/// A vector of dependency status entries for the report.
fn fetch_dependency_status(
    client: &github::GitHubClient,
    dependencies: &[schema::ExternalDependency],
    as_of: chrono::NaiveDate,
) -> Vec<report::data::DependencyStatus> {
    use report::data::{DependencyStatus, DependencyStatusKind};

    dependencies
        .iter()
        .map(|dep| {
            // Try to fetch tracking issue status
            let (rc_received, final_received) =
                if let Some((org, repo, number)) = dep.parse_tracking_issue() {
                    match client.fetch_issue(org, repo, number as u64) {
                        Ok(issue) => {
                            // Check for status labels
                            let rc = issue.has_label("rc-received")
                                || issue.has_label_prefix("rc-received");
                            let final_recv = issue.has_label("final-received")
                                || issue.has_label_prefix("final-received")
                                || issue.is_closed();
                            (rc || final_recv, final_recv)
                        }
                        Err(e) => {
                            eprintln!(
                                "  Warning: Could not fetch tracking issue for {}: {}",
                                dep.id, e
                            );
                            (false, false)
                        }
                    }
                } else {
                    (false, false)
                };

            // Calculate status based on received flags and due dates
            let status_kind = if final_received {
                DependencyStatusKind::Complete
            } else if rc_received {
                if dep.is_final_overdue(as_of) {
                    DependencyStatusKind::FinalOverdue
                } else {
                    DependencyStatusKind::RcReceived
                }
            } else if dep.is_rc_overdue(as_of) {
                DependencyStatusKind::RcOverdue
            } else {
                DependencyStatusKind::Pending
            };

            DependencyStatus {
                id: dep.id.clone(),
                name: dep.name.clone(),
                owner: dep.owner.clone(),
                rc_due: dep.rc_due,
                final_due: dep.final_due,
                rc_received,
                final_received,
                status: status_kind,
            }
        })
        .collect()
}

/// Fetch document status from GitHub issues with status labels.
///
/// For documents with a `status_label` configured, searches for issues
/// with that label and determines status:
/// - Any open issue with the label → In Progress
/// - Closed issue with the label → Complete
/// - No issues found → Not Started (or Overdue if past due date)
///
/// # Arguments
///
/// * `all_issues` - All issues fetched from GitHub.
/// * `documents` - Document definitions from the project configuration.
/// * `as_of` - Reference date for calculating overdue status.
///
/// # Returns
///
/// A vector of document status entries for the report.
fn calculate_document_status(
    all_issues: &[github::Issue],
    documents: &[schema::Document],
    as_of: chrono::NaiveDate,
) -> Vec<report::data::DocumentStatus> {
    use report::data::{DocumentStatus, DocumentStatusKind};

    documents
        .iter()
        .map(|doc| {
            // Find issues with the document's status label
            let doc_issues: Vec<_> = if let Some(ref label) = doc.status_label {
                all_issues.iter().filter(|i| i.has_label(label)).collect()
            } else {
                vec![]
            };

            let (status, completed_date) = if doc_issues.is_empty() {
                // No tracking issues found
                if doc.is_overdue(as_of) {
                    (DocumentStatusKind::Overdue, None)
                } else {
                    (DocumentStatusKind::NotStarted, None)
                }
            } else {
                // Check if all tracking issues are closed
                let all_closed = doc_issues.iter().all(|i| i.is_closed());
                let any_open = doc_issues.iter().any(|i| i.is_open());

                if all_closed {
                    // Find latest close date
                    let latest_close = doc_issues
                        .iter()
                        .filter_map(|i| i.closed_at)
                        .max()
                        .map(|dt| dt.date_naive());
                    (DocumentStatusKind::Complete, latest_close)
                } else if any_open {
                    (DocumentStatusKind::InProgress, None)
                } else if doc.is_overdue(as_of) {
                    (DocumentStatusKind::Overdue, None)
                } else {
                    (DocumentStatusKind::NotStarted, None)
                }
            };

            DocumentStatus {
                id: doc.id.clone(),
                name: doc.name.clone(),
                due_date: doc.due_date,
                status,
                completed_date,
                note: None,
            }
        })
        .collect()
}

/// Calculate CSCI status by matching issues to CSCIs.
///
/// For each CSCI, counts the issues in its associated repositories and
/// calculates completion percentage, adjusted by the backlog completeness factor.
///
/// # Arguments
///
/// * `issues` - All issues fetched from GitHub.
/// * `cscis` - CSCI definitions from the project configuration.
/// * `as_of` - Reference date for calculating days remaining.
/// * `backlog_completeness` - Factor (0.0-1.0) to adjust for undiscovered work.
///
/// # Returns
///
/// A vector of CSCI status entries for the report.
fn calculate_csci_status(
    issues: &[github::Issue],
    cscis: &[schema::Csci],
    as_of: chrono::NaiveDate,
    backlog_completeness: f64,
) -> Vec<report::data::CsciStatus> {
    use report::data::{CsciStatus, Projection};

    cscis
        .iter()
        .map(|csci| {
            // Count issues belonging to this CSCI's repositories
            let csci_issues: Vec<_> = issues
                .iter()
                .filter(|i| {
                    i.org_repo()
                        .map(|(org, repo)| csci.contains_repo(org, repo))
                        .unwrap_or(false)
                })
                .collect();

            let total = csci_issues.len() as u32;

            // Tier 1 = closed issues with the CSCI's tier1 label (integration-ready)
            // Tier 2 = closed issues with the CSCI's tier2 label (HIL-tested)
            // If no tier labels are found, fall back to counting closed issues
            let tier1_labeled = csci_issues
                .iter()
                .filter(|i| i.is_closed() && i.has_label(&csci.tier1_label))
                .count() as u32;
            let tier2 = csci_issues
                .iter()
                .filter(|i| i.is_closed() && i.has_label(&csci.tier2_label))
                .count() as u32;

            // If no tier1 labels found, use closed count as tier1 (simpler workflow)
            let closed_count = csci_issues.iter().filter(|i| i.is_closed()).count() as u32;
            let tier1 = if tier1_labeled > 0 {
                tier1_labeled
            } else {
                closed_count
            };

            // Adjusted completion (accounting for undiscovered work)
            let raw_completion = if total > 0 {
                (tier1 as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            let adjusted_completion = raw_completion * backlog_completeness;

            // Simple projection based on completion vs time elapsed
            let days_until = csci.days_until_target(as_of);
            let projection = if adjusted_completion >= 100.0 {
                Projection::Complete
            } else if days_until < 0 {
                Projection::Behind
            } else if adjusted_completion < 50.0 && days_until < 30 {
                Projection::AtRisk
            } else {
                Projection::OnTrack
            };

            CsciStatus {
                id: csci.id.clone(),
                name: csci.name.clone(),
                target_date: csci.target_date,
                days_until,
                total_tickets: total,
                tier1_complete: tier1,
                tier2_complete: tier2,
                completion_percent: adjusted_completion,
                projection,
                buffer_days: 0, // Would need velocity calculation
            }
        })
        .collect()
}
