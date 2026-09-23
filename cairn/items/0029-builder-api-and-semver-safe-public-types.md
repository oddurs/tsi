---
id: 29
title: Builder API and semver-safe public types
type: feature
status: backlog
milestone: v0.8
depends_on:
- 26
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: l
area: optimizer
breaking: 'true'
---

## Problem

`Problem`, `Constraints`, `Solution`, `MonteCarloResults` and `Uncertainty` have all-`pub` fields.
`Constraints::new` is positional and silently sets `max_engines = 9`. `Propellant`, error enums
and `TwrError` lack `#[non_exhaustive]`. rand 0.8 types leak via `sample_factor_with_rng`.
`optimizer_name: String` is stringly-typed.

## Proposal

`Problem::builder().payload(..).target(..).engines(..).build()?`. Private fields with accessors,
`#[non_exhaustive]` where variants will grow, an `OptimizerKind` enum, and no rand types in
signatures (take a `u64` seed).

## Acceptance criteria

- [ ] No public struct field in the optimizer module
- [ ] README and doctests use the builder
- [ ] A `tsiolkovsky::prelude` for the everyday imports
