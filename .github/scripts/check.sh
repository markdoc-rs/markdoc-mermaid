#!/usr/bin/env bash
#
# Everything that has to be true before markdoc-mermaid is published.
#
# ONE COPY: ci.yml runs it on every push and pull request, release.yml runs
# it before anything is published. A second list of steps drifts, and the
# half that drifts is always the one nobody watches.
#
# Inputs:
#   $1          the markdoc-mermaid checkout           default: $PWD
#   STRICT      1 to fail on clippy warnings           default: 1
set -euo pipefail

ROOT="${1:-$PWD}"
cd "$ROOT"
ROOT="$PWD"
STRICT="${STRICT:-1}"

group() { echo "::group::$*"; }
endgroup() { echo "::endgroup::"; }
die() { echo "::error::$*" >&2; exit 1; }

[ -f Cargo.toml ] || die "no Cargo.toml in $ROOT — point this at the markdoc-mermaid checkout"

failed=""
step() {
  local name="$1"; shift
  group "$name"
  if "$@"; then
    endgroup
  else
    endgroup
    echo "::error::$name failed"
    failed="$failed $name"
  fi
}

step "cargo fmt" cargo fmt --check


# `serde` is optional and derives on the graph types, so it is its own
# compile: a derive that stops building is invisible to the default build.
if [ "$STRICT" = "1" ]; then
  step "cargo clippy" cargo clippy --all-targets -- -D warnings
  step "cargo clippy --features serde" cargo clippy --all-targets --features serde -- -D warnings
else
  step "cargo clippy" cargo clippy --all-targets
  step "cargo clippy --features serde" cargo clippy --all-targets --features serde
fi

step "cargo test" cargo test --all-targets
step "cargo test --features serde" cargo test --all-targets --features serde

# Release as well as debug, and not only because an overflow that panics in
# debug wraps in release: the renderer's own layout validation panics in a
# debug build when edges name one subgraph by both its id and its title, so
# the SVG half of that test is release-only and runs nowhere else.
step "cargo test --release" cargo test --release --all-targets --features serde

# Doctests are not --all-targets.
step "cargo test --doc" cargo test --doc --features serde

for s in .github/scripts/*.sh; do
  step "bash -n $(basename "$s")" bash -n "$s"
done

if [ -n "$failed" ]; then
  die "failed:$failed"
fi
echo "==> all checks passed"
