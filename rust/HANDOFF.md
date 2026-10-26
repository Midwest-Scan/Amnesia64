# Handoff

Shared working notes for every agent on this port (Claude, GPT, …). **Read
this first; update it last.** Keep it under ~120 lines: current state,
next targets, open decisions, rules. Detail belongs in `README.md`
(per-module audits and verification), `PHASE2.md` (the Rust-owned class
pattern) and the code.

## Owner

- Builds only with Visual Studio (`Amnesia.sln`), Windows x64, Debug and
  Release. CMake files are stale: never edit or rely on them.
- Scope: HPL2 (`HPL2/core`) and Lux (`amnesia/src/game`). Editors and tools
  must keep building; don't port them.
- Commits himself (GitHub Desktop). **Never commit or push.** End each
  pass with a commit title and description, using your own attribution.
- After project file changes, tell him to reload the projects or restart
  VS (a stale VS cache once caused missing-library link errors).

## Current state (update every pass)

- HEAD `ae2250d` Move AI navigation ownership to Rust (phase 2 pilot).
- Rust owns: AI navigation (`cAINodeContainer`, `cAINode`,
  `cAINodeIterator`, `cAStarHandler`; phase 2).
- Phase 1 forwards (C++ keeps the objects): math, strings, preprocessor,
  logic timer, video-mode ordering, VoxelMap, the AI state machine and the
  node generator.
- Everything else is C++.
- Switches live in `HPL2/core/include/system/RustConfig.h`.
- **Awaiting the owner:** a Windows build and in-game check of the
  navigation pilot (enemy patrol, chase and pathing; delete one
  `*.nodes` file to force `Compile`). If it fails, fix that before
  anything else.

## How we work

- **One direction at a time.** Each pass has one target, agreed with the
  owner. The agent executing it does not choose the target after next on
  its own. It proposes, and the owner decides.
- **Pick targets by value:**
  1. Prefer code the game actually runs (real callers in Lux or the
     engine loop), so in-game testing means something. VoxelMap, the HPL2
     state machine and AINodeGenerator have no game callers, which made
     them weak targets.
  2. Prefer whole classes in the phase 2 pattern over method slices.
  3. Before starting, write in this file which C++ code the port removes
     from the Rust-mode path. If the honest answer is "none, it only adds
     bridges", the target is wrong.
- **Stop rule.** Stop and write a short proposal here instead of
  continuing when:
  - the bridge (callbacks, host structs, C++ glue) would be larger than
    the logic moved;
  - the target has no callers;
  - the original's structure blocks a clean port (e.g. classes subclassed
    by Lux).
- **Check in every 2–3 commits.** In the log below, say what moved, the
  lines of Rust versus bridge, and whether the approach still pays off.
- **Verification is unchanged:**
  - the public-C++-API differential harness compares four variants
    (original, switch off, Rust release, Rust debug) byte for byte;
  - 8–12 planted mutations;
  - `cargo test` and `cargo clippy -D warnings`;
  - a full solution build where available (Windows-native agents).
  - Never claim a run you didn't do. Baselines:
    `git archive <commit> HPL2/core rust/crates/hpl2-ffi/include`.

## Rules that bite (details in README/PHASE2)

- Results must be bit-identical; mark kept quirks `Faithful:`. Use
  `Deviation:` only where the original has undefined behaviour.
- Never delete original C++; switch 0 must compile it.
- One Rust staticlib (`hpl2-ffi`); `panic = "abort"`; C++ exceptions never
  unwind through Rust (catch, stop, rethrow after Rust returns).
- Rust holds no references across a callback into C++.
- cbindgen ignores `macro_rules!`: write exports out by hand and check
  `hpl2_rs.h`.
- Line endings and formatting:
  - CRLF: C++ files, VS project files, `build.cmd`, `hpl2_rs.h`.
  - LF: Rust, TOML, docs and tools.
  - No workspace `rustfmt`.
- Every new file goes in its `.vcxproj` and its `.filters` (no
  wildcards).
- Ask the owner before:
  - adding a dependency or `cxx`;
  - changing gameplay behaviour;
  - irreversible work, such as deleting tracked files.

## Next targets (proposed, owner decides)

1. **`resources/cFileSearcher`** (215 lines, used by every resource load).
   - Whole class, no subclasses: the multimap of names to paths and
     directory scans.
   - Directory listing stays C++ through `cPlatform::FindFilesInDir`.
   - Wrinkle: `GetFilePath` returns `const tWString&`. Keep a C++-side
     string per entry, or decide how to hand out stable wide strings.
   - In-game check: any map load.
2. **`input`** (2.6k lines; Lux actions are used every frame).
   - Needs a design for virtual device classes (`iKeyboard` etc. are
     implemented by the SDL backend).
   - Propose the design before porting.
3. Remaining phase 1 AI pieces: leave as they are (no game callers)
   unless they block something.

## Open decisions (owner)

- Phase 2 for classes that Lux subclasses (virtual dispatch from Rust into
  C++ overrides). Not needed until input, scene or gui.
- `cxx` crate: not adopted. Plain C ABI plus facades so far.

## Log (newest first; 3–5 lines per pass)

- 2026-10-25 Claude: phase 2 pilot, AI navigation (`ae2250d`).
  - Replaced the phase 1 A* and container bridges (about 700 lines and
    four harnesses).
  - `ai-nav-diff`: 683,543 lines, 4-way identical on Linux; 12/12
    mutations caught.
  - Ran every Linux harness for the first time; all pass.
  - Not built on Windows by me.
- `8cd7087`..`c17b6ab` GPT: LogicTimer/VideoComp, VoxelMap, then AI in
  method slices (A* core, grid, Compile, state machine, path setup,
  placement, scheduler).
  - Each slice was verified natively on Windows.
  - The slices were mostly bridges; navigation was later superseded by
    phase 2.
- `7d675c3`, `5cc7a98` Claude: math; strings and preprocessor (phase 1
  foundation).
