---
id: 23
title: Rename the crate to tsiolkovsky; keep the binary as tsi
type: chore
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
effort: s
area: infra
breaking: 'true'
---

`tsi` is taken on crates.io ("Terminal Speech Interface", 0.1.0). `tsiolkovsky` is free
and says what the crate does.

Decision needed: confirm `tsiolkovsky` (library `use tsiolkovsky::...`), binary name `tsi` via `[[bin]]`.

## Acceptance criteria

- [x] `[package] name = "tsiolkovsky"`, `[[bin]] name = "tsi"`
- [x] All doctests, examples and docs use the new path
- [ ] Name reserved on crates.io with a 0.8 publish

## 2026-09-23

Renamed: crate tsiolkovsky, [[bin]] tsi. Every tsi:: path in code, tests and docs updated. Not published: reserving the name on crates.io is a public, permanent action and waits for the owner's go-ahead (criterion 3 open).
