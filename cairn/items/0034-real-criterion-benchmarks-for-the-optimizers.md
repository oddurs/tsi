---
id: 34
title: Real criterion benchmarks for the optimizers
type: chore
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p2
effort: s
area: infra
---

`benches/optimizer.rs` is an 8-line placeholder.

## Acceptance criteria

- [x] Benchmarks: analytical 2-stage, brute force 3-stage single engine, brute force multi-engine, MC 10k
- [x] Baseline numbers recorded in the item notes

## 2026-09-23

Baselines, 2026-09-23, Apple Silicon (aarch64-apple-darwin), release, criterion 0.5, sample size 20: analytical 2 stages 1 engine 17.7 us; analytical 2 stages 3 engines 50.4 us; brute force 3 stages 1 engine 229 us; brute force 2 stages 3 engines 263 us; Monte Carlo 10,000 builds 661 us.
