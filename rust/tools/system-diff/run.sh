#!/usr/bin/env bash
# Differential test: original C++ cString / cPreprocessParser vs. the Rust
# port (and vs. the C++ path with the switch turned off).
# Usage: run.sh <repo-root> [<pristine-root>]
set -euo pipefail

REPO=${1:?repo root}
PRISTINE=${2:-}
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=$REPO/rust/target/system-diff
mkdir -p "$OUT"

CXX=${CXX:-g++}
CXXFLAGS="-std=c++17 -O2 -w"
CORE=$REPO/HPL2/core
RUST=$REPO/rust
RUST_INC=$RUST/crates/hpl2-ffi/include
COMMON="$CORE/sources/graphics/Color.cpp $CORE/sources/math/MathTypes.cpp $HERE/stubs.cpp"

build() { # name defines "sources" includeroots... -- extra link inputs
  local name=$1 defs=$2 srcs=$3; shift 3
  local incs=() objs=()
  while [ $# -gt 0 ] && [ "$1" != "--" ]; do incs+=("-I$1"); shift; done
  [ $# -gt 0 ] && shift
  local dir=$OUT/$name; rm -rf "$dir"; mkdir -p "$dir"
  for src in $srcs $COMMON $HERE/diff_main.cpp; do
    $CXX $CXXFLAGS $defs "${incs[@]}" -I"$HERE/sdl-stub" -c "$src" -o "$dir/$(basename "$src" .cpp).o"
  done
  for extra in "$@"; do objs+=("$extra"); done
  $CXX "$dir"/*.o "${objs[@]}" -lpthread -ldl -lm -o "$dir/diff"
  "$dir/diff" "$OUT/$name.txt"
  echo "$name: $(wc -l < "$OUT/$name.txt") lines"
}

(cd "$RUST" && cargo build -q -p hpl2-ffi --release && cargo build -q -p hpl2-ffi)

SYS=$CORE/sources/system
if [ -n "$PRISTINE" ]; then
  P=$PRISTINE/HPL2/core
  build original "" "$P/sources/system/String.cpp $P/sources/system/PreprocessParser.cpp" "$P/include" "$CORE/include"
fi
build cpp_switch_off "-DHPL2_RUST_SYSTEM=0" "$SYS/String.cpp $SYS/PreprocessParser.cpp" "$CORE/include" "$RUST_INC"

RUST_SRCS="$SYS/String.cpp $SYS/StringRust.cpp $SYS/PreprocessParser.cpp $SYS/RustBridge.cpp"
build rust_release "-DHPL2_RUST_SYSTEM=1 -DHARNESS_RUST=1" "$RUST_SRCS" "$CORE/include" "$RUST_INC" -- "$RUST/target/release/libhpl2_rs.a"
build rust_debug "-DHPL2_RUST_SYSTEM=1 -DHARNESS_RUST=1" "$RUST_SRCS" "$CORE/include" "$RUST_INC" -- "$RUST/target/debug/libhpl2_rs.a"

REF=$OUT/cpp_switch_off.txt
[ -n "$PRISTINE" ] && REF=$OUT/original.txt
status=0
for other in cpp_switch_off rust_release rust_debug; do
  if cmp -s "$REF" "$OUT/$other.txt"; then
    echo "IDENTICAL: $(basename "$REF" .txt) == $other"
  else
    echo "DIFFERENT: $(basename "$REF" .txt) != $other"
    diff "$REF" "$OUT/$other.txt" | head -20
    status=1
  fi
done
exit $status
