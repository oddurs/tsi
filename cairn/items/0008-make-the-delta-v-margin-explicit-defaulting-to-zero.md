---
id: 8
title: Make the delta-v margin explicit, defaulting to zero
type: feature
status: doing
milestone: v0.7
assignee: Oddur Sigurdsson
claimed: 2026-09-22
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: s
area: optimizer
breaking: 'true'
---

## Problem

`AnalyticalOptimizer` silently sizes for 1.02 × target (`analytical.rs:241`).
For 9,400 m/s that means designing for 9,588 m/s, about 7% extra mass, and the
user cannot turn it off. `tests/validation.rs:~346` asserts the overshoot as correct.

## Proposal

`Constraints::margin: Ratio` (default 0) plus `--margin 2%` on the CLI. Both
optimizers honour it the same way. Output reports the margin requested and the margin achieved.

## Acceptance criteria

- [x] Default margin is zero in library and CLI
- [x] Brute force and analytical apply it identically
- [x] Validation test updated to assert the new contract
- [ ] CHANGELOG notes the behaviour change
