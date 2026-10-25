# HPL2 / Lux Rust port

Incremental rewrite of the HPL2 engine and the Lux game code in Rust. The
game stays buildable and playable throughout: each C++ module is replaced by
Rust one at a time, behind a switch, and checked against the original before
moving on.

| Module | Status | Switch |
|---|---|---|
| `HPL2/core/*/math` | **Ported** (cMath, cQuaternion, cFrustum, cBoundingVolume, cCRC) | `HPL2_RUST_MATH` |
| `HPL2/core/*/system` | **Partly ported** (cString, cPreprocessParser, cLogicTimer, VideoComp; SHA1 in Rust only) | `HPL2_RUST_SYSTEM` |
| `HPL2/core/*/generate` | **Algorithms ported** (cVoxelMap sizing, writes, debug traversal; C++ allocation/factory shell retained) | `HPL2_RUST_GENERATE` |
| `HPL2/core/*/ai` | **Partly ported** (AStar expansion, minimum selection, cost and heuristic; graph/search ownership and path setup remain C++) | `HPL2_RUST_AI` |
| everything else | C++ | – |

The Lux project lives under `amnesia/src/game/` in this checkout. Editors
remain C++; do not port them. Engine/game CMake files are not used for this
port: integration and build verification use `Amnesia.sln` and the Visual
Studio projects (including the existing OpenAL dependency build wrapper).

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
    hpl-generate/       Voxel allocation arithmetic, flat indexing and live debug traversal
    hpl-ai/             AStar expansion, minimum selection and cost/heuristic calculations
    hpl2-ffi/           The C ABI: builds hpl2_rs.lib and generates include/hpl2_rs.h
  tools/math-diff/      Differential test: original C++ vs. Rust, bit for bit
  tools/system-diff/    The same for cString and cPreprocessParser
  tools/system-core-diff/ LogicTimer and VideoComp; native MSVC and Linux runners
  tools/generate-diff/  VoxelMap public API, real graphics-interface mock, allocation/exception traces
  tools/astar-diff/     AStar public searches, controlled graph/visibility, live search-node traces
```

C++ side:

```
HPL2/core/include/system/RustBridge.h    switches, type conversions, layout checks
HPL2/core/sources/system/RustBridge.cpp  routes Rust log output/panics to Log/Error
HPL2/core/sources/math/MathRust.cpp      cMath implemented by calling Rust
HPL2/core/sources/system/StringRust.cpp  cString implemented by calling Rust
HPL2/core/sources/system/LogicTimer.cpp  per-function Rust forwards, original bodies retained
HPL2/core/sources/system/Platform.cpp    VideoComp forwards; variadic wrappers stay C++
HPL2/core/sources/generate/VoxelMap.cpp per-function forwards; allocation/ownership stay C++
HPL2/core/sources/ai/AStar.cpp         four core forwards; path setup and native sets retained
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
* `HplVoxelRenderHost` reads live C++ members and buffer bytes through
  callbacks. No Rust slice/reference to map storage survives a drawing call:
  a virtual `DrawSphere` may resize the buffer or change later cells, transform
  or the caller's referenced color. The color stays in the C++ context.
  The drawing callback catches C++ exceptions into `std::exception_ptr` and
  returns false; Rust stops immediately, then the C++ wrapper rethrows after
  Rust returns. C++ exceptions must never unwind through an `extern "C"` frame.
* `HplAStarSearchHost` carries opaque search-node, graph-node and edge handles.
  C++ owns and dereferences them; Rust holds no reference into engine state
  across callbacks. `HplAStarOpenHost` walks the existing open set in native
  comparator order. The expansion context catches exceptions from best-node
  transfer, `CanAddNode` and node insertion, returns a stop signal, then
  rethrows after Rust returns. Ordinary rejection and saved exceptions are
  separate results; every nested invocation has its own exception slot.
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

Next candidates: further bounded AI algorithms with host callbacks.
Win32 platform/thread/mutex work, SDL timing, MemoryManager and Container
remain C++. Container is header-only templates plus serializer adapters;
SerializeClass and object hierarchies remain deferred to phase 2. Do not
start phase 2 or add runtime dependencies without the owner's approval.

## VoxelMap fidelity and verification

`hpl-generate` contains the algorithmic part of `generate/VoxelMap.cpp`.
The three exports in `hpl2-ffi/src/generate.rs` serve `SetSize`, `SetVoxel`
and `DebugRender`. `HPL2_RUST_GENERATE` defaults to 1 under `#ifndef`;
setting it to 0 compiles the untouched original bodies.

