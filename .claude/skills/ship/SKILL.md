---
name: ship
description: Land work on main through a pull request - branch, commit, push, open the PR, queue squash auto-merge, watch CI. Use whenever work in this repo is ready to merge, or the user says ship, PR, or merge.
---

# Ship a change

main is protected: every change lands as a squash-merged PR once the
required `CI OK` check is green. No reviews are required, so an agent can
take a change all the way to main.

1. **Branch** from an up-to-date main, named `type/short-slug`
   (`feat/`, `fix/`, `docs/`, `chore/`, `ci/`, `refactor/`, `perf/`, `test/`):
   `git fetch -q origin main && git switch -c fix/nan-payload origin/main`.
   One concern per branch; small PRs merge fast.
2. **Commit** with conventional messages (`fix(optimizer): ...`,
   `feat!: ...` for breaking). The `commit-msg` hook enforces the format;
   `pre-commit` checks formatting. Update CHANGELOG.md for user-visible
   changes, and tick/close the cairn item.
3. **Ship**: `scripts/ship.sh --wait` (optionally a PR title as the first
   argument; it becomes the squash commit on main). It runs the local gate,
   pushes, opens the PR, queues `--auto --squash --delete-branch`, and watches
   the required check.
4. **If CI fails**: read it with `gh run view --log-failed`, fix on the same
   branch, commit, and run `scripts/ship.sh --wait` again (auto-merge stays
   queued).
5. **After merge**: `git switch main && git pull -q --ff-only`, then delete
   the local branch.

Rules:
- Never push to main directly, never force-push a branch with an open PR
  someone else is reading, never bypass a failing check.
- No attribution trailers or "Generated with" lines in commits or PR bodies.
- Releases (tags, crates.io) are the user's call; don't tag or publish
  unasked.
