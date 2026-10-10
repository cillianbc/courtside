#!/usr/bin/env bash
# Builds the openvolley oracle image and writes its CSVs to target/parity/oracle.
# The Rust parity test (crates/dvw-codec/tests/parity.rs) reads them from there.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
docker build -q -t courtside-parity "$root/tools/parity" >/dev/null
rm -rf "$root/target/parity/oracle"
mkdir -p "$root/target/parity/oracle"
docker run --rm \
  -v "$root/testdata:/corpus:ro" \
  -v "$root/target/parity/oracle:/out" \
  courtside-parity /corpus /out