`SetSize` obtains the allocation count from Rust; deletion, member assignment,
`hplNewArray` and zeroing remain in C++. This preserves memory-manager tracking
and allocation exceptions without an allocation callback unwinding through
Rust. Constructors, destructors, inline properties and the `cGenerate`
factory/lifecycle shell stay in C++; `GenerateTypes` is empty. There are no
new third-party dependencies and no phase-2 object ownership changes.

The original was audited before implementation. Preserved quirks:

* Signed `x*y*z` is evaluated left-to-right before conversion to `size_t`.
  Two negative dimensions can allocate valid positive storage; a negative
  total still becomes the original huge C++ allocation request. Zero-volume
  and same-size resizes still allocate and clear, preserving position/scale.
* `SetVoxel` uses `((z*size.x)*size.y + y*size.x) + x`, without per-axis
  bounds checks. In a 3-by-2-by-1 map, `(-1,1,0)` validly writes byte 2.
  All 256 `char` bit patterns are preserved in the unsigned byte buffer.
* Rendering draws only byte 1, in z/y/x order. It reloads loop bounds and
  data after drawing callbacks, including callbacks that grow or shrink the
  map. Position, scale and referenced color are also read live.
* Centers keep the f32 sequence `(position + coords*voxel_size) + half_size`;
  radius is `voxel_size/2.0f`. Negative/zero/subnormal/infinite/NaN values are
  not clamped. `add_position` explicitly preserves the original MSVC offset
  NaN's sign/payload; otherwise Rust debug and release can choose different
  NaN operands. Quiet and signaling payloads are covered by the native test.
* Drawing exceptions preserve the completed draw prefix, callback edits,
  thrown value and propagation to the C++ caller.

