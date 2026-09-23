---
id: 60
title: 'Agentic PR workflow: ship script, hooks, auto-merge'
type: chore
status: doing
milestone: v0.9
created: 2026-09-23
updated: 2026-09-23
priority: p1
area: infra
effort: s
---

## 2026-09-23

scripts/ship.sh + ship skill; .githooks via core.hooksPath (commit-msg, pre-commit fmt, cairn post-merge); CI gains concurrency, PR-title check and one required 'CI OK' job; repo set to squash-only auto-merge.
