# Technical Report: deltav Project Scope and Architecture

Date: 2026-03-11
Status: Draft

## Background

deltav originated as a CLI-based DevSecOps metrics report generator for GitHub
Enterprise environments. The project has a working CLI scaffold, configuration
schema, GitHub API client skeleton, and report renderers — but no live data
fetching or web interface.

This investigation was initiated to scope a **pivot** from CLI-only to a
**web-based dashboard application** served from a Docker container. The
motivation: GitHub's native project management, milestone, and ticketing UX is
insufficient for engineering program management. The user has prior experience
building effective dashboards in Confluence for JIRA and wants equivalent
capability against GitHub — without requiring any GitHub plugins, admin
cooperation, or third-party SaaS.

The system is intended for **single-user operation**, using the operator's own
GitHub credentials. The existing CLI is replaced by the web application.

## Scope

This report covers:

1. Technology selection and rationale
2. System architecture
3. Incremental delivery roadmap with user-facing milestones
4. Development and testing infrastructure (docker-compose with mock GitHub)

**Out of scope:** Detailed UI/UX design, multi-user/multi-tenant operation,
write-back to GitHub (noted as future work beyond this roadmap).

## Architecture

### System Overview

```
docker-compose.yml
├── deltav (Rust)          ← main application container
│   ├── Axum web server    ← serves dashboard UI
│   ├── Askama templates   ← server-side HTML rendering
│   ├── HTMX (14KB JS)    ← dynamic page updates without custom JS
│   ├── CodeMirror 6       ← in-browser Lua editor (widget workbench)
│   ├── mlua (Lua 5.4)    ← sandboxed scripting for widget logic
│   ├── SQLite cache       ← local copy of GitHub data
│   ├── plotters           ← server-side SVG chart generation
│   └── Octocrab           ← GitHub API client (REST + GraphQL)
│
├── mock-github (Python)   ← test fixture, separate container
│   └── Flask/FastAPI app serving GitHub REST/GraphQL API
│       └── reads from volume-mapped TOML folder structure
│
└── volumes
    ├── /data              ← persistent storage (SQLite DB, config)
    │   ├── deltav.db      ← SQLite cache (disposable, rebuilt from GitHub)
    │   ├── servers.toml   ← GitHub server connections
    │   └── dashboards/    ← dashboard definitions and Lua scripts
    └── /config            ← read-only mounted config
        ├── tokens/        ← PAT files (one per server)
        └── project.toml   ← project configuration
```

### Technology Choices

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Web framework | Axum (v0.8) | Most popular Rust web framework, Tokio-maintained, simplest mental model |
| Templates | Askama | Compile-time Jinja2-style templates, errors caught at build time |
| Interactivity | HTMX | 14KB JS file, server returns HTML fragments, no custom JS needed |
| Lua editor | CodeMirror 6 | Standard embeddable editor, Lua syntax highlighting + autocomplete |
| Scripting | mlua (Lua 5.4, vendored) | Production-grade, sandboxable, serde integration |
| GitHub client | Octocrab | GHES support, built-in GraphQL, PAT/App/OAuth auth |
| Cache | SQLite (rusqlite) | Single-file, zero-config, disposable, concurrent reads |
| Charts | plotters + plotters-svg | Pure Rust SVG generation, no JS dependency |
| Config | TOML (serde) | Already implemented, human-readable |
| Mock server | Python (FastAPI) | Quick to build, serves GitHub API from TOML file fixtures |

### Key Design Decisions

**Server-side rendering with HTMX:** All dashboard rendering happens on the
server. The browser receives HTML, not JSON. HTMX attributes on elements trigger
partial page updates via AJAX. This means zero custom JavaScript for the
dashboard itself — the only client-side JS is HTMX (14KB) and CodeMirror (for
the Lua editor).

