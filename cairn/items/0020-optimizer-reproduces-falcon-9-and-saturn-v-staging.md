---
id: 20
title: Optimizer reproduces Falcon 9 and Saturn V staging
type: validation
status: planned
milestone: v0.7
depends_on:
- 9
- 10
created: 2026-09-22
updated: 2026-09-22
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

- [ ] Fix the unstacked F9 test at `validation.rs:196`
- [ ] Given F9's engines, payload and Δv, the optimizer's stage split lands within tolerance
- [ ] Same for Saturn V
- [ ] Each test cites its source in a comment
