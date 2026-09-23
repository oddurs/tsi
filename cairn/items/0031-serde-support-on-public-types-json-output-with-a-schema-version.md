---
id: 31
title: Serde support on public types; JSON output with a schema version
type: feature
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
depends_on:
- 29
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
effort: m
area: output
breaking: 'true'
---

## Problem

Units, `Stage`, `Rocket` and `Solution` do not implement `Serialize`; the CLI builds JSON by hand
with `json!`, so the library and CLI shapes can drift apart.

## Proposal

Optional `serde` feature (on under `cli`) deriving Serialize/Deserialize. Unit newtypes serialize
as numbers with the unit in the field name (`mass_kg`). CLI JSON is `serde_json::to_string(&solution)`
plus `"schema_version": 1`.

## Acceptance criteria

- [x] Hand-built `json!` output removed
- [x] Snapshot tests of JSON output (insta)
- [x] JSON schema documented in docs/commands.md
