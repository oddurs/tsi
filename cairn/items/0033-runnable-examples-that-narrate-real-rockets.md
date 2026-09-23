---
id: 33
title: Runnable examples/ that narrate real rockets
type: docs
status: backlog
milestone: v0.8
depends_on:
- 29
created: 2026-09-22
updated: 2026-09-22
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

- [ ] `cargo run --example <name>` works for each, with no CLI feature required
- [ ] CI runs them all
