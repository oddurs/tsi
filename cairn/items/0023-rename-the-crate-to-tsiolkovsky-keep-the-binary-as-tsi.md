---
id: 23
title: Rename the crate to tsiolkovsky; keep the binary as tsi
type: chore
status: backlog
milestone: v0.8
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: s
area: infra
breaking: 'true'
---

`tsi` is taken on crates.io ("Terminal Speech Interface", 0.1.0). `tsiolkovsky` is free
and says what the crate does.

Decision needed: confirm `tsiolkovsky` (library `use tsiolkovsky::...`), binary name `tsi` via `[[bin]]`.

## Acceptance criteria

- [ ] `[package] name = "tsiolkovsky"`, `[[bin]] name = "tsi"`
- [ ] All doctests, examples and docs use the new path
- [ ] Name reserved on crates.io with a 0.8 publish
