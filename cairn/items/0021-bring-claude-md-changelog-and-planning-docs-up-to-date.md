---
id: 21
title: Bring CLAUDE.md, CHANGELOG and planning docs up to date
type: docs
status: planned
milestone: v0.7
created: 2026-09-22
updated: 2026-09-22
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

- [ ] CHANGELOG backfilled for 0.4.0-0.6.0 with correct dates
- [ ] `v0.4.0` tag on commit `38fdafe`
- [ ] CLAUDE.md describes the current state and points at cairn for the roadmap
- [ ] No stale dependency or flag references remain
