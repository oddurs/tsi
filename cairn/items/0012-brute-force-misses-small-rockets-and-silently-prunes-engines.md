---
id: 12
title: Brute force misses small rockets and silently prunes engines
type: bug
status: planned
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
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

- [ ] Grid bounds scale from payload and target Δv
- [ ] Any pruning is reported in the solution metadata
- [ ] `iterations` counts every evaluated configuration
- [ ] `--max-engines` reaches the brute force optimizer; Super Heavy (33 × Raptor-2) is expressible
