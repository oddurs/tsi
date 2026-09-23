---
id: 14
title: --max-stages is treated as the exact stage count
type: bug
status: done
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
effort: s
area: cli
---

## What happens

`commands.rs:~423` passes `--max-stages` to `with_stage_count`, forcing exactly N stages.

## What should happen

`--max-stages 3` searches 1, 2 and 3 stages and reports the best. Add `--stages N` for an exact count.

## Acceptance criteria

- [x] CLI test: `--max-stages 3` can return a 2-stage solution
