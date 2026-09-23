---
id: 27
title: Remove panics reachable from public input
type: feature
status: backlog
milestone: v0.8
created: 2026-09-22
updated: 2026-09-22
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

- [ ] Each becomes a `Result` or is made unreachable by construction
- [ ] Proptest fuzzes public constructors with arbitrary f64 and never panics
- [ ] `clippy::unwrap_used` and `clippy::expect_used` denied in non-test library code
