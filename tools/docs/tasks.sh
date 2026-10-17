#!/usr/bin/env bash
# Opens a docs issue for each `Docs-Update:` and `Docs-New:` trailer in the
# commits of a push to main (.github/workflows/docs-tasks.yml).
#
#   tools/docs/tasks.sh <before> <after>
#
# `before` is the commit main was at before the push (all zeros for a new
# branch: then only `after` is read). Each trailer becomes one issue,
# labelled `docs: update` or `docs: new` plus `needs-triage`, unless an open
# issue already has the same title. DRY_RUN=1 prints the issues instead.
set -euo pipefail
before="$1"
after="$2"
if [ -z "${before//0/}" ]; then
  range=(-1 "$after")
else
  range=("$before..$after")
fi

# Oldest first, so issues come in the order the work was done.
git log --reverse --format=%H "${range[@]}" | while read -r sha; do
  subject="$(git log -1 --format=%s "$sha")"
  for key in Docs-Update Docs-New; do
    git log -1 --format="%(trailers:key=$key,valueonly,unfold)" "$sha" |
      while IFS= read -r what; do
        [ -n "$what" ] || continue
        if [ "$key" = Docs-Update ]; then
          title="Docs update: $what"
          label="docs: update"
        else
          title="New doc: $what"
          label="docs: new"
        fi
        body="$(printf 'From %s (%s).\n\nPart of #1 (user help docs).' "$sha" "$subject")"
        if [ "${DRY_RUN:-}" = 1 ]; then
          printf '%s\t%s\t%s\n' "$label" "$title" "$sha"
          continue
        fi
        open="$(gh issue list --state open --search "\"$title\" in:title" --json title |
          jq --arg t "$title" '[.[] | select(.title == $t)] | length')"
        if [ "$open" != 0 ]; then
          echo "already open: $title"
          continue
        fi
        gh issue create --title "$title" --body "$body" --label "$label" --label needs-triage
      done
  done
done
