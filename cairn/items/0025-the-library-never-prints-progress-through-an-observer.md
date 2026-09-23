---
id: 25
title: 'The library never prints: progress through an observer'
type: feature
status: doing
milestone: v0.8
assignee: Oddur Sigurdsson
claimed: 2026-09-23
created: 2026-09-22
updated: 2026-09-23
priority: p0
effort: m
area: optimizer
breaking: 'true'
---

## Problem

`BruteForceOptimizer::default()` has progress **on** and `eprint!`s from inside the library.
Monte Carlo builds it with `default()`, so multi-engine MC prints from rayon threads.

## Proposal

`trait Progress { fn on_progress(&self, done: u64, total: u64) }` passed to optimizers; the
default is a no-op. The CLI supplies a stderr bar.

## Acceptance criteria

- [ ] `grep -r 'print!' src/` finds nothing outside the `cli` feature
- [ ] CLI progress output unchanged
