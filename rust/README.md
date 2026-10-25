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
| `HPL2/core/*/ai` | **Navigation owned by Rust** (phase 2: cAINodeContainer, cAINode, cAINodeIterator, cAStarHandler are C++ facades over Rust data; physics and .nodes file I/O stay C++). **State machine and node generator partly ported** (phase 1: state transitions/dispatch/scheduling, node placement; objects stay C++) | `HPL2_RUST_AI` |
| everything else | C++ | – |

Switches live in `HPL2/core/include/system/RustConfig.h`. Phase 2 (Rust owns
a class's data behind a C++ facade) is described in [PHASE2.md](PHASE2.md).

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
    hpl-ai/             Navigation graph and A* (nav.rs, Rust-owned) and the algorithms it runs; state machine and node placement logic
    hpl2-ffi/           The C ABI: builds hpl2_rs.lib and generates include/hpl2_rs.h
  tools/math-diff/      Differential test: original C++ vs. Rust, bit for bit
  tools/system-diff/    The same for cString and cPreprocessParser
  tools/system-core-diff/ LogicTimer and VideoComp; native MSVC and Linux runners
  tools/generate-diff/  VoxelMap public API, real graphics-interface mock, allocation/exception traces
  tools/ai-nav-diff/    Navigation through the public API: Compile, iterators, GetPath, save/load, live callbacks
  tools/ai-state-diff/   Real StateMachine with derived callbacks, reentry, live IDs and exception traces
  tools/ai-placement-diff/ Real Generate/OnIntersect, body bounds, controlled rays, live parameters and node lists
  tools/ai-scheduler-diff/ Real state scheduling, live steps/counters, reentry, exceptions and remainder probes
```

C++ side:

```
HPL2/core/include/system/RustConfig.h    module switches (macros only; included by public headers)
HPL2/core/include/system/RustBridge.h    type conversions, string helpers, layout checks
HPL2/core/sources/system/RustBridge.cpp  routes Rust log output/panics to Log/Error
HPL2/core/sources/math/MathRust.cpp      cMath implemented by calling Rust
HPL2/core/sources/system/StringRust.cpp  cString implemented by calling Rust
HPL2/core/sources/system/LogicTimer.cpp  per-function Rust forwards, original bodies retained
HPL2/core/sources/system/Platform.cpp    VideoComp forwards; variadic wrappers stay C++
HPL2/core/sources/generate/VoxelMap.cpp per-function forwards; allocation/ownership stay C++
HPL2/core/sources/ai/AStarRust.cpp     cAStarHandler facade (phase 2); AStar.cpp keeps the original
HPL2/core/sources/ai/AINodeContainerRust.cpp cAINode/cAINodeIterator/cAINodeContainer facades (phase 2);
                                         AINodeContainer.cpp keeps the original, the ray callback and FreePath
HPL2/core/sources/ai/StateMachine.cpp  transition/dispatch/scheduler forwards; native timer storage, registration and virtual states retained
HPL2/core/sources/ai/AINodeGenerator.cpp placement/intersection forwards; physics, lists and file caching retained
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
   module's switch in `RustConfig.h` is 1. The original bodies stay in place
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
* **Phase 2 (Rust-owned classes)**: see [PHASE2.md](PHASE2.md). Public headers
  include only `system/RustConfig.h` and forward-declare opaque Rust types
  (`HplAINavGraph`, `HplAINode`, `HplAStar`); the original members stay under
  `#else`. Child objects handed to C++ by pointer (`cAINode*`) remain C++
  facades allocated where the original allocated them, and serve as opaque
  handles in Rust, including for the original's pointer-order tie breaks.
  Data C++ reads in place (positions, `cAINodeEdge` arrays) is `#[repr(C)]`
  in Rust with layout checks on both sides. Callbacks (`HplAINavCompileHost`,
  `HplAStarPathHost`) are used only for unported dependencies: physics,
  `iAStarCallback`, `std::sort` and the output list.
* The bullets below on `HplAStarSearchHost`, `HplAINodeGridHost`,
  `HplAINodeCompileHost` and `HplAStarPathHost` describe phase 1 bridges that
  phase 2 replaced; they are kept as a record of the audited behaviour.
* `HplAStarSearchHost` carries opaque search-node, graph-node and edge handles.
  C++ owns and dereferences them; Rust holds no reference into engine state
  across callbacks. `HplAStarOpenHost` walks the existing open set in native
  comparator order. The expansion context catches exceptions from best-node
  transfer, `CanAddNode` and node insertion, returns a stop signal, then
  rethrows after Rust returns. Ordinary rejection and saved exceptions are
  separate results; every nested invocation has its own exception slot.
* `HplAINodeGridHost` uses the same live-handle/exception-stop pattern for
  grid construction. `HplAINodeGridDimensions` mirrors an initialized numeric
  snapshot (minimum, maximum, map size), not a C++ object. Cell size is omitted
  because it is assigned only after native resize succeeds. Rust/C++ size,
  alignment and field offsets are checked. Native resize/list append retain
  ownership and allocation behavior; no node slice survives those calls.
* `HplAINodeCompileHost` applies that pattern to one source node at a time.
  The single `hpl_ai_node_compile` export controls candidate filtering and both
  trimming passes; C++ retains outer node iteration, candidate iterator
  construction, FreePath, AddEdge, native sorting and vector resize. Null
  candidate handles mean exhaustion. Visibility returns success separately
  from its boolean result, so blocked paths and saved exceptions stay distinct.
  Each source invocation owns its exception slot, including nested calls.
* `HplAIStateTransitionHost` and `HplAIStateUpdateHost` use opaque native state
  handles with the same per-invocation exception pattern. Rust controls leave,
  enter and final assignment order and machine-level update gating. Warning,
  enter, leave and update callbacks catch native exceptions; no C++ object or
  vtable is mirrored. The selected state's scheduler receives the original f32
  time value unchanged. Initial null-current ChangeState aborts (original UB);
  null current during machine Update remains a normal no-op.
* `HplAIStateScheduleHost` reads/writes the selected state's live native timer
  through callbacks. Rust publishes elapsed time before the first tick and
  rereads both counter and step after each virtual OnUpdate. The bridge holds
  no Rust reference into timer storage across reentry; every nested invocation
  has its own exception slot. A failed callback skips subtraction and rethrows
  only after Rust returns. No state/vtable mirror or timer snapshot is added.
* `HplAStarPathHost` keeps original start/goal references in its native context,
  rather than snapshotting them at FFI entry. Endpoint selectors are int32_t:
  0=start and 1=goal. Native iterators are constructed lazily in aligned stack
  storage with placement new and destroyed goal-before-start; this preserves
  lifetime without allocation or a new C++ language setting. Output callbacks
  receive a search handle and call native `push_back(search->mpAINode)`, retaining
  its field reference through allocation. Rust reads the parent only afterward.
  The host shares the existing per-call exception-stop/rethrow convention.
* `HplAINodePlacementHost` keeps the original physics world and parameter argument
  alongside the live generator. Bounds use two native virtual reads per body;
  grid steps read the original argument while wall thresholds use live state.
  Wall callbacks take an opaque node plus an offset, preserving the native
  position reference passed to CastRay and the shared collision callback.
  Node iteration advances only after all four rays, so appended nodes remain
  visible. `HplAINodeIntersectionHost` reads virtual mass before point/height;
  native append retains temporary construction, allocation and the subsequent
  live ID increment. Both hosts use per-call exception-stop/rethrow slots.
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
no checkout switch or changes to the owner's branch are needed. Also archive
its `rust/crates/hpl2-ffi/include`: the original variant of each harness uses
the baseline's generated header when present, because later ports remove
types from the current one (e.g. `git archive 9f35c56 HPL2/core
rust/crates/hpl2-ffi/include | tar x -C <pristine>`).

The Linux companions (`run.sh`) of all harnesses were first run on
2026-10-25 and match 4-way, except that the retired `astar-diff` and
`ai-grid-diff` differed in NaN payload bits only: which NaN an operation on
two NaNs returns is chosen by the C++ compiler (g++ here, MSVC on Windows),
and the Rust side reproduces MSVC. The AI harnesses' scripts now refer to
`impl/tinyXml` with its real case, which Linux requires.

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

## AI state transition slice

`hpl-ai/src/state_machine.rs` ports `cStateMachine::ChangeState` and the active/
current-state dispatch in `cStateMachine::Update`. Two exports in
`hpl2-ffi/src/ai_state.rs` use the existing `HPL2_RUST_AI` switch. Both complete
original bodies remain under `#else`; removing the new branches and bridge
includes reconstructs the original source byte-for-byte.

State objects, vtables, map ownership, registration, getters and lifecycle
remain native. This slice originally left `iAIState::Update`'s scheduler native;
the later scheduler slice below ports its loop. `Sleep` remains its original
no-op. No object layout, dependency or phase-2 ownership change is
involved. A repository-wide call-site audit found no users/subclasses of this
HPL2 state machine in the engine or Lux; Lux's player/enemy state systems are
separate. This port therefore has direct harness validation but no specific
in-game route that exercises the class.

The original was audited before implementation. Preserved quirks:

* Self-transition suppression compares the requested ID against the current
  object's live ID before lookup. Registration map keys and object IDs may
  differ; inactive machines still permit transitions.
* The target is resolved once. Leave receives its live ID. Enter receives the
  current state's ID reread after leave, including changes from a nested
  transition or ID mutation. Missing targets retain the exact native warning.
* The captured target becomes current only after enter succeeds, overwriting
  a nested transition performed by enter. Entry callbacks see the earlier
  current state. Exceptions stop without rollback or final assignment; callback
  changes already made remain visible. Warning failures are caught too.
* Each invocation has its own native exception slot. A caught inner exception
  cannot poison an outer transition. No exception unwinds through Rust.
* Machine Update tests active first, then current, and dispatches the selected
  state once. The scheduler may keep draining that state's backlog after
  callbacks switch the machine's current state or deactivate it. Its live-step,
  reentrant-counter and float behavior remain unchanged.
* Native registration still selects the first state without enter notification,
  preserves accumulated timing on re-registration, and leaves a failed duplicate
  insertion's unowned object to the caller. No timer or ownership cleanup is added.

**Deviation:** ChangeState with no current state explicitly aborts through the
Rust panic diagnostic. The original immediately dereferences null. Invalidated
objects and uninitialized state fields remain invalid caller input. Potentially
nonterminating fixed-step loops retain their original behavior; zero/negative steps are not
blanket-rejected because callbacks can change them and make the original finish.

`tools/ai-state-diff/run.ps1 <repo> [<pristine-repo>]` runs on native Windows
x64; `run.sh` is its Linux companion. Baseline `dbf8b58` was archived before edits
to `rust/target/ai-state-pristine`; archive `HPL2/core` from that commit to recreate
it. Original and current switch-off use `HPL2_RUST_AI=0`; Rust release/debug use 1.
The harness links actual StateMachine.cpp and uses ordinary concrete iAIState
subclasses with legal protected-member setters. It adds no private test hooks
and replaces no transition, dispatch, scheduling or registration algorithm.

The corpus runs 4,147 machines (4,096 seeded), 46,090 ChangeState calls,
27,007 Update calls, 73,147 virtual callbacks and 1,456 controlled exceptions.
It covers live IDs differing from map keys, missing and same-state transitions,
inactive transitions, nested
leave/enter/update calls, callback changes to activity/current/steps, and failures
from warning/leave/enter/update. Callback traces include float bits, IDs, current
state and activity; public follow-up updates observe retained timer remainders.
Duplicate registration preserves valid single ownership. Null-current ChangeState,
state deletion during callbacks and unbounded loops are excluded with comments.
Special zero/negative/infinite steps run only when a callback repairs them or
native arithmetic makes the loop terminate.

`tools/ai-state-diff/mutate.ps1 <repo>` plants realistic transition/dispatch bugs
one at a time, requires actual output mismatches, restores exact source bytes and
rebuilds both Rust profiles in `finally`. Run it without concurrent Rust edits
or builds. Logs stay under `rust/target/ai-state-diff/`.

Verification for this slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **152 passed** (61 AI, 13 generate, 33 math, 27 system,
  18 FFI; 20 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Differential: **789,159 lines, 4-way identical** against `dbf8b58`.
* Mutation check: **12/12 caught, 0 equivalent mutants**, all actual output
  mismatches, followed by exact source restoration, both Rust library rebuilds
  and a clean four-way comparison. Mutants cover self suppression, missing
  warnings, target/previous IDs, leave omission, stale IDs, exception stopping,
  early/missing assignment, inverted active gating and dropped elapsed time.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux and
  all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run. The Linux runner is supplied without claiming a pass.
  Unit tests/Clippy used process-only `CARGO_INCREMENTAL=0`; no build settings
  or toolchain pin changed. Source and project line endings were preserved;
  the generated header was restored to CRLF after the final builds.

Reload the Rust project in Visual Studio (or restart VS), then build Debug and
Release x64. Load a save, switch between normal movement and an interaction such
as a ladder or carried object, and observe enemy patrol/pursuit as a general
regression check. Those Lux systems do not call this HPL2 class; the public-API
differential harness directly validates the new code. Gameplay validation remains
for the owner.

## AI node generation placement slice

`hpl-ai/src/node_generator.rs` ports the placement portion of
`cAINodeGenerator::Generate` and the mass/height decisions of `OnIntersect`.
`hpl2-ffi/src/ai_placement.rs` adds `hpl_ai_node_generator_place` and
`hpl_ai_node_generator_intersect` under the existing `HPL2_RUST_AI` switch.
Both original source regions remain intact under `#else`; removing the forwards
and their added includes reconstructs the original file byte-for-byte.

Native C++ retains initialization, cache lookup/load/save, body and node iterators,
physics calls, the shared wall-ray callback, node construction/allocation and ID
increments. No class layout, language standard, dependency or phase-2 ownership
change is introduced. File serialization quirks remain unchanged, including the
duplicate `Name` attribute in SaveToFile and returning after a failed cache load.

The original was audited before implementation. Preserved quirks:

* Every body contributes bounds, including dynamic bodies. Each body makes two
  virtual GetBoundingVolume calls, copying minimum before fetching maximum.
  Ordered comparisons retain equal signed zeros, ignore NaNs and preserve the
  original +/-100000 sentinels for empty/unusual worlds.
* Bounds shrink only in x/z, then clamp each component independently. They are
  never normalized. Only z gates grid entry; an inverted x range still casts
  one ray per row. Traversal is x first, then z, with f32 arithmetic throughout.
* The physics world is captured once. Bounds use live generator parameters;
  grid increments reread the original parameter argument after each ray, even
  after a nested Generate changes the generator's parameter pointer. No blanket
  guard rejects steps that a later callback can repair.
* Existing nodes are never cleared, and IDs restart at zero. All old and new
  nodes receive wall checks. Node iteration advances after four rays, so list
  appends during those rays are visited when the native iterator would see them.
* Wall offsets snapshot distance after placement, in +X/-X/+Z/-Z order.
  Thresholds and positions remain live after each ray. Native CastRay receives
  the node's position by reference and retains the shared collision callback,
  including nested calls overwriting its result.
* Pushback uses a full three-component multiply and add, retaining zero-times-
  infinity/NaN and signed-zero behavior. Intersection placement likewise adds
  zero to x/z rather than copying those components unchanged.
* Virtual GetMass runs before reading point/height. Either signed zero accepts;
  nonzero, infinite and NaN masses skip. Native append captures the ID before
  allocation and increments the live counter only after successful insertion.
* Native exceptions stop immediately and rethrow after Rust returns, preserving
  partial nodes, callback changes and counter state. No rollback is added.

**No new deviations.** Uninitialized generator use, invalidated pointers or
iterators, switching node lists during iteration, signed ID overflow, concurrent
shared-callback access and unrescued nonterminating grid loops remain outside
the defined input corpus. Cache/XML error paths are outside this placement slice.

`tools/ai-placement-diff/run.ps1 <repo> [<pristine-repo>]` compares original,
switch-off, Rust release and Rust debug on Windows x64. `run.sh` is the Linux
companion. Baseline `1c3821f` was archived before edits to
`rust/target/ai-placement-pristine`; archive `HPL2/core` from that commit to
recreate it. The harness links production AINodeGenerator.cpp, using normally
constructed world/resources objects and concrete physics/body fixtures.
Fixed bounding volumes and controlled rays supply inputs; native iteration,
placement, filtering and node insertion are not replaced. Empty world paths
keep file I/O unreachable. Traces include float bits, node order and IDs,
virtual calls, allocations, exceptions and logs.

The corpus runs 2,098 Generate calls (2,048 seeded cases plus fixed cases),
5,915 vertical rays, 41,026 wall rays, 99,638 intersection callbacks, 8 controlled
exceptions and 3 nested calls. It covers boundaries, retained nodes, live
parameters/positions, repeated and nested generation, zero/negative steps rescued
by callbacks, nonfinite values on terminating paths, allocation failures and
partial output. Tests never invalidate live objects or cross list iterators.
`tools/ai-placement-diff/mutate.ps1 <repo>` requires actual output mismatches,
restores exact source bytes and rebuilds both Rust profiles in `finally`. Run it
without concurrent Rust edits or builds. Logs stay under
`rust/target/ai-placement-diff/`.

Verification for this slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **202 passed** (97 AI, 13 generate, 33 math, 27 system,
  32 FFI; 31 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Differential: **429,618 lines, 4-way identical** against `1c3821f`.
* Mutation check: **12/12 caught, 0 equivalent mutants**, all actual output
  mismatches. The existing finite-value cases catch double-precision wall
  correction through changed intermediate rounding. Exact source bytes were
  restored, both Rust libraries rebuilt and the clean four-way comparison
  repeated successfully.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux and
  all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run. The Linux runner is supplied without claiming a pass.
  Unit tests/Clippy used process-only `CARGO_INCREMENTAL=0`; no toolchain pin,
  project compiler setting or language standard changed. Source/project line
  endings were preserved; the generated header was restored to CRLF after the
  final builds.

Reload the Rust project in Visual Studio (or restart VS), then build Debug and
Release x64. Load a save and observe enemy patrol/pursuit around obstacles as a
general regression check. The only generator caller found is the engine's
`cWorld::GenerateAINodes` wrapper; Lux does not call it, so ordinary gameplay
does not directly validate this slice. The public Generate differential harness
provides that coverage. Gameplay validation remains for the owner.

The AI state scheduler is covered by the following slice.

## AI state scheduler slice

`hpl-ai/src/state_machine.rs` now also ports `iAIState::Update`'s fixed-step loop.
`HplAIStateScheduleHost` and `hpl_ai_state_schedule` in the existing
`hpl2-ffi/src/ai_state.rs` forward it through `HPL2_RUST_AI`. The complete original
body remains under `#else`. Timer fields, state objects/vtables, registration,
ownership, getters and the no-op Sleep stay in C++; no dependency, object layout,
language standard or phase-2 ownership change is introduced.

The original was audited before implementation. Preserved quirks:

* Elapsed time is added in f32 and stored before the first comparison, including
  when no callback runs or a callback later throws. Negative elapsed time is
  retained. Neither registration nor transitions reset the existing remainder.
* The ordered `count >= step` comparison includes equality and rejects NaNs.
  Every successful tick subtracts separately in f32; division, multiplication
  or a retained f64 accumulator would change rounding and sometimes tick counts.
* OnUpdate receives the current step. After it returns, both counter and step
  are reread before subtraction. A callback may change the step or recursively
  update the same state's shared counter. No stale local copy overwrites that
  nested work.
* The outer backlog stays with its selected state even when callbacks change
  current state or activity. Only a new machine-level Update checks those fields.
* Zero, negative, infinite and precision-stagnant steps are not rejected or
  capped: callbacks may make those loops terminate. Same-sign infinite count
  and step can produce one tick then NaN naturally. Sleep remains a no-op.
* Exceptions preserve elapsed accumulation and all completed callback/nested
  effects, skip that tick's subtraction, and stop immediately. Nested calls
  have independent exception slots, so a caught inner failure cannot poison the
  outer invocation. No C++ exception unwinds through Rust.

**No new deviations.** The prior null-current ChangeState deviation is unchanged.
Uninitialized steps, destroyed state objects, concurrent unsynchronized access,
unbounded recursion and nonterminating loops remain outside the defined corpus.
The test fixtures use initialized registered states and bounded callbacks; no
private timer access, layout tricks or production test hooks are introduced.

`tools/ai-scheduler-diff/run.ps1 <repo> [<pristine-repo>]` compares original,
switch-off, Rust release and Rust debug on Windows x64. `run.sh` is its Linux
companion. Baseline `9f35c56` was archived before edits to
`rust/target/ai-scheduler-pristine`; archive `HPL2/core` from that commit to
recreate it. The harness links production StateMachine.cpp and drives ordinary
derived states through public cStateMachine::Update. Callback traces print float
bits, state identity, activity and exceptions; later public updates probe the
otherwise private timer remainder. Registration, scheduling and transition
algorithms are never replaced by test stubs.

The corpus runs 4,183 machines (4,096 seeded), 92,894 public updates, 66,627
callbacks including 33,124 scheduler ticks, 2,226 controlled exceptions and
2,272 nested updates. It covers exact/adjacent and decimal steps, negative time,
live step edits, re-registration, same-state and cross-state reentry, transitions,
activity changes, caught/escaping failures and callback-rescued special steps.
Private NaN payload bits cannot be observed through this public API; the corpus
checks their persistent suppression of callbacks, while unit tests inspect the
pure host's numeric state. An emergency fixture callback budget reports a test
failure and can never count as a detected mutation; the clean corpus hits no
such limit.

`tools/ai-scheduler-diff/mutate.ps1 <repo>` requires actual output mismatches,
restores exact source bytes and rebuilds both Rust profiles in `finally`.
Run it without concurrent Rust edits or builds. Logs stay under
`rust/target/ai-scheduler-diff/`.

Verification for this slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **232 passed** (119 AI, 13 generate, 33 math, 27 system,
  40 FFI; 30 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Scheduler differential: **796,994 lines, 4-way identical** against `9f35c56`.
* Earlier state-transition/dispatch regression: **789,159 lines, 4-way identical**.
* Mutation check: **10/10 final mutations caught, 0 equivalent mutants**, all
  completed output mismatches. Exact source bytes were restored, both Rust
  libraries rebuilt and the clean four-way comparison repeated successfully.
  An initial dropped-time trial exposed a missing catch around a deliberate
  exception in a later public transition probe; that test boundary was repaired.
  A separate overwritten-remainder trial exhausted the callback budget on large
  negative-time/infinity probes and was excluded, not counted as a detection or
  equivalent mutant. The full corpus and budget were retained; the final ten
  mutations were rerun from scratch, including retained-f64 accumulation.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux and
  all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run. The Linux runner is supplied without claiming a pass.
  Unit tests/Clippy used process-only `CARGO_INCREMENTAL=0`; no toolchain pin,
  project compiler setting or language standard changed. Source/project line
  endings were preserved; the generated header was restored to CRLF after the
  final builds.

Reload the Rust project in Visual Studio (or restart VS), then build Debug and
Release x64. Load a save, switch between movement and an interaction such as a
ladder or carried object, and observe enemy patrol/pursuit as a general regression
check. Lux's state systems are separate; no engine/Lux callers or subclasses of
this HPL2 state machine were found, so the public-API differential harness is
the direct validation. Gameplay validation remains for the owner.

Superseded: AINodeContainer and AStar moved to Rust in phase 2 (see below);
the original recommendation was to audit AINodeContainer's FreePath ray setup and filtering,
keeping physics ownership and virtual ray execution native.

## AI navigation: phase 2 (Rust-owned)

`cAINodeContainer`, `cAINode`, `cAINodeIterator` and `cAStarHandler` keep their
public API, but with `HPL2_RUST_AI` at 1 their data lives in Rust
(`hpl-ai/src/nav.rs`, C ABI in `hpl2-ffi/src/ai_nav.rs`). See
[PHASE2.md](PHASE2.md) for the pattern. The C++ classes are facades
(`AINodeContainerRust.cpp`, `AStarRust.cpp`); the originals are untouched in
`AINodeContainer.cpp`/`AStar.cpp` and the headers' `#else` branches.

| Rust | C++ facade |
|---|---|
| Nodes (ID, position, user data, edges), name/ID lookups, grid map, properties, `Compile`, `AddEdge`, iterator cursor, A* open/closed/goal sets, `GetPath` | `cAINode::msName`, container names/world/collide size/node-at-center/ray callback, `FreePath` (physics), `SaveToFile`/`LoadFromFile` (TinyXML), `iAStarCallback*` |

The Rust code runs the algorithms ported and audited in phase 1 (`astar.rs`,
`astar_path.rs`, `node_container.rs`, `node_compile.rs`): their host traits are
now implemented on Rust data. Behaviour kept from the original:

* A* ties break by `cAINode*` address, as the original `std::set` did. The
  facades are allocated by `AddNode` like the original nodes, so the rule and
  its inputs are the same. (On Windows the heap randomizes these addresses, so
  the original tie order was never stable between runs either.)
* Open nodes are never improved, closed nodes never reopened; output is
  appended goal-to-start without clearing the list; a direct free path returns
  early with the list untouched.
* `Compile` sorts edges with the platform's `std::sort` (called back into
  C++ on the `#[repr(C)]` edge array) so tie order matches; rebuilding the
  grid keeps old cell lists (nodes appear twice), as `std::vector::resize` did.
* Callbacks may use the graph as the original allows (move nodes, change
  properties, add nodes/edges, rebuild the grid, compile another container,
  run another search that ends at its direct free-path check). Rust holds no
  reference across a callback and reads every value where the original does.

Deviations (original undefined behaviour only):

* `GetNode` out of range and `cAINodeIterator::Next` past the end return NULL;
  `GetEdge` out of range returns NULL.
* An empty container's `BuildNodeGridMap` (also via `Compile`/`LoadFromFile`)
  leaves the grid empty instead of reading `mvNodes[0]`; iterating an unbuilt
  grid yields nothing.
* `LoadFromFile`: a node ID not in the container is skipped (the original
  dereferences NULL). An edge naming a missing node keeps its NULL target for
  `GetEdge`, and the search skips it (the original would dereference it).
* A nested `GetPath` on the same handler that goes on to reset the search sets
  while the outer search is using them aborts; deleting a container or handler
  during its own operation aborts. (The original frees memory still in use.)
* Where the original's iterators or edge pointers are invalidated by a
  reallocation inside a callback (e.g. `AddNode` during `Compile`), Rust's
  indices stay valid.

Accepted simplification: `GetPath` copies its start and goal at entry (the
original reads the caller's vectors by reference); only a callback that
changes the caller's endpoint variables mid-search could tell the difference,
and no engine or Lux code does.

**Verification** (Linux, g++ and Rust 1.97.0; the owner builds on Windows):

* `cargo test --workspace --locked`: **233 passed** (133 AI incl. 14 for
  `nav.rs`, 13 generate, 33 math, 27 system, 27 FFI). Clippy clean with
  `-D warnings`.
* `tools/ai-nav-diff/run.sh <repo> <pristine>`: **683,543 lines, 4-way
  identical** (original from `e91d65c`, switch off, Rust release, Rust debug).
  The corpus: 1,500 random containers (lattices with many equal distances,
  random layouts, duplicate names/IDs/positions, walls, no physics world),
  1,000 walled lattices where paths need the graph (905 found paths of 3–19
  nodes), and 600 callback scenarios (all actions above, exceptions from
  physics and from `CanAddNode`, `SetMaxIterations`/`SetCallback(NULL)` mid
  search, nested direct searches); plus iterator copies, lookups,
  save/load round trips with renamed nodes and missing files. The harness
  uses a monotonic allocator so node addresses follow insertion order in
  every variant (see `stubs.cpp`). Linux only (it uses `mmap`).
* Mutation check: **12/12 caught** (address order, relaxing open nodes,
  grid rebuild clearing, iterator stepping, compile radius, duplicate lookups,
  stale goals, iterator radius, squared distance, iteration default, start
  costs).
* Regression: math, system, system-core, generate, ai-state, ai-placement and
  ai-scheduler harnesses rerun 4-way identical on Linux.
* Not verified here: the MSVC build and the game. Build Debug and Release x64
  in Visual Studio (reload the projects first) and check enemies in game.

The phase 1 harnesses `astar-diff`, `astar-path-diff`, `ai-grid-diff` and
`ai-compile-diff` were removed: they observed private allocations
(`cAStarNode` via `operator new`, edge vector growth) that phase 2 replaces.

## History: phase 1 AI navigation slices (superseded)

These sections document the phase 1 bridges for AStar and AINodeContainer,
replaced by phase 2 (above). Their C++ hosts, FFI modules (`ai.rs`,
`ai_grid.rs`, `ai_compile.rs`, `ai_path.rs`) and harnesses no longer exist.
The audits remain accurate: phase 2 runs the same Rust algorithm modules.

### AStar fidelity and verification

`hpl-ai::astar` ports the core of `cAStarHandler`: `IterateAlgorithm`,
the minimum scan in `GetBestNode`, `Cost` and `Heuristic`. Four exports in
`hpl2-ffi/src/ai.rs` forward these methods under `HPL2_RUST_AI`, defaulting
to 1 under `#ifndef`. At 0 the original bodies compile unchanged.

This initial phase-1 slice retained `GetPath` in C++; its setup/output decisions
are ported in the later AStar path slice below. `AddOpenNode` still retains
allocation, membership checks and insertion; `IsGoalNode` retains
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

### AI node/grid fidelity and verification

`hpl-ai::node_container` ports five bounded helpers in `AINodeContainer.cpp`:
`cAINode::AddEdge` metrics, `BuildNodeGridMap`, `GetGridPosFromLocal`, `GetGrid`
indexing and `cAINodeIterator::IncGridPos`. Five exports in `ai_grid.rs` use
the existing `HPL2_RUST_AI` switch. Original bodies remain unchanged at 0.

Node allocation, names/ID lookup, edge-vector append, grid-vector resize and
list append stay native. Iterator construction, `HasNext` and `Next` retain
their native list state. This slice left `Compile` filtering/trimming in C++;
the following slice ports those decisions. Native sorting, physics ray queries,
XML I/O and lifecycle remain C++. No class layout, third-party
dependency or phase-2 ownership change is involved.

The original was audited before implementation. Preserved quirks:

* Rebuilding never clears surviving cell lists. Same-size rebuilds append
  duplicates; resizing preserves old lower-index lists even after bounds or
  grid stride change, destroying only removed cells. Node order stays native.
* Bounds fold x/z coordinates with strict comparisons. A NaN in the first
  node stays; later NaNs are ignored. Equal signed zeros retain the first sign.
* Dimension uses f32 node count / nodes per grid, float square root, `+0.5f`,
  truncation, then `+1`. Storage is `(dimension+1)^2`, while cell extent divides
  by dimension. Queries clamp inclusively to the border row/column.
* Bounds and dimensions are written before resize; cell size follows success.
  Allocation failure leaves that exact partial state. Append failures retain
  the completed prefix, stop Rust immediately and rethrow in C++.
* Build insertion divides only when cell extent is strictly positive. Query
  conversion divides unconditionally, truncates, clamps low, then clamps high.
  Flat indexing preserves defined cross-axis aliases; it does not add axis
  checks absent from the original.
* Iterator stepping advances x first, then wraps and advances y. Exhaustion
  leaves coordinates advanced. Negative radii/inverted ranges are not
  normalized, and native `Next` still advances before returning its node.
* Edge metrics store both Euclidean distance and directly computed squared
  distance; squaring the rounded root would change bits. Duplicate and self
  edges remain accepted, with C++ allocation exceptions preserved.
  The native AddEdge call site selects squared-sum NaNs in y/x/z priority;
  this is explicit locally, including payload conflicts with infinity-minus-
  infinity. Shared math code is unchanged and finite arithmetic keeps its order.

Deviations are limited to original undefined operations: empty grid builds,
signed dimension/allocation/index/increment overflow, and out-of-buffer flat
indices abort through the Rust panic hook. Invalid float-to-int casts use
the existing x86 `INT_MIN` helper before the original clamps/indexing; this
includes zero-extent query division. Defined cases are compared separately
from these defensive unit tests. Invalidated graph/list pointers remain
invalid caller input, not a new supported operation.

`tools/ai-grid-diff/run.ps1 <repo> [<pristine-repo>]` runs on native Windows
x64; `run.sh` is its Linux companion. Baseline `890af92` was archived before
edits into `rust/target/ai-grid-pristine`; archive `HPL2/core` from that commit
to recreate it. All graph/grid/node/iterator bodies come from the actual
production source in each variant. TinyXML is linked to satisfy MSVC's unused
references, while unused world/string/platform dependencies have abort-only
stubs. The harness does not replace the algorithm or expose private members.

The corpus exercises 2,664 builds and 42,389 public iterator queries, including
2,048 seeded node sets, plus 2,166 special float edge cases and 4,096 seeded
random edges. It covers public node/edge/build/iterator APIs, repeated builds,
changed positions and growing grids, border and negative-radius queries,
allocation failures and recovery, special float edge metrics, and thousands
of seeded cases. It prints float bits, allocation events, node order and logs.
An allocation callback adds an inside-bounds node during grid resize, checking
that later insertion reads the live node count. Tiny/large finite grids use
bounded query-to-cell ratios so the original casts remain defined.
Empty builds, invalid grid casts/indices and unrepresentable query quotients
are excluded with comments. Degenerate grid builds are tested without making
undefined queries into zero-width axes.

`tools/ai-grid-diff/mutate.ps1 <repo>` plants twelve independent metric,
precision, sizing, border, traversal, live-count and exception-stop bugs. It
restores source bytes and rebuilds both Rust profiles in `finally`. Only real
output mismatches count; compile/runtime failures do not. Run it without
concurrent Rust edits/builds. Logs remain under `rust/target/ai-grid-diff/`.

Verification for this slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **117 passed** (35 AI, 13 generate, 33 math, 27 system,
  9 FFI; 21 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Differential: **1,309,674 lines, 4-way identical** against `890af92`.
* Mutation check: **12/12 caught, 0 equivalent mutants**, all output mismatches,
  followed by restored source bytes/libraries and a clean four-way pass.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux and
  all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run. NaN operand choices are verified against native MSVC;
  the Linux companion does not imply an untested GCC result.

Reload the Rust project in Visual Studio (or restart VS), build Debug and
Release x64, then load an enemy map, watch patrols and draw an enemy around
corners. Reload a save to repeat grid setup. Both cached `LoadFromFile` and
uncached `Compile` build grids; AddEdge also runs during connection rebuilding
and is directly covered by the differential corpus. In-game validation remains
for the owner.

### AI connection compilation slice

`hpl-ai/src/node_compile.rs` ports `AINodeContainer::Compile`'s per-source
candidate filtering, edge-count cap and distance trimming. One export in
`hpl2-ffi/src/ai_compile.rs` uses the existing `HPL2_RUST_AI` switch; the complete
original Compile body is preserved byte-for-byte under `#else`. The C++ shell
still builds the grid and walks the native node vector. It constructs each
candidate iterator with the original `max_distance * 1.5f` radius, then supplies
live opaque-node callbacks to Rust. No new switch, dependency, object layout,
ownership model or phase-2 interop is introduced.

The original was audited before implementation. Preserved quirks:

* Self candidates are skipped before any position reads. Distance rejection is
  strictly `distance > max_distance * 2`, then height acceptance is
  `abs(source.y - candidate.y) <= max_height`, with the original f32 arithmetic.
  NaN distance passes rejection; NaN height fails acceptance.
* Filters read live limits and positions. Visibility retains native FreePath
  with ray count `-1` and SkipDynamic | SkipVolatile. Successful visibility calls
  native AddEdge, recomputing metrics after any callback position changes.
* Existing edges, self edges and duplicates survive. Native unstable std::sort
  preserves platform tie ordering. It runs before the positive maximum cap;
  the minimum edge count cannot override that cap. Nonpositive caps are unlimited.
* Trimming truncates the complete suffix at the first edge strictly beyond the
  live distance limit whose signed index meets the minimum. Exact-distance ties
  remain. Negative minimum counts allow truncation at index zero.
* Count/index narrowing keeps MSVC's signed low 32 bits. Allocation/visibility
  exceptions stop immediately with the completed prefix intact, are caught in
  the per-invocation C++ context, and rethrow only after Rust returns.

**No new deviations.** Prior grid deviations remain as documented above.
Invalidated native iterators and graph pointers remain invalid caller input;
NaN edge distances that violate native sorting's strict-weak-order requirement
are excluded from the native differential corpus. Pure Rust host tests cover
NaN filter decisions and synthetic huge counts without invoking native UB.

`tools/ai-compile-diff/run.ps1 <repo> [<pristine-repo>]` runs on native Windows
x64; `run.sh` is its Linux companion. Baseline `27301fd` was archived before
edits to `rust/target/ai-compile-pristine`; archive `HPL2/core` from that commit
to recreate it. Because earlier AI helpers were already ported at that commit,
the archived reference also compiles with `HPL2_RUST_AI=0`. This exercises its
original native helpers and Compile without linking Rust into the reference.
Current switch-off, Rust release and Rust debug are compared to that output.

The harness uses real production Compile, grid/iterator, AddEdge, FreePath and
sorting. A normally constructed lightweight cWorld and concrete iPhysicsWorld
fixture control ray results; unrelated virtual methods abort if reached.
TinyXML and Color.cpp satisfy native link dependencies. Float bits, edge order,
ray calls, allocations and logs all enter the comparison. Real physics body
prefiltering is unchanged native code and is outside this fixture's coverage.

The corpus makes 2,794 Compile calls and 204,255 ray calls, including 36
controlled exceptions and 2,048 seeded graphs. It includes fixed and adjacent
thresholds, negative/zero limits, pre-existing and duplicate edges, unstable-sort
ties, repeated compilation, absent physics, live position/limit/edge changes,
nested compilation with a caught inner exception, partial failures and recovery.
Safe nonfinite y/height cases exercise comparison behavior without sorting NaNs.
Nonempty grids with finite x/z coordinates, nonzero extents and bounded query
to cell ratios avoid undefined native casts; source comments explain exclusions.

`tools/ai-compile-diff/mutate.ps1 <repo>` plants independent realistic bugs one
at a time, requires actual output mismatches, restores exact source bytes and
rebuilds both Rust profiles in `finally`. Run it without concurrent Rust edits
or builds. Logs stay under `rust/target/ai-compile-diff/`.

Verification for this slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **132 passed** (46 AI, 13 generate, 33 math, 27 system,
  13 FFI; 15 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Differential: **565,569 lines, 4-way identical** against `27301fd`.
* Mutation check: **12/12 caught, 0 equivalent mutants**, all output mismatches,
  followed by exact source restoration, both library rebuilds and a clean
  four-way comparison. Mutants cover self candidates, cutoff multipliers and
  boundaries, signed height, blocked paths, ignored append exceptions, zero
  caps, minimum off-by-one, skipped sort, stale limits and double arithmetic.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux and
  all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run. The Linux runner is supplied without claiming a pass.
  Unit tests/Clippy used process-only `CARGO_INCREMENTAL=0` to avoid restricted
  filesystem hard-link warnings; no build settings or toolchain pin changed.

Reload the Rust project in Visual Studio (or restart VS) after the explicit
file-list additions. For an in-game check, temporarily rename the relevant
`<map>_<container>.nodes` cache, load an enemy map and confirm the log says
`Rebuilding node connections and saving to ...`. Check enemy patrol/pursuit
around obstacles. Normal cached loads bypass Compile. Preserve the original
cache so it can be restored after the check. In-game validation remains for
the owner.

### AStar path setup and output slice

`hpl-ai/src/astar_path.rs` ports `cAStarHandler::GetPath`: the direct-path decision,
fallback sequence, start/goal candidate filtering and result-chain traversal.
`hpl2-ffi/src/ai_path.rs` exports `hpl_astar_get_path` through the existing
`HPL2_RUST_AI` switch. The complete original GetPath body, including its original
comments, remains under `#else`. Removing the new branch and `<new>` include
reconstructs the previous AStar.cpp byte-for-byte.

Graph and search-node ownership, set allocation/order, native reset operations,
AddOpenNode, visibility, native iterator lifetime and list insertion remain C++.
The previous Rust search core still performs the search through its existing
bridge. No new dependency, object layout, C++ language setting or phase-2
ownership change is introduced. The lazy stack holders use C++11 alignment and
placement new; there is no extra heap allocation on either path.

The original was audited before implementation. Preserved quirks:

* Height limit is captured as `maxHeight * 1.5f` at entry. Direct visibility is
  called only when the height comparison passes. Direct success clears only
  the goal pointer and returns true; old search sets, saved goal position and
  caller output survive. The initially uninitialized goal pointer is not read.
* Fallback deletes closed nodes before open nodes, clears the goal set, resets
  the goal pointer, copies the live goal reference, then captures
  `maxDistance * 2`. Native exception side effects remain in that order.
* Original start/goal references stay live across visibility and allocation
  callbacks. The start iterator is created after reset; the goal iterator only
  after all start candidates. Both use the captured radius, but each captures
  its own origin when constructed. Subsequent filters reread live endpoints.
* Candidate distance uses strict `<`; height uses `<=`. Arithmetic stays f32.
  Start insertion receives the distance computed before visibility even when
  the callback changes positions. Height/radius limits are not refreshed during
  either candidate pass. Both passes and search iteration still run when a
  candidate set is empty.
* Every visibility query retains native FreePath with ray count -1 and
  SkipDynamic. AddOpenNode still performs native allocation/membership checks
  and computes its cost/heuristic from the state then visible to C++.
* A found goal is captured once. Nullable output skips traversal. Non-null
  output appends goal-to-start without clearing its existing contents. Native
  list insertion preserves the graph-field reference through allocation, then
  the parent link is reread. Nested direct success during append may clear the
  handler's live goal pointer; the outer call still returns true.
* Failures from visibility, reset, insertion, search or output stop immediately
  and rethrow after Rust returns, retaining partial sets/output and callback
  changes. There is no cleanup or rollback absent from the original.

**No new deviations.** Prior AStar/grid deviations remain documented above.
Invalidated graph/search pointers, invalid grid queries, uninitialized search
fields and cyclic parent chains remain outside the defined caller contract.
No cycle guard or storage ownership redesign is added.

`tools/astar-path-diff/run.ps1 <repo> [<pristine-repo>]` runs on native Windows
x64; `run.sh` is its Linux companion. Baseline `e465c7d` was archived before edits
to `rust/target/astar-path-pristine`; archive `HPL2/core` from that commit to
recreate it. Original and current switch-off use `HPL2_RUST_AI=0`, so the archived
reference exercises native AStar/grid bodies without linking Rust. Release and
debug Rust outputs are compared to the same reference.

The harness links production AStar.cpp and AINodeContainer.cpp, including real
grid/iterator, FreePath and search behavior. A normally constructed world and
concrete physics fixture supply controlled rays; unrelated dependencies use the
same bounded stubs as the connection harness. Real physics body filtering remains
native and is outside the ray fixture's coverage. Memory-manager observations
expose public search-node state without private production hooks; comparisons
include float bits, search/output order, callbacks, allocations and logs.

The corpus runs 4,399 GetPath queries across 2,048 seeded graphs plus fixed cases,
104,173 production rays, 10 controlled exceptions and 6 nested searches. It covers
repeated searches, direct success with retained sets/output, no-start/no-goal
cases, nullable output, strict/adjacent
thresholds, live endpoint aliases, captured limits, pre-visibility distances,
reset/insertion/output exceptions and nested searches. Output allocation hooks
edit a graph-field reference or parent link while preserving stored set ordering
and object lifetimes. A native allocation arena makes graph pointer ties
reproducible; leftover native allocation-failure orphans are reclaimed only after
the handler has finished and graph nodes remain alive. The allocation observer
reads its tracking vector only during armed faults inside the test's lifetime.

`tools/astar-path-diff/mutate.ps1 <repo>` plants independent realistic bugs one
at a time, requires actual output mismatches, restores exact source bytes and
rebuilds both Rust profiles in `finally`. Run it without concurrent Rust edits
or builds. Logs stay under `rust/target/astar-path-diff/`.

Verification for this slice (Rust 1.97.0, native Windows x64 MSVC):

* Workspace unit tests: **171 passed** (74 AI, 13 generate, 33 math, 27 system,
  24 FFI; 19 added by this slice).
* `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
* Path differential: **276,006 lines, 4-way identical** against `e465c7d`.
* Earlier AStar search-core regression: **294,345 lines, 4-way identical**.
* Mutation check: **12/12 caught, 0 equivalent mutants**, all actual output
  mismatches. An initial double-precision distance mutant exposed a coverage
  gap; an adjacent-float boundary case closed it, then all twelve mutations
  were rerun. Exact source bytes were restored, both Rust libraries rebuilt,
  and the clean four-way comparison passed again.
* Full `Amnesia.sln` **Debug x64 and Release x64 passed**, including Lux and
  all six editor/tool projects, using normal process access for MSVC.
* Linux/WSL was not run. The Linux runner is supplied without claiming a pass.
  Unit tests/Clippy used process-only `CARGO_INCREMENTAL=0`; no build settings
  or toolchain pin changed. Source and project line endings were preserved;
  the generated header was restored to CRLF after the final builds.

Reload the Rust project in Visual Studio (or restart VS), then build Debug and
Release x64. Load an enemy map and draw an enemy around corners or obstacles,
then return to clear line of sight. Repeat after loading a save to exercise
successive graph and direct paths. `cLuxEnemyPathfinder::MoveTo` calls this API;
cached navigation data is sufficient, without rebuilding node connections.
Gameplay validation remains for the owner.

AINodeGenerator's placement logic is covered by the following slice.

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
