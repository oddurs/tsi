---
id: 6
title: Make clippy clean across all targets
type: chore
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: s
area: infra
---

`cargo clippy --all-targets` fails today:

- **error:** `approx_constant` at `src/units/ratio.rs:77` (3.14159 in a test)
- `manual_range_contains` at `src/optimizer/uncertainty.rs:346` and `tests/validation.rs:347`, `:408`
- `assertions_on_constants` at `src/output/terminal.rs:521-522`

## Acceptance criteria

- [x] `cargo clippy --all-targets -- -D warnings` passes
