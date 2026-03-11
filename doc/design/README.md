# Design

*Software Design Description (SDD) for <system name>*

This directory contains the design specification — the authoritative record of **how** the system accomplishes its requirements.

## System Overview

<Brief description of the system: what it is, what it does, and its operational context.>

## Document Description

<Brief overview of what this document covers and how it is organized.>

## Purpose

Each design unit document describes a cohesive piece of the system: its purpose, the requirements it satisfies, the interfaces it provides and consumes, and its internal behavior. Design units map directly to implementation — each links to source files and test files, enabling full traceability from requirement through design to code.

A design unit might be a hardware module, a source file, a library, or a logical grouping of related code.

## Conventions

- **Naming:** `unit_NNN_<name>.md` — 3-digit zero-padded number, lowercase, underscores
- **Child units:** `unit_NNN.NN_<name>.md` — dot-notation encodes parent (e.g., `unit_002.01_pid_controller.md`)
- **Create new:** `.syskit/scripts/new-unit.sh <name>` or `.syskit/scripts/new-unit.sh --parent UNIT-NNN <name>`
- **Cross-references:** Use `UNIT-NNN` or `UNIT-NNN.NN` identifiers (derived from filename)
- **Traceability:** Source files link back via `Spec-ref` comments; use `impl-stamp.sh` to keep hashes current

## Framework Documents

- **concept_of_execution.md** — System runtime behavior, startup, data flow, and event handling
- **design_decisions.md** — Architecture Decision Records (ADR format)

## Table of Contents

<!-- TOC-START -->
- [Design Decisions](design_decisions.md)
- [Concept of Execution](concept_of_execution.md)
<!-- TOC-END -->
