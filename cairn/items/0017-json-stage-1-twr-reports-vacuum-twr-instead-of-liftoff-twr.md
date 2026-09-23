---
id: 17
title: JSON stage-1 "twr" reports vacuum TWR instead of liftoff TWR
type: bug
status: planned
milestone: v0.7
depends_on:
- 10
created: 2026-09-22
updated: 2026-09-22
priority: p1
effort: s
area: output
---

The README example emits `"twr": 2.43` (vacuum) for stage 1; liftoff sea-level TWR is 2.24.

## Acceptance criteria

- [ ] JSON emits `twr_liftoff` (sea level, stage 1) and `twr_ignition` (vacuum, every stage), named unambiguously
- [ ] Terminal output labels which TWR it shows
