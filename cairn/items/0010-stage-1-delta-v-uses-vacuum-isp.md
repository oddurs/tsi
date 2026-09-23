---
id: 10
title: Stage 1 delta-v uses vacuum Isp
type: bug
status: planned
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: m
area: physics
---

## What happens

`stage.rs:183`, `:194` and `analytical.rs:102` use vacuum Isp for every stage, which
overstates booster Δv by 5-10%. A Merlin-1D is 282 s at sea level and 311 s in vacuum.
`Engine::isp_at(pressure_ratio)` exists but is never called.

## What should happen

A first stage burns through the atmosphere. Model its effective Isp as a trajectory-averaged
value between sea level and vacuum. A documented blend such as `isp_at` over a nominal
pressure profile is fine. Make the model a named, swappable choice (`IspModel::Vacuum`,
`IspModel::AscentAveraged`) so the educational docs can show the difference.

## Acceptance criteria

- [ ] Booster stages default to the ascent-averaged model; upper stages to vacuum
- [ ] Falcon 9 S1 Δv validation tightened to match the new model
- [ ] Rustdoc explains why nozzle expansion makes Isp altitude-dependent