**Hybrid REST + GraphQL:** GitHub Projects v2 has no REST API — GraphQL is
mandatory. Use GraphQL for bulk data fetching and Projects v2. Use REST with
ETags for efficient incremental polling (304 responses don't count against rate
limits).

**SQLite as disposable cache:** GitHub is the source of truth. The SQLite
database can be deleted and rebuilt at any time. This simplifies schema
migrations — just drop and re-sync.

**TOML + Lua hybrid configuration:** Dashboard layout and widget placement are
TOML. Widget data logic (queries, filters, calculations, thresholds) is Lua.
Lua scripts are editable in the browser via CodeMirror with live preview.

**Tokens as files, not config values:** Access tokens are mounted as separate
files in `/config/tokens/`, never embedded in TOML config. This prevents
accidental commits and works naturally with Docker secrets.

### Authentication

| Aspect | GHES (on-prem) | Enterprise github.com | Public github.com |
|--------|---------------|----------------------|-------------------|
| API base URL | `https://HOST/api/v3/` | `https://api.github.com/` | `https://api.github.com/` |
| GraphQL | `https://HOST/api/graphql` | `https://api.github.com/graphql` | `https://api.github.com/graphql` |
| SSO token auth | Not required | Required (authorize PAT per org) | Not required (unless org enforces) |

The auth layer accepts `(name, base_url, token_file)` tuples via `servers.toml`,
making it work identically across all GitHub variants. On GHES, a PAT just
works. On enterprise github.com, the user must SSO-authorize their PAT via the
GitHub web UI once per organisation — a one-time manual step.

### Caching and Sync Strategy

1. **Initial sync:** Full fetch of all issues/PRs using GraphQL bulk queries
   (~10-20 queries for 1000 repos)
2. **Background polling:** Every 15-30 minutes, use REST `?since=` to find
   changed issues. ETags on unchanged repos return 304 (free, doesn't count
   against rate limits)
3. **Manual refresh:** Button in the UI triggers immediate full re-sync
4. **GHES advantage:** Rate limits are disabled by default and admin-configurable

### Lua Sandboxing

```rust
// Only load safe standard libraries — io, os, package, debug never exist
let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH)?;
```

Lua scripts are pure functions: they receive query functions (provided by Rust)
and return widget descriptors. No file I/O, no network access, no shell
execution. Instruction count limits prevent infinite loops.

## Roadmap

Each milestone is a small, user-facing increment. The definition of done for
each includes any necessary SQLite schema changes and GitHub API integration
required to support the feature.

### Milestone 1: Docker-Compose Skeleton with Persistent Storage

**User sees:** `docker compose up` starts a container with a mounted volume.

**Details:**
- `docker-compose.yml` with the deltav service
- Volume mount at `/data` for persistent storage
- Volume mount at `/config` for read-only configuration
- Container starts, creates `/data` directory structure if missing, and exits
  cleanly
- Replaces the existing CLI entry point

**Definition of done:** `docker compose up` runs without error, volume persists
across restarts.

### Milestone 2: TOML Configuration

**User sees:** A `project.toml` file is read from the config volume on startup.

**Details:**
- Reuse existing `src/schema/` system for `project.toml` parsing and validation
- Add `servers.toml` schema for GitHub server connections:
  ```toml
  [[servers]]
  name = "My GHES"
  url = "https://github.mycompany.com"
  token_file = "/config/tokens/ghes.token"
  ```
- Log configuration summary on startup (project name, configured servers, etc.)
- Fail fast with clear errors if config is invalid

**Definition of done:** Container reads and validates both config files on
startup, logs summary.

### Milestone 3: Blank Web Page

**User sees:** Opening `http://localhost:8080` shows a page titled "deltav".

**Details:**
- Add Axum web server to the container
- Single Askama template: page title "deltav", empty content area
- HTMX included via `<script>` tag
- Basic CSS for consistent styling (will evolve with each milestone)
- Server starts on container launch, listens on configurable port

**Definition of done:** Browser shows "deltav" page. `curl localhost:8080`
returns HTML.

