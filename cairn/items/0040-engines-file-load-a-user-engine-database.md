---
id: 40
title: '--engines-file: load a user engine database'
type: feature
status: backlog
milestone: v0.9
depends_on:
- 39
created: 2026-09-22
updated: 2026-09-22
priority: p2
effort: s
area: engine
---

Deferred since v0.2 and already documented in `docs/engines.md:83`.

## Acceptance criteria

- [ ] Merges with or replaces the embedded database (`--engines-file x.toml --no-builtin`)
- [ ] Validation errors point at file:line
