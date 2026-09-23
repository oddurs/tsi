---
id: 42
title: 'tsi sweep: trade studies as a table or CSV'
type: feature
status: backlog
milestone: v0.9
created: 2026-09-22
updated: 2026-09-22
priority: p2
effort: m
area: cli
---

## Proposal

`tsi sweep --payload 1t..20t --steps 20 --engine raptor-2` prints how total mass and payload fraction
scale, as a table, CSV or JSON. This is the scriptable "what if" loop the concept doc promises, without
a shell for-loop.

## Acceptance criteria

- [ ] Sweep any one of payload, target Δv, structural ratio, min TWR
- [ ] `-o csv` pipes cleanly into other tools
