#!/usr/bin/env bash
# Linux/WSL companion to run.ps1, using the same public C++ API harness.
# Usage: run.sh <repo-root> [<pristine-root>]
# VARIANT=rust_release SKIP_CARGO=1 run.sh ... reuses an existing library.
set -euo pipefail
REPO=$(cd "${1:?repo root}" && pwd)
PRISTINE=$(cd "${2:-$REPO/rust/target/ai-placement-pristine}" && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=$REPO/rust/target/ai-placement-diff
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
  sources=("$core/sources/ai/AINodeGenerator.cpp" "$core/sources/graphics/Color.cpp" "$core/sources/impl/tinyXML/tinyxml.cpp"
    "$core/sources/impl/tinyXML/tinyxmlparser.cpp" "$core/sources/impl/tinyXML/tinyxmlerror.cpp" "$HERE/stubs.cpp" "$HERE/diff_main.cpp")
  # Earlier AI ports already exist in the pristine commit: select native bodies.
  if [[ "$variant" = original || "$variant" = cpp_switch_off ]]; then defs+=(-DHPL2_RUST_AI=0); fi
  if [[ "$variant" = rust_* ]]; then
    profile=${variant#rust_}
    cargoargs=()
    [ "$profile" != release ] || cargoargs+=(--release)
    if [ "${SKIP_CARGO:-0}" != 1 ]; then (cd "$RUST" && cargo build -q --locked -p hpl2-ffi "${cargoargs[@]}"); fi
    defs+=(-DHPL2_RUST_AI=1 -DHARNESS_RUST=1)
    sources+=("$REPO/HPL2/core/sources/system/RustBridge.cpp")
    link+=("$RUST/target/$profile/libhpl2_rs.a" -lpthread -ldl -lm)
  fi
  "$CXX" -std=c++17 -O2 -w -ffunction-sections -fdata-sections -Wl,--gc-sections "${defs[@]}" -I"$core/include" -I"$RUST/crates/hpl2-ffi/include" \
    "${sources[@]}" "${link[@]}" -o "$dir/diff" >"$dir/compile.log" 2>&1 || { cat "$dir/compile.log"; exit 1; }
  "$dir/diff" "$OUT/$variant.txt"
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
    diff -u "$reference" "$OUT/$variant.txt" | head -30 || true
    exit 1
  fi
done
[ "$VARIANT" != all ] || echo '4-way identical'
