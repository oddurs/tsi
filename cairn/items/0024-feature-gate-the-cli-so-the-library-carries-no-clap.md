---
id: 24
title: Feature-gate the CLI so the library carries no clap
type: feature
status: done
milestone: v0.8
assignee: Oddur Sigurdsson
depends_on:
- 23
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
effort: l
area: cli
breaking: 'true'
---

## Problem

`cli` and `output` are public library modules, so every library user compiles clap,
clap_complete and clap_mangen and sees terminal formatting in the API.

## Proposal

A default-on `cli` feature owns `clap*`, the terminal/diagram formatters, and `main.rs`
(`required-features = ["cli"]`). The library exposes data, and the CLI decides how it looks.

## Acceptance criteria

- [x] `cargo build --no-default-features` compiles without clap in `cargo tree`
- [x] CI builds both feature sets
- [x] `cargo install tsiolkovsky` still installs `tsi`
