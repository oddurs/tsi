---
id: 16
title: --gravity and --sea-level never reach the optimizer
type: bug
status: done
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p2
effort: s
area: cli
---

Both flags change only the printed output; the optimizer always uses g₀ (`commands.rs`).

## Acceptance criteria

- [x] `--gravity 3.71` (Mars) changes the TWR constraint the optimizer enforces
- [x] CLI test for each flag

## 2026-09-22

The --gravity flag takes a body (earth|mars|moon), not a number. Mars and Moon also switch the booster to vacuum Isp and vacuum thrust, since there's no atmosphere worth modelling. The --sea-level flag is now a hidden, deprecated no-op: liftoff TWR always uses sea-level thrust.
