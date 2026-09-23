---
id: 59
title: 'CLI output system: structured data, documents, charts and diagrams'
type: feature
status: done
milestone: v0.9
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p1
effort: l
area: output
---

## Problem

The CLI prints in three unrelated styles (banner boxes, panels, key: value lines), with hand-padded boxes that misalign, a histogram frame that doesn't line up, '-0 m/s', a 25-line rocket drawing, and no colour. Only optimize and engines have JSON, in different shapes.

## Proposal

A small output system in the binary:
- **Data**: each command produces one serializable data structure; --output json emits it in a common envelope (schema_version, command).
- **Document model**: title, fields, table, chart, art and note blocks made of spans with semantic tones.
- **Renderer**: width-aware layout, Unicode or ASCII glyphs, colour through anstream (respects NO_COLOR, pipes, --color).
- **Charts**: stacked delta-v and mass budgets, loss bars, a compact Monte Carlo histogram with a target marker, and a compact rocket diagram.

## Acceptance criteria

- [x] Every command renders through the document model; no hand-padded boxes remain
- [x] --output json on calculate, engines and optimize, each with schema_version and command
- [x] --color auto|always|never and NO_COLOR respected; --ascii for plain glyphs
- [x] Snapshot tests of pretty output for each command
- [x] docs/commands.md and README show the new output

## 2026-09-23

Data → Doc → renderer. Views in src/output/views.rs; one renderer with Unicode/ASCII glyphs, anstream colour; JSON envelopes carry command; pretty snapshots pin the look.