Deviations are confined to original undefined memory/integer operations:
overflowing signed dimension/index intermediates and negative/out-of-buffer
flat indices panic (abort through the engine's Rust panic hook) instead of
overflowing or accessing unrelated memory. They are unit-tested separately.
Allocation failures and null-renderer behavior are not changed.

`tools/generate-diff/run.ps1 <repo> [<pristine-repo>]` runs natively on Windows
x64; `run.sh` is the Linux/WSL companion. The pristine baseline is `8cd7087`,
archived before edits into `rust/target/generate-pristine`; the runner defaults
to that directory. Archive `HPL2/core` from that revision to recreate it.
The real `iLowLevelGraphics` interface is mocked; no engine header or class
layout is replaced. Array allocation/deletion events, logs, float bits,
sphere ordering and exception results are recorded without pointer addresses.

The corpus includes all 256 byte values, 324 float-edge combinations,
2,048 maps with 32 seeded operations each, valid flat-index aliases, zero
and two-negative dimensions, resizing and six renderer callback mutation
cases. It excludes signed overflow, invalid flat indices, allocation-failure
recovery and impractically large allocations/empty loops, with filters
documented in the harness. Native comparison currently yields **196,623
lines, 4-way identical** against pristine, switch-off, Rust release and
Rust debug.

`tools/generate-diff/mutate.ps1 <repo>` plants twelve independent arithmetic,
ordering, byte-selection and callback-state bugs, one at a time. It restores
source bytes and rebuilds/reruns the clean four-way matrix in `finally`.
Run without concurrent builds or source edits; build failures and crashes do
not count as output-mismatch detections. Logs stay in
`rust/target/generate-diff/mutations/`.

Verification for the generate slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **78 passed** (13 generate, 33 math, 27 system,
  5 FFI; 14 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Differential: **196,623 lines, 4-way identical** against `8cd7087`.
* Mutation check: **12/12 caught, 0 equivalent mutants**, followed by
  restored source bytes, rebuilt libraries and another clean four-way pass.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux
  and all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run; in particular, the explicit NaN operand selection
  is verified against MSVC, not an assumed GCC result.

There are no gameplay callers of `CreateVoxelMap`/`SetVoxel` in this checkout,
so ordinary gameplay is only an integration smoke check. The differential
graphics mock is the direct exercise of the new algorithm.

## AStar fidelity and verification

`hpl-ai::astar` ports the core of `cAStarHandler`: `IterateAlgorithm`,
the minimum scan in `GetBestNode`, `Cost` and `Heuristic`. Four exports in
`hpl2-ffi/src/ai.rs` forward these methods under `HPL2_RUST_AI`, defaulting
to 1 under `#ifndef`. At 0 the original bodies compile unchanged.

This is a bounded phase-1 slice. `GetPath` retains direct-path testing,
candidate iterators, visibility queries, resets and path output. `AddOpenNode`
retains allocation, membership checks and insertion; `IsGoalNode` retains
native set lookup. `GetBestNode` transfers the selected node between the
native sets. Constructors, destructors, callbacks and graph objects stay
C++; there are no class-layout or ownership changes, new third-party
dependencies or phase-2 work.

The original was audited before implementation. Preserved quirks:

* Open nodes are never relaxed and closed nodes never reopened. Even a
  duplicate attempt constructs/deletes a temporary native search node.
* The best-node scan keeps C++ pointer order and strict `>` comparisons.
  Equal costs retain the first node; an initial NaN cannot be displaced,
  while a later NaN does not replace a finite best. Native open-set erasure
  still precedes closed-set insertion, including on allocation failure.
* The iteration limit is read each pass, with any negative value unlimited.
  The goal check precedes expansion. Edge count is captured for that node,
  but callback choice, edge destination/distance and parent distance stay
  live. The actual edge handle survives the callback; it is not refetched.
* Cost multiplies the entire traveled distance by `1 + abs(height change)`.
  Subtraction, `fabs(float)`, addition and multiplication retain f32 order.
  Root costs read no positions and preserve the original distance bits.
  Heuristic remains the Euclidean f32 distance calculation. Native MSVC NaN
  operand choices are explicit locally: height wins over distance in Cost,
  and squared-sum NaNs select y, then x, then z. This avoids Rust debug/release
  payload differences without changing shared math code or finite arithmetic.
* Direct-path success retains old search sets and the caller's output list;
  graph paths append goal-to-start. Defined nested searches and callback
  exceptions preserve state changes and the accepted prefix of edges.

Exactly two defensive deviations affect original undefined behavior:
`GetBestNode` on an empty open set and signed iteration-counter overflow
abort through the Rust panic hook. The normal private call contract excludes
the former; finite engine graphs do not reach the latter. Invalidated graph,
edge or parent pointers remain invalid caller input, and are excluded from
the corpus. No defined gameplay behavior is intentionally changed.

`tools/astar-diff/run.ps1 <repo> [<pristine-repo>]` uses native Windows x64
MSVC; `run.sh` is its Linux/WSL companion. The baseline is `e91d65c`, archived
before edits into `rust/target/astar-pristine`. Archive `HPL2/core` from that
revision to recreate it. Four variants exercise the public C++ `GetPath` API.
The controlled container fixture retains real engine class layouts, node and
iterator algorithms, with a single grid and scripted visibility in place of
physics. Nodes occupy a contiguous array so pointer ties are reproducible.
The existing memory-manager observer exposes initialized public search-node
fields for exact cost/distance/parent traces, without private test hooks.

The corpus covers 2,048 seeded graphs with three searches each, cycles,
self-edges, negative weights, limits, no relaxation, reverse path append,
direct-path retained state, callback rejection/replacement, live edge and
distance edits, nested direct/full searches, exceptions and float edge cases.
Float results are printed as bits and logs share the output stream. Invalid
pointer lifetimes and signed overflow are excluded with comments in the
harness. Physics collision and grid construction are outside this slice.
One-shot allocation failures cover new search-node allocation and transfer
to the closed set. The original leaks the selected node if closed-set insertion
throws after open-set erasure; this behavior stays native. The fixture records
that initialized orphan and explicitly cleans it up after the test.

`tools/astar-diff/mutate.ps1 <repo>` plants twelve independent selection,
arithmetic, precision, limit, edge-order and callback-state bugs. It restores
source bytes and rebuilds both Rust profiles in `finally`; build failures and
crashes do not count as detections. Run it without concurrent builds or Rust
edits. Logs and output stay under `rust/target/astar-diff/`.

Verification for the AStar slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **96 passed** (16 AI, 13 generate, 33 math, 27 system,
  7 FFI; 18 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Differential: **294,345 lines, 4-way identical** against `e91d65c`, covering
  6,623 public searches and exact initialized search-node fields.
* Mutation check: **12/12 caught, 0 equivalent mutants**, all actual output
  mismatches, followed by restored sources/libraries and a clean four-way pass.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux and
  all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run. Explicit NaN operand selection is verified against
  native MSVC; the Linux companion makes no claim of an untested GCC match.

Reload the changed Rust project in Visual Studio (or restart VS), then build
Debug and Release x64. In game, load an area with an enemy, let it patrol,
then draw it around a corner or obstacle so its path requires graph nodes;
break line of sight and load a save to exercise repeated searches. This is
the actual `amnesia/src/game/LuxEnemyPathfinder.cpp` caller. In-game testing
remains for the owner.

Recommended next slice: bounded `AINodeContainer` graph/grid helpers, keeping
physics visibility calls and graph ownership in C++. The rest of AStar path
setup and the AI state machine also remain C++ for a later measured slice.

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
