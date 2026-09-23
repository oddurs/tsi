---
id: 13
title: Brute force materialises the whole search space in memory
type: bug
status: planned
milestone: v0.7
depends_on:
- 12
created: 2026-09-22
updated: 2026-09-22
priority: p1
effort: m
area: optimizer
---

## What happens

Every configuration is collected into a Vec before evaluation. A single-engine three-stage
run peaks around 1 GB RSS; three engines is about 405³ ≈ 66M configurations.

## What should happen

Stream candidates through rayon (`par_bridge` or nested `into_par_iter` over the
dimensions) and fold to the best solution. Memory should be O(threads), not O(space).

## Acceptance criteria

- [ ] Peak RSS under 50 MB for the three-stage, three-engine case
- [ ] Criterion benchmark shows no regression in wall time
