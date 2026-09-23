---
id: 22
title: Three doctests are marked ignore
type: bug
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p2
effort: s
area: docs
---

`solution.rs:34`, `monte_carlo.rs:178` and `output/mod.rs:10` use ` ```ignore ` because they reference undefined variables.

## Acceptance criteria

- [x] All three compile and run (hide setup lines with `#`)
- [x] `cargo test --doc` reports 0 ignored
