---
id: 58
title: Gravity-loss-aware staging optimization
type: feature
status: backlog
milestone: later
created: 2026-09-22
updated: 2026-09-22
priority: p2
effort: l
area: optimizer
part_of:
- 54
---

## Problem

Ideal staging theory ignores gravity losses, so it puts as much delta-v as possible on high-Isp, low-thrust upper stages. Asked to do Saturn V's job with its engines, the optimizer designs a rocket about 30% lighter than the real one, with the S-IC cut to the 2 km/s floor (see #20 and tests/validation.rs). Falcon 9 shows the same lean, more mildly.

## Proposal

Charge each stage its gravity loss as a function of its TWR and burn time, and optimize total delta-v *including* losses. The trajectory work in #54 would give this a physical basis; an empirical per-stage model could come first.

## Acceptance criteria

- [ ] Saturn V redesign lands within 15% of the real vehicle's mass and stage split
- [ ] Falcon 9 stage split within 400 m/s of the real one