### Milestone 4: Server Settings Page

**User sees:** A settings page at `/settings` where they can add, remove, and
test GitHub server connections.

**Details:**
- Settings page lists configured servers from `servers.toml`
- Form to add a new server (name, URL, token file path)
- "Test Connection" button per server — calls GitHub's `/user` endpoint and
  reports success/failure (username, scopes) or error
- "Remove" button per server
- Changes persist to `servers.toml` on the mounted volume
- HTMX handles form submission and partial page updates (no full reload)
- Navigation: link between dashboard and settings pages

**Definition of done:** User can add a server, test its connection, see success,
and the config survives container restart.

### Milestone 5: Mock GitHub Server (No Data)

**User sees:** A second container in docker-compose that responds to GitHub API
endpoints with empty results.

**Details:**
- Python (FastAPI) container serving GitHub REST v3 API:
  - `GET /api/v3/user` → authenticated user info
  - `GET /api/v3/orgs` → empty list
  - `GET /api/v3/repos` → empty list
  - `GET /api/v3/rate_limit` → rate limit status
- Pre-configured in `docker-compose.yml` alongside deltav
- deltav's `servers.toml` points to the mock by default
- "Test Connection" from Milestone 4 succeeds against the mock

**Definition of done:** `docker compose up` starts both containers. Settings
page successfully tests connection to mock server.

### Milestone 6: Mock GitHub Server with Test Data

**User sees:** The mock server returns realistic issue/PR/repo data from a
folder structure.

**Details:**
- Volume-mapped directory structure for mock data:
  ```
  mock-data/
  ├── acme-corp/                          # organisation
  │   ├── firmware-core/                  # repository
  │   │   ├── issues/
  │   │   │   ├── 001.toml               # issue
  │   │   │   └── 002.toml
  │   │   └── pulls/
  │   │       └── 010.toml               # pull request
  │   ├── hardware-drivers/
  │   └── test-framework/
  ├── platform-team/
  │   ├── ci-pipelines/
  │   ├── deploy-tools/
  │   └── monitoring/
  └── research-lab/
      ├── prototype-alpha/
      └── data-analysis/
  ```
- ~8 repositories across 3 organisations
- Issues with labels (bug, feature, blocked, size:M, tier1, tier2), milestones,
  assignees, dates, open/closed states
- PRs with merge status, review state
- Mock serves standard GitHub REST endpoints with pagination
- SQLite schema created: `servers`, `repositories`, `issues`, `pull_requests`,
  `labels`, `milestones`
- Background sync worker fetches from mock and populates SQLite cache

**Definition of done:** deltav syncs data from mock server into SQLite. Data
visible via a debug/status page showing sync state and record counts.

### Milestone 7: Issue Search Widget (Table)

**User sees:** A dashboard page with a widget displaying issue search results as
a table.

**Details:**
- Dashboard page at `/dashboards/{id}` with a grid layout
- Dashboard definition in TOML:
  ```toml
  [dashboard]
  name = "Sprint Overview"
  columns = 2

  [[dashboard.widgets]]
  type = "search"
  title = "Open Bugs"
  position = { row = 1, col = 1 }
  script = "open_bugs.lua"
  ```
- Lua script defines the query and display:
  ```lua
  return widget.search({
    query = {
      state = "open",
      labels = {"bug"},
    },
    columns = {"repo", "number", "title", "assignee", "updated"},
    sort = {field = "updated", order = "desc"},
  })
  ```
- Table rendered server-side via Askama
- Queries run against SQLite cache
- Widget workbench mode: navigate to `/widgets/{id}/edit` to see the widget
  alone alongside a CodeMirror Lua editor with live preview
- Lua API: `widget.search({ query = {...}, columns = {...}, sort = {...} })`
- Sandboxed mlua execution with query functions injected by Rust

**Definition of done:** Dashboard shows a table of issues matching the Lua
query. User can edit the Lua in the workbench and see the table update.

