# Architecture

*Architecture overview for deltav*

## System Description

The _deltav_ system provides team and project management reports and dashboards for a GitHub Enterprise environment. It runs as a Docker container that connects to a GitHub Enterprise server via the GitHub API and serves a web interface to operators.

The backend is implemented in Rust and serves a web interface built with the _leptos_ frontend framework. The system is stateless with respect to GitHub data — it fetches live data on demand — but maintains persistent state (configuration, generated reports, caches) in a mounted `/data` volume. Read-only project configuration is supplied via a `/config` volume mount.

## Deployment Model

deltav is deployed as a Docker container managed by `docker-compose`. Two volume mounts are required:

- `/data` — Persistent read-write storage for generated reports, caches, and runtime state. The container creates the required directory structure under `/data` on first startup if it does not already exist.
- `/config` — Read-only mount providing the `project.toml` configuration file(s).

See DD-001 in `doc/design/design_decisions.md` for the rationale behind the container-based execution model.

## Design Philosophy

- **Fetch, don't store:** GitHub is the source of truth for ticket data. The system fetches on demand rather than maintaining a local database of issue state.
- **Single configuration source:** `project.toml` (mounted at `/config`) is the authoritative definition of project structure, team, and reporting parameters.
- **Self-contained outputs:** Generated reports are standalone documents (inline CSS, base64 images) that can be shared without external dependencies.
- **Container-native:** The deployment unit is a Docker container. All runtime dependencies are bundled; operators do not install or configure the Rust toolchain.

## Component Interactions

The container exposes a web interface. On each report request, the web handler reads `project.toml` from `/config`, queries the GitHub Enterprise API for issue and PR data, computes delivery metrics, and renders a report. Rendered reports are written to `/data/reports/` for retrieval.

Design units (UNIT-NNN) and interfaces (INT-NNN) will be added to the block diagram below as components are formally specified.

---

<!-- syskit-arch-start -->
### Block Diagram

```mermaid
flowchart LR
    %% No design units found
```

### Software Units

*No design units found.*
<!-- syskit-arch-end -->
