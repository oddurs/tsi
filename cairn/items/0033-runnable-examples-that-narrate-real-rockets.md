---
id: 33
title: Runnable examples/ that narrate real rockets
type: docs
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
depends_on:
- 29
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
effort: m
area: docs
---

No `examples/` directory exists. Each example is short and tells a story:

- `falcon9.rs`: why nine engines
- `hydrogen_upper_stage.rs`: why LH2 wins up high and loses down low
- `saturn_v_with_raptors.rs`: a what-if
- `monte_carlo.rs`: how much margin is enough

## Acceptance criteria

- [x] `cargo run --example <name>` works for each, with no CLI feature required
- [x] CI runs them all
