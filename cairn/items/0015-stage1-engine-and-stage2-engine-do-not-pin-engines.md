---
id: 15
title: --stage1-engine and --stage2-engine do not pin engines
type: bug
status: done
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
effort: m
area: cli
---

## What happens

`--stage1-engine merlin-1d --engine raptor-2` returns a Raptor-2 first stage. The flags only
add engines to the pool.

## What should happen

Pinned engines constrain their stage. The problem model gains per-stage engine sets.

## Acceptance criteria

- [x] CLI test asserting the pinned engine appears in the pinned stage
- [x] Library API: `Problem` supports per-stage engine choices
