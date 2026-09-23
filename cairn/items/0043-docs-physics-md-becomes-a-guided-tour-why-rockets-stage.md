---
id: 43
title: 'docs/physics.md becomes a guided tour: why rockets stage'
type: docs
status: backlog
milestone: v0.9
depends_on:
- 33
created: 2026-09-22
updated: 2026-09-22
priority: p1
effort: m
area: docs
---

Rewrite the physics guide as a walk-through the reader can follow in the terminal. It starts with
one stage that can't reach orbit, adds a second, then shows the optimum split, hydrogen vs kerosene,
and gravity losses. Every section has a runnable `tsi` command and its real output.

## Acceptance criteria

- [ ] Every command in the doc is exercised by a CLI test so the doc can't rot
- [ ] Includes the orbital Δv reference table and propellant comparison table
