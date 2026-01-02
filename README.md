# deltav

DevSecOps metrics aggregator for GitHub Enterprise in systems engineering environments.

## Overview

deltav tracks delivery metrics for systems engineering projects using data from GitHub Enterprise. It generates weekly reports showing team velocity, CSCI completion, and external dependency status—adapted for environments where "deployment" means hardware-in-the-loop testing rather than continuous deployment.

## Features

- **Project configuration via TOML** with JSON schema for editor autocomplete
- **Tier 1/Tier 2 metrics** distinguishing "integration-ready" from "HIL-passed" work
- **External dependency tracking** with RC and final due dates
- **Capacity planning** with leave tracking and velocity adjustments
- **Multiple output formats**: Markdown (for LLM consumption), HTML (for humans), PDF (for reporting)

## Installation

```bash
cargo build --release
```

## Usage

### Generate a stub project configuration

```bash
deltav init > project.toml
```

### Generate JSON schema for editor autocomplete

```bash
deltav schema > project.schema.json
```

### Validate your configuration

```bash
deltav validate project.toml
```

### Generate a weekly report

```bash
# Current week, markdown to stdout
deltav report --config project.toml

# Specific week, HTML output
deltav report --config project.toml --week 2026-W02 --format html -o report.html

# All formats
deltav report --config project.toml --format all -o ./reports/
```

## Configuration

See `deltav init` for a complete example. Key sections:

- **[project]** — Name, dates, backlog completeness estimate
- **[team]** — Members, capacity allocations, planned leave
- **[github]** — Enterprise URL, organisations with repo filters, project boards
- **[deliverables]** — Documents, CSCIs (with tier labels), demonstrations
- **[dependencies]** — External dependencies with RC/final due dates
- **[sizing]** — T-shirt size point values

## Metrics Philosophy

Traditional DORA metrics assume continuous deployment. In systems engineering:

| DORA Metric | deltav Analog |
|-------------|---------------|
| Deployment Frequency | Release candidate frequency |
| Lead Time | Requirements → Integration-ready |
| Change Failure Rate | HIL test failures |
| MTTR | Time to resolve blocking DRs |

The **Tier 1/Tier 2** distinction separates team-controllable metrics (code ready for integration) from system-level metrics (HIL test results).

## Development Status

This is a working prototype. Current limitations:

- GitHub API integration is scaffolded but not wired up (reports show placeholder data)
- PDF generation requires external tooling (generates HTML instead)
- No persistent storage—re-fetches from GitHub each run

## License

MIT
