---
id: 27
title: Remove panics reachable from public input
type: feature
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
effort: m
area: stage
breaking: 'true'
---

Known sites:
- `Rocket::new` asserts (`rocket.rs:100`)
- `partial_cmp().unwrap()` panics on NaN (`brute_force.rs:165`, `:175`), so use `total_cmp`
- `Normal::new(..).expect` on negative sigma (`uncertainty.rs`)
- `EngineDatabase::default()` calls `expect`

## Acceptance criteria

- [x] Each becomes a `Result` or is made unreachable by construction
- [x] Proptest fuzzes public constructors with arbitrary f64 and never panics
- [x] `clippy::unwrap_used` and `clippy::expect_used` denied in non-test library code

## 2026-09-23

Review follow-up: stage-index methods on Rocket (stage_delta_v, stage_twr, stage_twr_in) still panic past the top, like slice indexing. They now document it under '# Panics', and Rocket::stage(i) is the checked lookup. The release notes' 'never panics on public input' was narrowed to 'invalid values are errors, not panics'.
