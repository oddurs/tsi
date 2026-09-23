---
id: 19
title: Property tests for optimizer invariants
type: chore
status: planned
milestone: v0.7
depends_on:
- 9
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: m
area: optimizer
---

The property suite covers units and the rocket equation but nothing about optimizers,
which is how the 11% gap between analytical and brute force went unnoticed.

## Acceptance criteria

- [ ] Every solution meets target Δv (within margin) and every TWR constraint
- [ ] Total mass is monotone in payload and in target Δv
- [ ] Analytical ≤ brute force × 1.01 wherever both apply
- [ ] Payload fraction strictly decreases as target Δv rises
