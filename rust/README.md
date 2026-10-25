# HPL2 / Lux Rust port

Incremental rewrite of the HPL2 engine and the Lux game code in Rust. The
game stays buildable and playable throughout: each C++ module is replaced by
Rust one at a time, behind a switch, and checked against the original before
moving on.

| Module | Status | Switch |
|---|---|---|
| `HPL2/core/*/math` | **Ported** (cMath, cQuaternion, cFrustum, cBoundingVolume, cCRC) | `HPL2_RUST_MATH` |
| `HPL2/core/*/system` | **Partly ported** (cString, cPreprocessParser; SHA1 in Rust only) | `HPL2_RUST_SYSTEM` |
| everything else | C++ | – |

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
cargo clippy --workspace   # lints
cargo build -p hpl2-ffi    # what Visual Studio runs
```

## Layout

```
rust/
  HPL2Rust.vcxproj      Visual Studio wrapper project (Makefile type) -> build.cmd
  HPL2Rust.props        Shared MSBuild settings, imported by HPL2, Lux and the tools
  crates/
    hpl-math/           Port of HPL2/core/{include,sources}/math. Pure Rust, no C ABI.
    hpl-system/         Port of parts of HPL2/core/*/system (strings, shader preprocessor)
    hpl2-ffi/           The C ABI: builds hpl2_rs.lib and generates include/hpl2_rs.h
  tools/math-diff/      Differential test: original C++ vs. Rust, bit for bit
  tools/system-diff/    The same for cString and cPreprocessParser
```

C++ side:

```
HPL2/core/include/system/RustBridge.h    switches, type conversions, layout checks
HPL2/core/sources/system/RustBridge.cpp  routes Rust log output/panics to Log/Error
HPL2/core/sources/math/MathRust.cpp      cMath implemented by calling Rust
HPL2/core/sources/system/StringRust.cpp  cString implemented by calling Rust
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

## Troubleshooting

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
