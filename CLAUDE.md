# CLAUDE.md - deltav Development Guide

## Project Overview

deltav is a DevSecOps metrics aggregator for GitHub Enterprise, designed for systems engineering environments with hardware-in-the-loop (HIL) testing constraints. It generates weekly delivery performance reports from GitHub issues/PRs across multiple organizations.

## Key Rules

- `./build.sh --check` must pass after every change (cargo fmt, cargo check, cargo clippy).
- Minimize blast radius: only change code directly related to the current task. If you notice problems in other areas, mention them but don't fix them without approval.
- `ARCHITECTURE.md` is the authoritative high-level architecture document.
- All code follows its respective style guide:
  - Rust: `.claude/skills/claude-skill-rust/SKILL.md`


## Current State

The project scaffolding is complete with:
- CLI structure (clap-based)
- Schema definitions for `project.toml` configuration
- GitHub API client (skeleton)
- Report data structures
- Markdown and HTML renderers

**Not yet implemented:**
- Actual GitHub API data fetching (client exists but not wired up)
- PDF generation (placeholder only)
- Metrics calculations from real data
- Status tracking for dependencies/documents (currently placeholder)

## Build Notes

**Rust Version Constraint:** The development environment has Rust 1.75. Some dependencies (chrono, reqwest via idna/ICU) pull in crates requiring newer Rust. You may need to:
1. Pin dependencies to older versions
2. Use `cargo update <pkg> --precise <version>` to downgrade
3. Disable default features that pull in ICU (e.g., chrono's `iana-time-zone`)

Known working approach:
```toml
chrono = { version = "0.4.26", default-features = false, features = ["serde", "std", "clock"] }
reqwest = { version = "0.11.22", default-features = false, features = ["json", "blocking", "rustls-tls"] }
```

You may need to pin `idna` and `url` to pre-ICU versions if build fails.

## Architecture

```
src/
├── main.rs          # CLI entry point, command handlers
├── cli/mod.rs       # Clap definitions, ISO week parsing
├── schema/          # project.toml structure
│   ├── mod.rs       # ProjectConfig (root)
│   ├── project.rs   # Project metadata
│   ├── team.rs      # Team members, leave
│   ├── github.rs    # GitHub orgs, repos, projects
│   ├── deliverables.rs  # Documents, CSCIs, demos
│   ├── dependencies.rs  # External dependencies
│   └── sizing.rs    # T-shirt sizing definitions
├── github/          # GitHub API interaction
│   ├── mod.rs
│   ├── client.rs    # API client with pagination
│   └── types.rs     # API response types
└── report/          # Report generation
    ├── mod.rs
    ├── data.rs      # Report data structures
    ├── markdown.rs  # Markdown renderer
    └── html.rs      # HTML renderer
```

## Key Design Decisions

1. **No persistent storage** - Fetch from GitHub on each run. The `project.toml` is the single source of truth for project structure; GitHub is the source of truth for ticket data.

2. **Two-tier metrics** - Tier 1 (integration-ready) and Tier 2 (HIL-passed) to account for hardware testing bottlenecks.

3. **Backlog completeness adjustment** - All completion percentages are adjusted by the `backlog_completeness` estimate to account for undiscovered work.

4. **Self-contained outputs** - Markdown uses base64 data URLs for images; HTML has inline CSS; both viewable without external dependencies.

## CLI Commands

```bash
deltav init [--output FILE]           # Generate stub project.toml
deltav schema [--output FILE]         # Output JSON schema for editor autocomplete
deltav validate <config>              # Validate project.toml
deltav report [options]               # Generate weekly report
  --config, -c <FILE>                 # Path to project.toml (default: project.toml)
  --week, -w <YYYY-Www>               # ISO week (default: current)
  --format, -f <FORMAT>               # markdown|html|pdf|all
  --output, -o <PATH>                 # Output file or directory
  --token <TOKEN>                     # GitHub PAT (or GITHUB_TOKEN env)
```

## Next Steps (Priority Order)

1. **Fix build** - Resolve dependency version conflicts for Rust 1.75

2. **Wire up GitHub client** - Connect `cmd_report` to actually fetch data:
   - Fetch issues from configured repos
   - Filter by date range for the requested week
   - Calculate ticket counts, points, velocity

3. **Implement metrics calculations** in `main.rs::generate_sample_report`:
   - Count closed/opened issues per week
   - Extract size labels and sum points
   - Calculate rolling velocity average
   - Identify blocked issues (by label or linked issues)
   - Track distraction work from configured repos

4. **Add status tracking** - The current design assumes dependency/document status comes from `project.toml`, but we could:
   - Add a `status.toml` for point-in-time overrides
   - Or track via GitHub labels/milestones

5. **PDF generation** - Options:
   - Shell out to `wkhtmltopdf` or `weasyprint`
   - Use a Rust PDF library like `printpdf` or `genpdf`
   - Generate HTML and let user convert

6. **Charts/visualizations** - The HTML renderer has placeholders for progress bars. Could add:
   - Velocity trend sparklines (SVG)
   - Burndown/burnup charts
   - Dependency timeline

## Testing

Run tests with:
```bash
cargo test
```

Most modules have unit tests. Integration tests would require a GitHub instance.

## Configuration Schema

The JSON schema can be generated with `deltav schema` for editor autocomplete. Key sections:

- `[project]` - name, dates, backlog_completeness
- `[team]` - members (name, github, capacity), leave periods
- `[github]` - enterprise_url, organisations (with repo_pattern regex), projects, distractions
- `[deliverables]` - documents, csci, demonstrations
- `[dependencies]` - external dependencies with RC/final due dates
- `[sizing]` - T-shirt size point values (defaults provided)

## Code Style

- Use `anyhow` for error handling in application code
- Use `thiserror` for library-style error types
- Prefer `chrono::NaiveDate` for dates without timezone
- All public items should have doc comments
- Tests go in the same file as the code they test

## Common Tasks

### Adding a new field to project.toml

1. Add field to appropriate struct in `src/schema/`
2. Update `ProjectConfig::stub()` in `src/schema/mod.rs`
3. Add validation in `cmd_validate` if needed
4. Update report generation to use the field

### Adding a new report section

1. Add data structure in `src/report/data.rs`
2. Update `ReportBuilder` and `ReportData`
3. Add rendering in both `markdown.rs` and `html.rs`
4. Wire up data collection in `generate_sample_report`

### Adding a new CLI command

1. Add variant to `Command` enum in `src/cli/mod.rs`
2. Add handler function `cmd_*` in `src/main.rs`
3. Match in `main()`

<!-- syskit-start -->
## syskit

This project uses **syskit** for specification-driven development. Specifications in `doc/` define what the system must do, how components interact, and how the design is structured. Implementation follows from specs. When creating new specifications, define interfaces and requirements before design — understand the contracts and constraints before deciding how to build.

### Working with code

- Source files may contain `Spec-ref:` comments linking to design units — **preserve these; never edit the hash manually**.
- Before modifying code, check `doc/design/` for a relevant design unit (`unit_NNN_*.md`) that describes the component's intended behavior.
- After code changes, run `.syskit/scripts/impl-check.sh` to verify spec-to-implementation freshness.
- After spec changes, run `.syskit/scripts/impl-stamp.sh UNIT-NNN` to update Spec-ref hashes in source files.

### Documentation principle

- **Reference, don't reproduce.** Don't duplicate definitions, requirements, or design descriptions — reference the authoritative source instead. For project documents, reference by ID (`REQ-NNN`, `INT-NNN`, `UNIT-NNN`, `VER-NNN`). For external standards, reference by name, version/year, and section number (e.g., "IEEE 802.3-2022 §4.2.1", "RFC 9293 §3.1"). This applies to specification documents and code comments alike.

### Making changes

For non-trivial changes affecting system behavior, use the syskit workflow:

1. `/syskit-impact <change>` — Analyze what specifications are affected
2. `/syskit-propose` — Propose specification updates
3. `/syskit-refine --feedback "<issues>"` — Iterate on proposed changes based on review feedback (optional, repeatable)
4. `/syskit-approve` — Approve changes (works across sessions, enables overnight review)
5. `/syskit-plan` — Break into implementation tasks
6. `/syskit-implement` — Execute with traceability

New to syskit? Run `/syskit-guide` for an interactive walkthrough.

### Reference

- Specifications: `doc/requirements/`, `doc/interfaces/`, `doc/design/`, `doc/verification/`
- Working documents: `.syskit/analysis/`, `.syskit/tasks/`
- Scripts: `.syskit/scripts/`
- Full instructions: `.syskit/AGENTS.md` (read on demand, not auto-loaded)
<!-- syskit-end -->
