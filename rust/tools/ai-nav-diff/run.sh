#!/usr/bin/env bash
# Differential test of AI navigation (phase 2): original C++ vs. the
# Rust-owned facades, through the public C++ API only.
# Usage: run.sh <repo-root> <pristine-root>
#   <pristine-root>: HPL2/core from before any AI port, e.g.
#   git archive e91d65c HPL2/core | tar x -C <pristine-root>
# VARIANT=rust_release SKIP_CARGO=1 run.sh ... reuses an existing library.
set -euo pipefail
REPO=$(cd "${1:?repo root}" && pwd)
PRISTINE=$(cd "${2:?pristine root (HPL2/core before the AI ports)}" && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=$REPO/rust/target/ai-nav-diff
RUST=$REPO/rust
CXX=${CXX:-g++}
VARIANT=${VARIANT:-all}
mkdir -p "$OUT"
if [ "$VARIANT" = all ]; then
  variants=(original cpp_switch_off rust_release rust_debug)
else
  case "$VARIANT" in original|cpp_switch_off|rust_release|rust_debug) ;; *) echo "Unknown variant: $VARIANT" >&2; exit 2;; esac
  variants=("$VARIANT")
fi

for variant in "${variants[@]}"; do
  srcroot=$REPO
  [ "$variant" != original ] || srcroot=$PRISTINE
  core=$srcroot/HPL2/core
  dir=$OUT/$variant
  mkdir -p "$dir"
  defs=() link=()
  sources=("$core/sources/ai/AINodeContainer.cpp" "$core/sources/ai/AStar.cpp"
    "$core/sources/impl/tinyXml/tinyxml.cpp" "$core/sources/impl/tinyXml/tinyxmlparser.cpp" "$core/sources/impl/tinyXml/tinyxmlerror.cpp"
    "$core/sources/graphics/Color.cpp" "$HERE/stubs.cpp" "$HERE/diff_main.cpp")
  [ "$variant" != cpp_switch_off ] || defs+=(-DHPL2_RUST_AI=0)
  if [[ "$variant" = rust_* ]]; then
    profile=${variant#rust_}
    cargoargs=()
    [ "$profile" != release ] || cargoargs+=(--release)
    if [ "${SKIP_CARGO:-0}" != 1 ]; then (cd "$RUST" && cargo build -q --locked -p hpl2-ffi "${cargoargs[@]}"); fi
    defs+=(-DHPL2_RUST_AI=1 -DHARNESS_RUST=1)
    sources+=("$core/sources/ai/AINodeContainerRust.cpp" "$core/sources/ai/AStarRust.cpp" "$core/sources/system/RustBridge.cpp")
    link+=("$RUST/target/$profile/libhpl2_rs.a" -lpthread -ldl -lm)
  fi
  "$CXX" -std=c++17 -O2 -w "${defs[@]}" -I"$core/include" -I"$RUST/crates/hpl2-ffi/include" \
    "${sources[@]}" "${link[@]}" -o "$dir/diff" >"$dir/compile.log" 2>&1 || { cat "$dir/compile.log"; exit 1; }
  NAV_DIFF_TMP="$dir/save" "$dir/diff" "$OUT/$variant.txt"
  echo "$variant: $(wc -l < "$OUT/$variant.txt") lines"
done

reference=$OUT/original.txt
[ -f "$reference" ] || { echo 'Run original (or all variants) first.' >&2; exit 1; }
for variant in "${variants[@]}"; do
  [ "$variant" != original ] || continue
  if cmp -s "$reference" "$OUT/$variant.txt"; then
    echo "IDENTICAL: original == $variant"
  else
    echo "DIFFERENT: original != $variant"
    diff -u "$reference" "$OUT/$variant.txt" | head -40 || true
    exit 1
  fi
done
[ "$VARIANT" != all ] || echo '4-way identical'
