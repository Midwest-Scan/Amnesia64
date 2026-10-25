#!/usr/bin/env bash
# Differential test: original C++ math vs. Rust port (and vs. the C++ path
# with the switch turned off). Usage: run.sh <repo-root> [<pristine-root>]
set -euo pipefail

REPO=${1:?repo root}
PRISTINE=${2:-}
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=$REPO/rust/target/math-diff
mkdir -p "$OUT"

CXX=${CXX:-g++}
CXXFLAGS="-std=c++17 -O2 -w"
CORE=$REPO/HPL2/core
RUST=$REPO/rust
MATH_SRC="Math Quaternion Frustum BoundingVolume CRC MathTypes"

build() { # name defines srcroot includeroots... -- extra objects
  local name=$1 defs=$2 srcroot=$3; shift 3
  local incs=() objs=()
  while [ $# -gt 0 ] && [ "$1" != "--" ]; do incs+=("-I$1"); shift; done
  [ $# -gt 0 ] && shift
  local dir=$OUT/$name; mkdir -p "$dir"
  for f in $MATH_SRC; do
    local src=$srcroot/sources/math/$f.cpp
    [ -f "$src" ] || src=$CORE/sources/math/$f.cpp
    $CXX $CXXFLAGS $defs "${incs[@]}" -c "$src" -o "$dir/$f.o"
  done
  $CXX $CXXFLAGS $defs "${incs[@]}" -c "$CORE/sources/graphics/Color.cpp" -o "$dir/Color.o"
  $CXX $CXXFLAGS $defs "${incs[@]}" -c "$HERE/stubs.cpp" -o "$dir/stubs.o"
  $CXX $CXXFLAGS $defs "${incs[@]}" -c "$HERE/diff_main.cpp" -o "$dir/main.o"
  for extra in "$@"; do objs+=("$extra"); done
  $CXX "$dir"/*.o "${objs[@]}" -lpthread -ldl -lm -o "$dir/diff"
  "$dir/diff" "$OUT/$name.txt"
  echo "$name: $(wc -l < "$OUT/$name.txt") lines"
}

RUST_INC=$RUST/crates/hpl2-ffi/include
(cd "$RUST" && cargo build -q -p hpl2-ffi --release && cargo build -q -p hpl2-ffi)

if [ -n "$PRISTINE" ]; then
  build original "" "$PRISTINE/HPL2/core" "$PRISTINE/HPL2/core/include" "$CORE/include"
fi
build cpp_switch_off "-DHPL2_RUST_MATH=0" "$CORE" "$CORE/include" "$RUST_INC" -- "$RUST/target/release/libhpl2_rs.a"

# Rust path: add MathRust.cpp
MATH_SRC="$MATH_SRC MathRust"
build rust_release "-DHPL2_RUST_MATH=1" "$CORE" "$CORE/include" "$RUST_INC" -- "$RUST/target/release/libhpl2_rs.a"
build rust_debug "-DHPL2_RUST_MATH=1" "$CORE" "$CORE/include" "$RUST_INC" -- "$RUST/target/debug/libhpl2_rs.a"

REF=$OUT/cpp_switch_off.txt
[ -n "$PRISTINE" ] && REF=$OUT/original.txt
status=0
for other in cpp_switch_off rust_release rust_debug; do
  if cmp -s "$REF" "$OUT/$other.txt"; then
    echo "IDENTICAL: $(basename "$REF" .txt) == $other"
  else
    echo "DIFFERENT: $(basename "$REF" .txt) != $other"
    diff <(cut -d: -f1 "$REF") <(cut -d: -f1 "$OUT/$other.txt") > /dev/null || echo "  (line structure differs)"
    diff "$REF" "$OUT/$other.txt" | head -20
    status=1
  fi
done
exit $status
