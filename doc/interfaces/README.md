# Interfaces

*Interface Design Description (IDD) for <system name>*

This directory contains the interface specifications — the authoritative record of **contracts** between components and with external systems.

## System Overview

<Brief description of the system: what it is, what it does, and its operational context.>

## Document Description

<Brief overview of what this document covers and how it is organized.>

## Purpose

Each interface document defines a precise contract: data formats, protocols, APIs, or signal definitions that components agree on. Interfaces are the bridge between requirements (what) and design (how), ensuring components can be developed and tested independently.

Interface types:

- **Internal** — Defined by this project (register maps, packet formats, internal APIs)
- **External Standard** — Defined by an external spec (PNG, SPI, USB)
- **External Service** — Defined by an external service (REST API, cloud endpoint)

## Conventions

- **Naming:** `int_NNN_<name>.md` — 3-digit zero-padded number, lowercase, underscores
- **Child interfaces:** `int_NNN.NN_<name>.md` — dot-notation encodes parent (e.g., `int_005.01_uart_registers.md`)
- **Create new:** `.syskit/scripts/new-int.sh <name>` or `.syskit/scripts/new-int.sh --parent INT-NNN <name>`
- **Cross-references:** Use `INT-NNN` or `INT-NNN.NN` identifiers (derived from filename)

## Table of Contents

<!-- TOC-START -->
*No documents yet.*
<!-- TOC-END -->
