---
id: 41
title: Colour output that respects NO_COLOR and tty detection
type: feature
status: backlog
milestone: v0.9
created: 2026-09-22
updated: 2026-09-22
priority: p2
effort: s
area: output
---

Deferred since v0.3. Highlight the key numbers (Δv, payload fraction, the binding constraint) and warnings.

## Acceptance criteria

- [ ] Honours `NO_COLOR`, `--color auto|always|never`, and pipes
- [ ] Colour never carries meaning on its own (still readable in monochrome)
