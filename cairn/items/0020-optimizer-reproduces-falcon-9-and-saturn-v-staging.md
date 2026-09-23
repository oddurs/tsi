---
id: 20
title: Optimizer reproduces Falcon 9 and Saturn V staging
type: validation
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
depends_on:
- 9
- 10
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
effort: m
area: physics
---

## Reference

Today's validation tests check rocket-equation arithmetic but never ask the optimizer to
design a real rocket. `validation.rs:196` also sums F9 stage Δv without stacking stage 2's
mass on stage 1, and asserts > 18,000 m/s. That is physically wrong.

Vehicle / engine: Falcon 9 Block 5 (Merlin-1D ×9 / MVac ×1), Saturn V (F-1 ×5 / J-2 ×5 / J-2 ×1)
Source: SpaceX Falcon User's Guide (2021); NASA SP-4206 *Stages to Saturn*, appendix

## Expected

| Quantity | Published | Tolerance |
|----------|-----------|-----------|
| F9 stacked ideal Δv | ~9,300 m/s | ±5% |
| Saturn V stage mass split | S-IC 2,290 t / S-II 496 t / S-IVB 123 t | ±15% |

## Acceptance criteria

- [x] Fix the unstacked F9 test at `validation.rs:196`
- [x] Given F9's engines, payload and Δv, the optimizer's stage split lands within tolerance
- [ ] Same for Saturn V
- [x] Each test cites its source in a comment

## 2026-09-22

Falcon 9 reproduced: optimizer with F9's engines pinned designs 581 t vs real 571.5 t (+1.7%), 9 Merlins + 1 MVac, stage split within 700 m/s. Saturn V is NOT reproduced, and cannot be under ideal staging theory: the optimum is about 30% lighter than the real vehicle, with the S-IC at the 2 km/s floor, because the J-2's Isp dominates once gravity losses are ignored. The test now asserts that finding and explains it. Criterion 3 stays unticked; gravity-loss-aware staging is tracked in the later milestone.
