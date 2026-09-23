---
id: 7
title: GitHub Actions CI
type: chore
status: planned
milestone: v0.7
depends_on:
- 6
created: 2026-09-22
updated: 2026-09-22
priority: p0
effort: m
area: infra
---

There is no `.github/` directory, so nothing enforces the quality bar CLAUDE.md describes.

## Acceptance criteria

- [ ] Workflow on push and PR: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings`
- [ ] Matrix: ubuntu, macos, windows on stable
- [ ] MSRV job pinned to `rust-version` in Cargo.toml
- [ ] `cairn check` step
- [ ] CI badge in README
