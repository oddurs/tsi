---
id: 32
title: Loss models use Velocity, share G0, and cover every stage
type: chore
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p2
effort: s
area: physics
---

`losses.rs` duplicates `G0`, returns raw f64, and counts only stage 1. Gravity loss `0.85/√TWR`
and a flat 100 m/s steering loss are fine as first-order models if documented as such.

## Acceptance criteria

- [x] Returns `Velocity`; single `G0` in `physics`
- [x] Rustdoc states each empirical model's origin and where it breaks down

## 2026-09-23

Returns Velocity, uses physics::G0, and each model's rustdoc now says it is an empirical fit, what it was fitted to, and where it breaks down. Not done: per-stage (upper-stage) gravity loss. I found no defensible closed-form model; any factor would have been invented. The docs say so and point to the trajectory work (#54). Added ascent_losses(&Rocket).
