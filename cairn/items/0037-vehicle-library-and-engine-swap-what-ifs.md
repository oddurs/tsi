---
id: 37
title: Vehicle library and engine-swap what-ifs
type: feature
status: backlog
milestone: v0.9
depends_on:
- 36
created: 2026-09-22
updated: 2026-09-22
priority: p1
effort: l
area: stage
---

## Proposal

`data/vehicles.toml` describes real launchers (Falcon 9, Saturn V, Electron, Ariane 5, Starship,
Atlas V) by stage: engines, propellant, dry and wet mass, with sources. `tsi vehicle saturn-v`
prints its performance. `tsi vehicle saturn-v --swap-engine s1=raptor-2` answers the what-if
questions from the concept doc.

## Acceptance criteria

- [ ] At least six vehicles, each validated in `tests/validation.rs` against published payload to LEO within 20%
- [ ] `--swap-engine` re-sizes the stage and reports the payload change
- [ ] `tsi vehicle --list`
