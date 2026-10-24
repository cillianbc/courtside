#!/usr/bin/env bash
# Writes the CLI's source (AGPL-3.0-only) into a checkout of the public
# repo cillianbc/courtside-cli, as one snapshot of this repo's HEAD:
# the five crates the CLI builds from, LICENSE, a workspace Cargo.toml and
# the lock file. No history, no corpus, no app code (console, store,
# desktop). Run before tagging a CLI release; then commit and push there.
#
#   tools/publish/cli-source.sh <path to courtside-cli checkout>
set -euo pipefail

dest=${1:?usage: cli-source.sh <courtside-cli checkout>}
root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
crates=(dvw-codec vb-grammar vb-engine courtside-ffi courtside-cli)

[ -d "$dest/.git" ] || { echo "$dest is not a git checkout" >&2; exit 1; }
if [ -n "$(git -C "$root" status --porcelain -- crates LICENSE Cargo.toml Cargo.lock)" ]; then
  echo "commit the crates first: the snapshot is HEAD" >&2
  exit 1
fi

# Replace what a previous snapshot wrote; the README is the public repo's own.
rm -rf "$dest/crates" "$dest/Cargo.toml" "$dest/Cargo.lock" "$dest/LICENSE"
paths=(LICENSE Cargo.lock)
for c in "${crates[@]}"; do paths+=("crates/$c"); done
git -C "$root" archive HEAD "${paths[@]}" | tar -x -C "$dest"

# The workspace manifest with only these crates.
python3 -I - "$root/Cargo.toml" "$dest/Cargo.toml" "${crates[@]}" <<'PY'
import re, sys
src, out, crates = sys.argv[1], sys.argv[2], sys.argv[3:]
s = open(src).read()
members = "members = [\n" + "".join(f'    "crates/{c}",\n' for c in crates) + "]"
s = re.sub(r"members = \[.*?\]", members, s, count=1, flags=re.S)
s = re.sub(r"^courtside-(store|console) = \{ path = .*\n", "", s, flags=re.M)
head = "# The courtside CLI's source, published from the app's private repository.\n\n"
open(out, "w").write(head + s)
PY

printf '/target\n' > "$dest/.gitignore"

# Cargo trims the lock to these crates; the build must pass on its own.
(cd "$dest" && cargo build -q -p courtside-cli && cargo test -q --workspace >/dev/null)
git -C "$dest" add -A
echo "snapshot of $(git -C "$root" rev-parse --short HEAD) staged in $dest"
