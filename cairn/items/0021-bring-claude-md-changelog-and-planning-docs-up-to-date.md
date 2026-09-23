---
id: 21
title: Bring CLAUDE.md, CHANGELOG and planning docs up to date
type: docs
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
effort: m
area: docs
---

Stale today:
- CLAUDE.md says Phase 3 / v0.3.0 / 168 tests (actual: v0.6.0, 259) and links `docs/architecture.md` (actually `docs/plan/`)
- CHANGELOG stops at 0.3.0 and dates it 2025-01-15; the real date is 2026-01-15
- No `v0.4.0` git tag
- `comfy-table` and `num-format` listed as dependencies in `libraries.md` and the old roadmap; neither is used
- `docs/engines.md:83` documents `--engines-file`, which does not exist
- `yourusername` placeholder in Cargo.toml, README, getting-started

## Acceptance criteria

- [x] CHANGELOG backfilled for 0.4.0-0.6.0 with correct dates
- [x] `v0.4.0` tag on commit `38fdafe`
- [x] CLAUDE.md describes the current state and points at cairn for the roadmap
- [x] No stale dependency or flag references remain

## 2026-09-22

Also rewrote the stale optimize sections of README, docs/commands.md and docs/examples.md from real output (they described v0.3 behaviour and missed v0.4-v0.6 flags), corrected the documented exit codes to match reality (1 for errors including infeasible, 2 for usage), fixed the --optimizer help text, and moved the physics guide's staging section onto the new model. Release dates corrected from git: v0.1.0 and v0.2.0 are 2026-01-15. Tagged v0.4.0 locally on 38fdafe (not pushed).