### Milestone 8: Count Widget

**User sees:** A dashboard widget showing a single number with a label, based on
an issue search query.

**Details:**
- New widget type `counter`:
  ```lua
  return widget.counter({
    title = "Open Bugs",
    query = {
      state = "open",
      labels = {"bug"},
    },
    -- optional date range filter
    date_range = { from = "2026-01-01", to = "2026-03-11" },
  })
  ```
- Renders as a prominent number with title text
- Optional colour thresholds:
  ```lua
  return widget.counter({
    title = "Blocked Items",
    query = { labels = {"blocked"}, state = "open" },
    threshold = { green = {0, 2}, amber = {3, 5}, red = {6, math.huge} },
  })
  ```
- Colour changes based on count falling within threshold ranges
- Editable via the same widget workbench from Milestone 7

**Definition of done:** Counter widget renders on dashboard with correct count
and colour. Thresholds work. Editable in workbench.

### Milestone 9: Dial/Gauge Widget

**User sees:** A dashboard widget showing a dial/gauge visualisation based on
issue search queries.

**Details:**
- New widget type `dial` — a semicircular or circular gauge rendered as SVG
  via plotters:
  ```lua
  return widget.dial({
    title = "Integration Readiness",
    query = {
      labels = {"tier1"},
      state = "closed",
    },
    total_query = {
      labels = {"tier1"},
    },
    -- value = count(query) / count(total_query) * 100
    label = "% Complete",
    ranges = {
      red    = {0, 40},
      amber  = {40, 70},
      green  = {70, 100},
    },
  })
  ```
- SVG rendered server-side by plotters, embedded inline in HTML
- Needle/arc position computed from ratio of two queries
- Lua defines: title, queries, label text, range boundaries and colours
- Custom Lua calculation supported:
  ```lua
  local closed = #query.issues({ labels = {"tier1"}, state = "closed" })
  local total  = #query.issues({ labels = {"tier1"} })
  local risk   = #query.issues({ labels = {"blocked"} }) * 5

  return widget.dial({
    title = "Risk-Adjusted Progress",
    value = (closed / total * 100) - risk,
    label = "adjusted %",
    ranges = { red = {0, 30}, amber = {30, 60}, green = {60, 100} },
  })
  ```
- Editable via widget workbench

**Definition of done:** Dial renders as SVG with correct arc position and
colour. Custom Lua calculations work. Editable in workbench.

### Future Milestones (Not Yet Scoped)

These are anticipated but not yet detailed. Each would follow the same
incremental pattern:

- **Dashboard management:** Create/clone/delete dashboards. Clone from templates
  with name/milestone substitution. Multiple dashboards for tracking different
  project milestones.
- **Burndown chart widget:** SVG line chart showing remaining work over time.
  Requires historical data accumulation in SQLite.
- **Timeline widget:** Gantt-style visualisation of milestones and dependencies.
- **Print/PDF support:** CSS `@media print` rules for browser Print→PDF as the
  initial approach. Server-side PDF rendering (headless Chrome or equivalent) as
  a later upgrade for consistency.
- **GraphQL integration:** For GitHub Projects v2 boards, custom fields, and
  status columns. Required for GHES 3.8+.
- **Write-back:** Create/update issues, manage project board items from the
  dashboard. Freshness check before any mutation.
- **Device flow OAuth:** Alternative to PAT files for authentication. User
  visits a URL, enters a code, container receives a token.

## Findings

### F1: Axum + Askama + HTMX is the optimal web stack

Server-side rendering with zero custom JavaScript (except CodeMirror for the Lua
editor). Compile-time template checking. Lowest learning curve for a Rust
developer new to web development. The single-user constraint eliminates any
performance-based argument for WASM frameworks like Leptos or Dioxus.

### F2: GraphQL is mandatory for Projects v2

