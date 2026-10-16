#!/usr/bin/env bash
# Builds the openvolley oracle image and writes its CSVs to target/parity/oracle.
# The Rust parity test (crates/dvw-codec/tests/parity.rs) reads them from there.
#
# `run.sh validate` instead generates the bad-file set (target/bad, from
# testdata/bad/recipes.toml) and writes dv_validate's messages for it and
# for the clean corpus to target/parity/oracle/validate/{bad,corpus}.
#
# `run.sh rehearsal` runs the console rehearsal (a whole match coded through
# the console, crates/courtside-console/tests/rehearsal.rs) and has
# openvolley read and validate its exports into target/parity/rehearsal.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
docker build -q -t courtside-parity "$root/tools/parity" >/dev/null
if [ "${1:-}" = rehearsal ]; then
  (cd "$root" && cargo test -q -p courtside-console --test rehearsal)
  out="$root/target/parity/rehearsal"
  rm -rf "$out"
  mkdir -p "$out"
  docker run --rm -v "$root/target/tmp/rehearsal:/r:ro" -v "$out:/out" \
    courtside-parity validate /r "/out"
  exit 0
fi
if [ "${1:-}" = validate ]; then
  (cd "$root" && cargo test -q -p vb-engine --test bad_files bad_files_are_generated)
  out="$root/target/parity/oracle/validate"
  rm -rf "$out"
  mkdir -p "$out"
  docker run --rm -v "$root/target/bad:/bad:ro" -v "$out:/out" \
    courtside-parity validate /bad /out/bad
  docker run --rm -v "$root/testdata:/corpus:ro" -v "$out:/out" \
    courtside-parity validate /corpus /out/corpus
  exit 0
fi
rm -rf "$root/target/parity/oracle"
mkdir -p "$root/target/parity/oracle"
docker run --rm \
  -v "$root/testdata:/corpus:ro" \
  -v "$root/target/parity/oracle:/out" \
  courtside-parity /corpus /out
