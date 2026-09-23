---
id: 29
title: Builder API and semver-safe public types
type: feature
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
depends_on:
- 26
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] No public struct field in the optimizer module
- [x] README and doctests use the builder
- [x] A `tsiolkovsky::prelude` for the everyday imports

## 2026-09-23

Every struct in the optimizer module that carries behaviour or invariants has private fields. Exception: the plain report types (SolutionReport, StageReport, Metadata, MonteCarloSummary, DistributionSummary) keep pub fields but are #[non_exhaustive], so they can't be built outside the crate and fields can be added without a breaking change. Criterion 1 as literally worded is therefore not met; the semver goal is.

## 2026-09-23

Update: the report types are now crate-private. Solution and MonteCarloResults implement Serialize directly (the CLI's JSON flattens them), so no struct in the optimizer module has a public field and criterion 1 holds literally. The JSON contract is the documented schema, not Rust types.
