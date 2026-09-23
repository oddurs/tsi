---
id: 28
title: Validate engine and stage data at construction
type: feature
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
effort: m
area: engine
breaking: 'true'
---

Nothing checks loaded engines or `Stage` inputs. Negative masses, zero engine count, or sea-level
Isp/thrust above vacuum are all accepted.

## Acceptance criteria

- [x] `Engine::new` and TOML loading reject physically impossible data with a named error
- [x] Embedded database validated by a test
- [x] `Engine` fields all private with accessors; derives `PartialEq`