GitHub Projects v2 has no REST API. Any dashboard that displays project board
data must use GraphQL. This is deferred to a future milestone but architecturally
accounted for — Octocrab supports GraphQL natively.

### F3: A mock GitHub server enables test-driven development

A Python FastAPI container serving GitHub API responses from a TOML folder
structure allows realistic testing without creating hundreds of repos. The mock
is a separate container, so the main application connects to real GitHub without
modification — only `servers.toml` changes.

### F4: Lua-first widget logic enables rapid iteration

By defining widget queries and thresholds in Lua with an in-browser editor and
live preview, the user can iterate on dashboard content without recompiling or
restarting the container. The sandboxed mlua environment prevents any safety
concerns.

### F5: SQLite schema evolves with features

Each milestone that introduces a new data requirement (e.g., labels for search
widgets, milestones for timeline widgets) extends the SQLite schema
incrementally. Since the cache is disposable, schema changes just trigger a
re-sync.

### F6: ~40-50% of existing code is reusable

| Component | Reuse? | Notes |
|-----------|--------|-------|
| Schema system (`src/schema/`) | **Yes** | `project.toml` parsing, validation, all calculations |
| GitHub types (`src/github/types.rs`) | **Partial** | Useful as internal domain types alongside octocrab |
| GitHub client (`src/github/client.rs`) | **Replace** | Hand-rolled reqwest → octocrab |
| Report data (`src/report/data.rs`) | **Partial** | Domain models carry over for export functionality |
| Markdown renderer | **Yes** | Report export reuses this |
| HTML renderer | **Replace** | Static HTML → Askama templates |
| CLI (`src/cli.rs`, `src/main.rs`) | **Replace** | CLI → Axum web server |
| Build infrastructure | **Yes** | `build.sh`, Dockerfile adapt with additions |

### F7: Authentication is simpler than expected

A PAT file mounted into the container covers all GitHub variants. On GHES, it
just works. On enterprise github.com, one-time SSO authorization via the web UI.
The `servers.toml` + token-file pattern supports multiple simultaneous GitHub
instances cleanly.

## Conclusions

The roadmap delivers nine concrete milestones, each building on the last:

1. **Milestones 1-3** establish the container, configuration, and web server
   foundation.
2. **Milestones 4-6** connect to GitHub (real or mock) and populate the cache.
3. **Milestones 7-9** deliver the first three widget types with Lua-driven
   queries and an in-browser editor for rapid iteration.

Each milestone is independently testable against the mock GitHub server.
The mock's TOML folder structure makes test data transparent and version-
controllable.

**Remaining uncertainties:**
- mlua MSRV compatibility with Rust 1.75 (may require toolchain upgrade)
- Minimum GHES version at target deployment (Projects v2 needs 3.8+)
- Whether `plotters` SVG quality meets expectations for the dial widget
  (prototype during Milestone 9)
- Optimal CodeMirror configuration for Lua autocomplete of the deltav widget API

## Recommendations

1. **Prototype the dial SVG early** (even before Milestone 9) to validate
   `plotters` output quality. If unsatisfactory, Chart.js or ECharts are
   fallback options that work alongside HTMX.

2. **Design the Lua widget API carefully** during Milestone 7. The `query`,
   `widget.search`, `widget.counter`, and `widget.dial` functions form a
   contract that users will build scripts against. Getting this right early
   avoids breaking changes later.

3. **Keep the mock server minimal.** It only needs to serve the endpoints deltav
   actually calls. Expand it as new milestones require new API endpoints.

4. **Consider `tower-livereload`** for development — auto-refreshes the browser
   when Askama templates change, reducing the edit-compile-refresh cycle.

5. **Version the SQLite schema** even though it's disposable. A simple
   `schema_version` table lets deltav detect stale caches and auto-rebuild
   rather than failing with cryptic errors.

When ready to begin implementation, use this report as context for
`/syskit-impact` to start formal specification changes, beginning with
Milestone 1.
