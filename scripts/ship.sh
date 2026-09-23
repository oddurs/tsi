#!/usr/bin/env bash
# Ship the current branch: local gate, push, open a PR, queue auto-merge.
#
#   scripts/ship.sh                 # PR title from the first commit on the branch
#   scripts/ship.sh "fix: title"    # or give one (it becomes the squash commit)
#   scripts/ship.sh --wait          # also block until CI finishes
#   SHIP_FAST=1 scripts/ship.sh     # skip the local clippy/test gate
#
# The PR squash-merges itself once the required "CI OK" check passes, and the
# branch is deleted. Nothing here merges past a red CI.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

wait=0
title=""
for arg in "$@"; do
  case "$arg" in
    --wait) wait=1 ;;
    *) title="$arg" ;;
  esac
done

branch=$(git branch --show-current)
base=main
[ -n "$branch" ] && [ "$branch" != "$base" ] || { echo "ship: create a branch first (git switch -c feat/...)" >&2; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "ship: commit or stash your changes first" >&2; exit 1; }

git fetch -q origin "$base"
ahead=$(git rev-list --count "origin/$base..HEAD")
[ "$ahead" -gt 0 ] || { echo "ship: nothing to ship; $branch has no commits beyond $base" >&2; exit 1; }

if [ "${SHIP_FAST:-}" != 1 ]; then
  echo "ship: local gate (fmt, clippy, tests)"
  cargo fmt --all --check
  cargo clippy -q --all-targets -- -D warnings
  cargo test -q >/tmp/ship-test.log 2>&1 || { grep -E 'FAILED|panicked' /tmp/ship-test.log >&2; echo "ship: tests fail locally" >&2; exit 1; }
  command -v cairn >/dev/null && cairn check -q
fi

git push -q -u origin HEAD

if ! gh pr view --json number >/dev/null 2>&1; then
  if [ -z "$title" ]; then
    title=$(git log --reverse --format=%s "origin/$base..HEAD" | head -n1)
  fi
  body=$(git log --reverse --format='- %s' "origin/$base..HEAD")
  gh pr create --base "$base" --title "$title" --body "$body" >/dev/null
fi

gh pr merge --auto --squash --delete-branch >/dev/null
url=$(gh pr view --json url -q .url)
echo "ship: $url (auto-merge queued)"

if [ "$wait" = 1 ]; then
  # Checks appear once CI's runners pick the run up; that can take minutes
  for _ in $(seq 300); do
    gh pr checks 2>&1 | grep -q "no .*checks reported" || break
    sleep 2
  done
  gh pr checks --watch --fail-fast --interval 15 || { echo "ship: CI failed; fix, commit, and run ship again" >&2; exit 1; }
  echo "ship: CI green; merging"
fi
