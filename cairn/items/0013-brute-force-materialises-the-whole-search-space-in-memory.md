---
id: 13
title: Brute force materialises the whole search space in memory
type: bug
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
depends_on:
- 12
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-22
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

- [x] Peak RSS under 50 MB for the three-stage, three-engine case
- [x] Criterion benchmark shows no regression in wall time

## 2026-09-22

Measured with /usr/bin/time -l: 3-stage, 3-engine (raptor-2, merlin-1d, rs-25) brute force peaks at 3.3 MB RSS and runs in 0.19 s wall time (release). v0.6 needed about 1 GB. The criterion comparison waits on real benchmarks (#34); benches/optimizer.rs is still a placeholder.

## 2026-09-23

Closed out in v0.8 with #34's criterion benchmarks: brute force 3 stages 1 engine in 229 us (release, Apple Silicon). v0.6 needed roughly 1 GB for this class of search and seconds of wall time.
