---
id: 30
title: Per-stage structural ratio, with the textbook definition
type: feature
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
effort: m
area: stage
breaking: 'true'
---

## Problem

tsi's structural ratio is structure ÷ propellant, excluding engines (`stage.rs:35`). The textbook
structural coefficient is ε = dry ÷ (dry + propellant). One ε also applies to every stage, but a
LH2 upper stage and an RP-1 booster differ a lot.

## Proposal

Per-stage ε, named for what it is. Either adopt the textbook ε or rename the current value
`tankage_fraction` and document the conversion. Rustdoc gets a table of real stage values
(S-IC, S-II, F9 S1, Centaur).

## Acceptance criteria

- [x] Naming and definition match the documentation exactly
- [x] `--structural-ratio` accepts per-stage values (`0.06,0.09`)
- [x] Reference table in rustdoc with sources
