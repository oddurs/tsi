---
id: 12
title: Brute force misses small rockets and silently prunes engines
type: bug
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
effort: m
area: optimizer
---

## What happens

- Propellant grid floor is 10 t (`brute_force.rs:101`): `--payload 300 --engine rutherford --optimizer brute-force` reports infeasible; analytical finds 11.5 t
- Only the top 3 engines are kept, silently (`:291`)
- `iterations` reports only the refinement pass because the counter is reset (`:547`)
- Max 9 engines per stage is hardcoded, so no 33-Raptor Super Heavy

## Acceptance criteria

- [x] Grid bounds scale from payload and target Δv
- [x] Any pruning is reported in the solution metadata
- [x] `iterations` counts every evaluated configuration
- [x] `--max-engines` reaches the brute force optimizer; Super Heavy (33 × Raptor-2) is expressible

## 2026-09-22

Grid bounds scale with payload (0.05x to 2000x); target delta-v enters through the feasibility filter rather than the bounds. The coarse grid doubles in density (up to 3 times) if it finds nothing, which fixed a property-test case near Merlin's structural limit. There is no pruning left to report: every engine is tried on every stage, and engine count is solved in closed form rather than searched.
