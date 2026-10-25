# HPL2 / Lux Rust port

Incremental rewrite of the HPL2 engine and the Lux game code in Rust. The
game stays buildable and playable throughout: each C++ module is replaced by
Rust one at a time, behind a switch, and checked against the original before
moving on.

| Module | Status | Switch |
|---|---|---|
| `HPL2/core/*/math` | **Ported** (cMath, cQuaternion, cFrustum, cBoundingVolume, cCRC) | `HPL2_RUST_MATH` |
| `HPL2/core/*/system` | **Partly ported** (cString, cPreprocessParser, cLogicTimer, VideoComp; SHA1 in Rust only) | `HPL2_RUST_SYSTEM` |
| everything else | C++ | – |

The Lux project lives under `amnesia/src/game/` in this checkout. Editors
remain C++; do not port them. CMake files are unused: integration and build
verification use `Amnesia.sln` and the Visual Studio projects.

## Building

1. Install Rust with [rustup](https://rustup.rs) (default options: MSVC
   toolchain). The pinned version in `rust-toolchain.toml` is downloaded
   automatically on the first build.
2. Restart Visual Studio, open `Amnesia.sln` and build as usual.

The `HPL2Rust` project in the solution runs `cargo` (via `build.cmd`) before
HPL2 is compiled. The resulting `hpl2_rs.lib` is merged into `HPL2.lib`, so
Lux and the tools need no extra link settings. Output goes to
`rust/target/<triple>/<debug|release>/`. Win32 builds also work; `build.cmd`
installs the `i686-pc-windows-msvc` target on first use.

Rust code can be stepped through in the Visual Studio debugger (PDBs are
generated for Debug and Release).

From a terminal in `rust/`:

```
cargo test --workspace     # unit tests
cargo clippy --workspace --all-targets -- -D warnings   # lints
cargo build -p hpl2-ffi    # what Visual Studio runs
```

## Layout

```
rust/
  HPL2Rust.vcxproj      Visual Studio wrapper project (Makefile type) -> build.cmd
  HPL2Rust.props        Shared MSBuild settings, imported by HPL2, Lux and the tools
  crates/
    hpl-math/           Port of HPL2/core/{include,sources}/math. Pure Rust, no C ABI.
    hpl-system/         Strings, shader preprocessor, SHA1, logic timer, video mode ordering
    hpl2-ffi/           The C ABI: builds hpl2_rs.lib and generates include/hpl2_rs.h
  tools/math-diff/      Differential test: original C++ vs. Rust, bit for bit
  tools/system-diff/    The same for cString and cPreprocessParser
  tools/system-core-diff/ LogicTimer and VideoComp; native MSVC and Linux runners
```

C++ side:

```
HPL2/core/include/system/RustBridge.h    switches, type conversions, layout checks
HPL2/core/sources/system/RustBridge.cpp  routes Rust log output/panics to Log/Error
HPL2/core/sources/math/MathRust.cpp      cMath implemented by calling Rust
HPL2/core/sources/system/StringRust.cpp  cString implemented by calling Rust
HPL2/core/sources/system/LogicTimer.cpp  per-function Rust forwards, original bodies retained
HPL2/core/sources/system/Platform.cpp    VideoComp forwards; variadic wrappers stay C++
```

## How a module is ported

1. **Rust crate** `crates/hpl-<module>`: an idiomatic Rust version of the
   module that keeps the original operation order, so results match the C++
   exactly. Deliberately preserved quirks are commented `Faithful:`.
   List new `.rs` files in `HPL2Rust.vcxproj` (and `.filters`) so they show
   up in Solution Explorer; cargo finds them on its own (VC projects don't
   support wildcards).
2. **C ABI** in `crates/hpl2-ffi/src/<module>.rs`: one `hpl_<module>_*`
   function per C++ function. `build.rs` regenerates `include/hpl2_rs.h`
   with cbindgen on every build.
3. **C++ forwarding**: the C++ functions call the Rust ones when the
   module's switch in `RustBridge.h` is 1. The original bodies stay in place
   under `#else`, so a module can be switched back to C++ in one line to
   compare behaviour in game.
