---
id: 47
title: Cargo metadata and package contents
type: chore
status: backlog
milestone: v1.0
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: s
area: infra
---

Missing `rust-version`, `readme`, `documentation`, `homepage`, `authors`. `repository` points at
`yourusername`. `cargo package` currently ships CLAUDE.md, cairn/ and docs/plan.

## Acceptance criteria

- [ ] `cargo package --list` shows only what users need
- [ ] `cargo publish --dry-run` clean
