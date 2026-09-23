---
id: 7
title: GitHub Actions CI
type: chore
status: done
milestone: v0.7
assignee: Oddur Sigurdsson
depends_on:
- 6
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p0
effort: m
area: infra
---

There is no `.github/` directory, so nothing enforces the quality bar CLAUDE.md describes.

## Acceptance criteria

- [x] Workflow on push and PR: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings`
- [x] Matrix: ubuntu, macos, windows on stable
- [x] MSRV job pinned to `rust-version` in Cargo.toml
- [x] `cairn check` step
- [x] CI badge in README

## 2026-09-22

MSRV is 1.87 (u64::is_multiple_of); verified 1.86 fails and 1.87 passes cargo check --all-targets. Every job was run locally (fmt, clippy -D warnings, tests with RUSTFLAGS=-D warnings, rustdoc -D warnings, MSRV). The workflow itself has not run on GitHub yet: first push of this branch will be its first run. cairn is installed from github.com/oddurs/cairn at tag v0.2.1 because the crates.io name 'cairn' belongs to an unrelated crate. Also moved theory docs from private modules onto public types, since rustdoc -D warnings caught links to them and docs.rs would never have shown them.
