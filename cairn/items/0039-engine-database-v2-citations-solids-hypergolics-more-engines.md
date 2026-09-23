---
id: 39
title: 'Engine database v2: citations, solids, hypergolics, more engines'
type: feature
status: backlog
milestone: v0.9
created: 2026-09-22
updated: 2026-09-22
priority: p1
effort: l
area: data
---

## Problem

Values are broadly right but unsourced. BE-4 Isp and mass are estimates and aren't flagged.
Raptor-Vacuum dry mass (1,600 kg) understates the large nozzle. Rutherford conflates its SL and
vacuum variants (published SL Isp is 311 s, not 303). There are no solids or hypergolics, even
though both `Propellant` variants exist.

## Proposal

Schema: `source`, `estimated: bool`, `throttle_min`, `restartable`, `mixture_ratio`,
`expansion_ratio`, `status` (active/retired/development). Add about 14 engines: Vulcain 2,
Vinci, RS-68A, RD-171, NK-33, LE-9, YF-100, BE-3U, Archimedes, Rutherford Vacuum, Merlin-1D
Block 5, Shuttle SRB, GEM-63, P120C.

## Acceptance criteria

- [ ] Every engine has a `source`
- [ ] `tsi engines` marks estimated values
- [ ] Known data errors fixed
- [ ] Solid motors behave sensibly in `calculate` (fixed propellant load)