4. **Differential test**: the same test program is built against the
   original C++ and against Rust; outputs must be byte-identical.

FFI conventions:

* Value types (`cVector3f`, `cMatrixf`, ...) have `#[repr(C)]` mirrors
  (`HplVector3f`, ...) with identical layout, checked by `static_assert`s in
  `RustBridge.h` and `const` asserts in Rust, so C++ arrays are passed by
  pointer without copying.
* Small values go by value, matrices and arrays by pointer, outputs through
  nullable pointers written after all inputs are read (C++ may pass the same
  object as input and output).
* Strings go in as views, `HplStr` / `HplWStr` (pointer + length, so
  embedded NULs survive; a null pointer stands for a null `const char*`).
  Results come back through sinks, `HplStrOut` etc. (callback + context),
  which assign, append or push into a C++ container: Rust never hands out
  memory that C++ has to free. `RsStr`, `RsAssign`, `RsAppend` and `RsPush`
  in `RustBridge.h` build them:
  `hpl_string_get_file_name(RsStr(sPath), RsAssign(sName));`
* `wchar_t` is `HplWChar`: 16 bits on Windows, 32 elsewhere, matching the
  engine's `tWString`. `RustBridge_Init` checks both sides agree.
* Callbacks into C++ (e.g. `HplPreprocessHost`) are used where the Rust
  code needs engine state that has not been ported yet.
* `HplLogicTimerState` is a numeric snapshot, **not** a memory view of
  `cLogicTimer`. `ToRustState`/`FromRustState` copy its five numeric members;
  the C++ class layout and unused low-level-system pointer stay unchanged.
  Its Rust mirror and C ABI layout are checked on both sides. The
  `HplLogicTimerHost` clock callback converts the native application time
  to double in C++; Rust invokes it only where the original reads time.
* Enums cross as `int32_t`. C++ bool members that might be uninitialised go
  through `RsBool`.
* All engine Rust code is exported from the single `hpl2-ffi` crate: a
  program can only link one Rust static library.
* `panic = "abort"`; a panic is reported through `hpl::Error` first.

Header-only C++ code (the `cVector2/3`, `cMatrix` templates and inline
`cMath` helpers) has Rust equivalents but stays C++ for C++ callers until
those callers are ported, since a call across the language boundary cannot
be inlined.

## Differential test

`tools/math-diff/run.sh <repo> [<pristine-repo>]` (Linux or WSL, needs g++
and cargo) builds the math module four ways and compares the outputs:
original sources (from a checkout of the commit before the port, e.g.
`git worktree add ../pristine <commit>`), switch off, Rust release and Rust
debug. `tools/system-diff/run.sh` does the same for the system module; it
also compares the error and warning messages.

These tests run on Linux, where `wchar_t` is 32 bits and the C library is
glibc. The Windows-only behaviour (16-bit `wchar_t`, UCRT `atoi` overflow
and NaN spelling, the `GetHash` reads) is covered by unit tests with
reference values instead.

`tools/system-core-diff/run.ps1 <repo> [<pristine-repo>]` runs the new
timer/comparator test natively on Windows x64 using MSVC and the pinned
Rust toolchain. `run.sh` is its Linux/WSL companion. Both require pristine
sources and always compare four variants; the default pristine path is
`rust/target/system-core-pristine`. For this port the baseline is `5cc7a98`.
A read-only `git archive` of that revision's `HPL2/core` is sufficient;
no checkout switch or changes to the owner's branch are needed.

The harness prints all observable timer state, floating-point bits, clock
read counts and log messages, and uses the actual C++ video-mode constructors.
It covers 180 fixed rate/speed combinations, 96,000 mixed timer operations,
13,296 comparator pairs and 4,800 stable-sort entries. Signed counter
overflow is excluded, and `GetUpdatesPerSec` is skipped when its C++ cast
would be undefined. Zero/negative rates, clock wrap, signed zero, subnormal,
infinite and NaN speeds are included.

For a focused rerun use `run.ps1 <repo> -Variant rust_release`; `-SkipCargo`
reuses the Rust library. `mutate.ps1 <repo>` plants twelve independent
arithmetic, boundary and ordering bugs and restores source bytes in `finally`.
Run it without other builds or source edits in progress. Mutation build
failures do not count as detections. Logs and outputs stay under
`rust/target/system-core-diff/`.

