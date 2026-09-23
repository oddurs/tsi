---
id: 32
title: Loss models use Velocity, share G0, and cover every stage
type: chore
status: doing
milestone: v0.8
assignee: Oddur Sigurdsson
claimed: 2026-09-23
created: 2026-09-22
updated: 2026-09-23
priority: p2
effort: s
area: physics
---

`losses.rs` duplicates `G0`, returns raw f64, and counts only stage 1. Gravity loss `0.85/√TWR`
and a flat 100 m/s steering loss are fine as first-order models if documented as such.

## Acceptance criteria

- [ ] Returns `Velocity`; single `G0` in `physics`
- [ ] Rustdoc states each empirical model's origin and where it breaks down
