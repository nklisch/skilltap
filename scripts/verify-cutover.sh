#!/bin/sh
# Evidence gate for retiring the public legacy skilltap-skills publisher.
# This is deliberately offline and never mutates a harness or sibling checkout.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

scripts/verify-release-contract.sh
scripts/verify-installer.sh
scripts/verify-install-surfaces.sh
cargo test --locked -p skilltap --test plugin_package
cargo test --locked -p skilltap --lib bootstrap_tests
cargo test --locked -p skilltap-harnesses --test bootstrap

test -f plugin/skills/skilltap/SKILL.md || {
  echo "error: canonical skilltap SKILL.md is missing" >&2
  exit 1
}
for reference in configuration instructions diagnostics; do
  test -f "plugin/skills/skilltap/references/$reference.md" || {
    echo "error: canonical skilltap reference $reference.md is missing" >&2
    exit 1
  }
done

echo "canonical cutover evidence verified; legacy retirement remains operator-gated"
