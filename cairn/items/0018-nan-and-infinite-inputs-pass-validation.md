---
id: 18
title: NaN and infinite inputs pass validation
type: bug
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
effort: s
area: optimizer
---

`Problem::is_valid` checks `payload <= 0`, which is false for NaN, so NaN payloads fail later with an unrelated TWR error.

## Acceptance criteria

- [x] Every numeric input rejects NaN, ±∞ and non-positive values with a named error
- [x] Proptest over arbitrary f64 inputs: validation either rejects or the optimizer returns a finite solution
