---
id: 26
title: Typed errors with structured infeasibility reasons
type: feature
status: doing
milestone: v0.8
assignee: Oddur Sigurdsson
claimed: 2026-09-23
created: 2026-09-22
updated: 2026-09-23
priority: p0
effort: m
area: optimizer
breaking: 'true'
---

## Problem

`EngineDatabase` returns `anyhow::Result`. `OptimizeError::Infeasible { reason: String }`
carries CLI flag advice such as "Lower --structural-ratio", which means nothing to a library caller.

## Proposal

`thiserror` enums throughout the library. `Infeasible` carries a structured cause
(`TwrBelowMinimum { stage, achieved, required }`, `MassRatioUnreachable { ... }`) and the CLI
turns causes into advice. `anyhow` becomes a `cli`-only dependency.

## Acceptance criteria

- [ ] No `anyhow` outside the `cli` feature
- [ ] Every CLI suggestion is derived from a structured cause
- [ ] Error enums are `#[non_exhaustive]`
