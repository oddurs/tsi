---
id: 24
title: Feature-gate the CLI so the library carries no clap
type: feature
status: backlog
milestone: v0.8
depends_on:
- 23
created: 2026-09-22
updated: 2026-09-22
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

- [ ] `cargo build --no-default-features` compiles without clap in `cargo tree`
- [ ] CI builds both feature sets
- [ ] `cargo install tsiolkovsky` still installs `tsi`
