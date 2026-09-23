---
id: 4
key: v1.0
title: Liftoff
type: milestone
status: backlog
depends_on:
- 3
created: 2026-09-22
updated: 2026-09-22
priority: p2
due: 2026-12-01
---

API freeze, release, and publication.

Nothing new in the library, only review, documentation and packaging.
After this, breaking the API costs a 2.0.

**Exit criteria:** `cargo install tsiolkovsky` and `brew install` both give a
working `tsi`. docs.rs has no undocumented public items. cargo-semver-checks runs in CI.
