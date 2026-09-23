---
id: 41
title: Colour output that respects NO_COLOR and tty detection
type: feature
status: done
milestone: v0.9
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p2
effort: s
area: output
---

Deferred since v0.3. Highlight the key numbers (Δv, payload fraction, the binding constraint) and warnings.

## Acceptance criteria

- [x] Honours `NO_COLOR`, `--color auto|always|never`, and pipes
- [x] Colour never carries meaning on its own (still readable in monochrome)

## 2026-09-23

anstream AutoStream: NO_COLOR, pipes and --color auto|always|never; tones are semantic, output reads fine in monochrome.
