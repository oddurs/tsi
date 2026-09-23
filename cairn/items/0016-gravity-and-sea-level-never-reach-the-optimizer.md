---
id: 16
title: --gravity and --sea-level never reach the optimizer
type: bug
status: planned
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
priority: p2
effort: s
area: cli
---

Both flags change only the printed output; the optimizer always uses g₀ (`commands.rs`).

## Acceptance criteria

- [ ] `--gravity 3.71` (Mars) changes the TWR constraint the optimizer enforces
- [ ] CLI test for each flag