Verification for this slice (Windows x64, Rust 1.97.0, MSVC):

* `cargo test --workspace --locked`: **64 passed** (33 math, 27 system,
  4 FFI; 11 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Native differential test: **142,264 lines, 4-way identical** against
  pristine `5cc7a98`, switch-off C++, Rust release and Rust debug.
* Mutation check: **12/12 caught, 0 equivalent mutants**, followed by
  restored source bytes, rebuilt libraries and another clean 4-way pass.
* Full `Amnesia.sln` MSBuild: **Debug x64 and Release x64 passed**, including
  Lux and all six editor/tool projects. Initial restricted-process runs
  stalled or hit MSVC C1902 (debug-symbol manager); normal process access
  resolved this without changing project/compiler settings.
* Linux/WSL companion was not run in this pass (WSL access unavailable).
* In-game testing remains for the owner: after reloading the VS projects,
  build both x64 configurations, walk/interact, pause/resume and load a save;
  inspect monitor/resolution ordering in Graphics options. If using the
  debug menu, toggle fast-forward to exercise the speed multiplier.

## LogicTimer and Platform fidelity

The original was audited before this slice was ported. Preserved quirks:

* The constructor's update cap is integer `updates_per_sec / 10`.
  `SetUpdatesPerSec` resets time but leaves the cap and count unchanged.
* Every `WantUpdate` increments the count, including unsuccessful checks.
  Exceeding the cap skips the clock callback; time comparison is strict `<`.
* `EndUpdateLoop` resets time only when count is strictly greater than the
  cap, then clears the count. `Reset` alone does not clear the count.
* Time and speed arithmetic remain double precision. `GetStepSize` narrows
  the increment to float **before** dividing by `1000.0f`, ignores speed,
  and `GetUpdatesPerSec` truncates the reciprocal (60 may report 59).
* Zero/negative rates and zero/negative/nonfinite speeds are not clamped.
  Application-time wrap is not corrected.
* Video modes sort by display, width, then height, ignoring color depth,
  refresh rate and the custom flag.

There are exactly two defined replacements for original undefined behavior:
signed update-counter overflow wraps; nonfinite/out-of-range rate-to-int
conversion returns `INT_MIN` using the existing `f64_to_i32` helper. Both
are marked `Deviation:` and unit-tested separately from the differential
corpus. No defined gameplay behavior was intentionally changed.

The C++ destructor and inline getters/`SetSpeedMul` stay in C++, as do the
message-box `va_list` wrappers. No new module switch, crate or dependency
was needed; the existing `HPL2_RUST_SYSTEM` switch controls this slice.

Next candidates: `generate/VoxelMap` fits phase 1 with C++ buffer ownership
and drawing callbacks; then carefully bounded AStar host callbacks.
Win32 platform/thread/mutex work, SDL timing, MemoryManager and Container
remain C++. Container is header-only templates plus serializer adapters;
SerializeClass and object hierarchies remain deferred to phase 2. Do not
start phase 2 or add runtime dependencies without the owner's approval.

## Troubleshooting

* **Changed project files / missing `hpl2_rs.lib` after edits**: accept VS's
  project reload prompt or restart VS so it reloads `HPL2Rust.vcxproj` and
  its explicit file lists. `HplRustCheckLib` reports a missing Rust library.

* **`error HPLRUST2: cargo was not found`**: install Rust and restart
  Visual Studio.
* **Unresolved external symbols from the Rust standard library** (e.g.
  `__imp_...` from a Windows API): the needed system libraries are listed
  with `#pragma comment(lib, ...)` in `RustBridge.h`. Run
  `cargo rustc -p hpl2-ffi --crate-type staticlib -- --print native-static-libs`
  and add any missing entry there.
* **`LNK4098: defaultlib 'msvcrt.lib' conflicts`** in a Debug executable:
  that project does not import `HPL2Rust.props`; add
  `<Import Project="...\rust\HPL2Rust.props" />` as in `Lux.vcxproj`.
