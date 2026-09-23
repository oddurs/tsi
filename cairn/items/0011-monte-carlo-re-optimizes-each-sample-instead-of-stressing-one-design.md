---
id: 11
title: Monte Carlo re-optimizes each sample instead of stressing one design
type: bug
status: planned
milestone: v0.7
depends_on:
- 8
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: l
area: optimizer
---

## What happens

`monte_carlo.rs:~250` builds a new optimal rocket for every perturbed sample. That answers
"can an optimizer close *some* design under these parameters", not "does *this* design
survive its uncertainties". With the hidden 2% margin it always reports `success_probability: 1.0`.

Also:
- Sea-level and vacuum Isp are perturbed independently (`uncertainty.rs:~140`), so sea-level Isp can exceed vacuum
- No seed, so runs are not reproducible
- `Uncertainty::new` argument order (isp, structural, thrust) does not match field order

## What should happen

Fix the nominal design, perturb the *as-built* parameters (Isp, structural mass, thrust,
propellant load), and evaluate the resulting Δv and TWR. One sampled factor per engine scales
both its sea-level and vacuum Isp.

## Acceptance criteria

- [ ] Success probability < 1.0 for a zero-margin design under default uncertainty
- [ ] `--seed N` gives bit-identical results across runs and thread counts
- [ ] Property test: sampled isp_sl ≤ isp_vac always
- [ ] Output says which design was stressed
