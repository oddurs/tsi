---
id: 38
title: 'Unit-aware inputs: 5t, 9.4km/s, 2450kN'
type: feature
status: backlog
milestone: v0.9
created: 2026-09-22
updated: 2026-09-22
priority: p1
effort: m
area: units
---

## Proposal

Implement `FromStr` for `Mass`, `Velocity`, `Force` and `Isp`. The CLI accepts `--payload 5t`,
`--target-dv 9.4km/s`. Bare numbers keep today's units. Errors name the accepted suffixes.

## Acceptance criteria

- [ ] Proptest: `parse(format(x)) == x` for every unit type
- [ ] Imperial input (lb, lbf, ft/s) accepted and converted, never output unless asked
