---
id: 22
title: Three doctests are marked ignore
type: bug
status: planned
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
priority: p2
effort: s
area: docs
---

`solution.rs:34`, `monte_carlo.rs:178` and `output/mod.rs:10` use ` ```ignore ` because they reference undefined variables.

## Acceptance criteria

- [ ] All three compile and run (hide setup lines with `#`)
- [ ] `cargo test --doc` reports 0 ignored
