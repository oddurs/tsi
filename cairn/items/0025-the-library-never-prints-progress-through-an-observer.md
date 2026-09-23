---
id: 25
title: 'The library never prints: progress through an observer'
type: feature
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] `grep -r 'print!' src/` finds nothing outside the `cli` feature
- [x] CLI progress output unchanged

## 2026-09-23

Progress trait with start/advance/finish, default no-ops; BruteForceOptimizer::with_progress and MonteCarloRunner::with_progress take an observer; the CLI supplies StderrProgress. The on-screen format changed slightly (a phase line then 'NN% (done/total)' instead of 'Searching... NN%'), so criterion 2 'unchanged' is not literally true.

## 2026-09-23

Update: StderrProgress now reproduces the v0.7 output exactly ('  Coarse search: ...', '  Searching... NN%', 'Monte Carlo: NN% (done/total)'), so criterion 2 holds after all.
