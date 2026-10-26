# Handoff

Shared working notes for every agent (Claude, GPT, …). **Read this first;
update it last.** Keep it under ~120 lines. Detail belongs in `README.md`
(audits/verification), `PHASE2.md` (Rust-owned classes), and the code.

## Owner

- Visual Studio only (`Amnesia.sln`), Windows x64, Debug and Release.
  CMake files are stale: never edit or rely on them.
- Scope: HPL2 (`HPL2/core`) and Lux (`amnesia/src/game`). Editors/tools
  must keep building; don't port them.
- Commits himself (GitHub Desktop). **Never commit or push.** End each
  pass with a commit title/description and your own attribution.
- After project changes, tell him to reload projects or restart VS.

## Current state (update every pass)

- HEAD `a45c738` shared notes; navigation implementation is `ae2250d`.
- Rust owns `cAINodeContainer`, `cAINode`, `cAINodeIterator` and
  `cAStarHandler` (phase 2). Phase 1 forwards: math, strings/preprocessor,
  logic timer, video ordering, VoxelMap, AI state machine/node generator.
- Switches live in `HPL2/core/include/system/RustConfig.h`.
- Windows verification passed: full Debug/Release x64 with AI at 1 and 0,
  all six native harnesses 4-way, 233 unit tests, strict Clippy clean.
- Uncommitted: Windows ai-nav runner/allocator, project lists, these notes
  and README. No production port/fix this pass; FileSearcher stays C++.
- AI default restored to 1; last built executables use 0. Rebuild for 1.
- **Awaiting owner:** in-game navigation check (several maps, patrol/chase,
  save/reload, temporarily rename a `.nodes` cache to force Compile), then
  compare with AI 0. Also decide the FileSearcher proposal below.

## How we work

- **One direction at a time.** Each pass has one owner-agreed target.
  The executing agent proposes the target after next; the owner decides.
- **Pick targets by value:** prefer real Lux/engine-loop callers and whole
  phase 2 classes. VoxelMap, the HPL2 state machine and AINodeGenerator
  have no game callers, so they were weak targets.
- Before starting, write here which C++ code leaves the Rust-mode path.
  If the honest answer is "none, it only adds bridges", the target is wrong.
- **Stop rule.** Stop and write a proposal here instead when the bridge
  (callbacks, host structs, C++ glue) would be larger than the logic moved,
  the target has no callers, or structure blocks a clean port (e.g. classes
  subclassed by Lux).
- **Check in every 2–3 commits.** Log what moved, Rust versus bridge line
  counts, and whether the approach still pays off.
- **Verification:** public-C++-API harness, original/switch-off/Rust release/
  Rust debug byte-identical; 8–12 planted mutations; `cargo test` and
  `cargo clippy -D warnings`; full solution builds on native Windows.
  Never claim an unrun check. Archive baselines with
  `git archive <commit> HPL2/core rust/crates/hpl2-ffi/include`.

## Rules that bite (details in README/PHASE2)

- Bit-identical results; mark kept quirks `Faithful:`. Use `Deviation:`
  only for original undefined behaviour. Never delete original C++;
  switch 0 must compile it.
- One Rust staticlib (`hpl2-ffi`), `panic = "abort"`. Native exceptions
  never unwind through Rust: catch, stop, rethrow after Rust returns.
- Rust holds no references across a C++ callback.
- cbindgen ignores `macro_rules!`: explicit exports; check `hpl2_rs.h`.
- CRLF: C++/VS files, `build.cmd`, generated header. LF: Rust/TOML/docs/
  tools. Preserve existing formatting; no workspace `rustfmt`.
- Every new file goes in `.vcxproj` and `.filters`; no wildcards.
- Ask before dependencies/`cxx`, gameplay changes, or irreversible work
  such as deleting tracked files.

## Next targets (proposed, owner decides)

1. **`resources/cFileSearcher`: stopped under the bridge-size rule.**
   - Plan: Rust owns filename buckets, duplicate checks, split components,
     ranking and recursive scans; removes native multimap/lookup/traversal
     decisions from the Rust-mode path. Original bodies stay at switch 0.
   - C++ keeps `cPlatform::FindFilesInDir`, `FindFoldersInDir`,
     `GetFullFilePath`, and exception-catching adapters.
   - Stable `const tWString&`: one native string per accepted entry in
     nonmoving storage, plus `msNull`. Adds/lookups preserve references;
     clear/destruction invalidates entry references. Preserve deep copying.
   - Preserve filename-only lowercase, case-sensitive path ranking,
     first-entry-only duplicate suppression, insertion-order score ties,
     and fast returns that leave the equal-count output untouched.
   - Measured original: 215 physical /107 nonblank, noncomment C++ lines.
     Estimated bridge: 130–190 code lines across ABI + C++ facade, before
     pure Rust logic. Estimate only: no resources code/switch was written.
   - Proposal: defer until a shared directory/path boundary can amortize
     this bridge across useful resource code, or explicitly authorize a
     bridge-size exception. README contains the full audit/test proposal.
   - In-game check after a future port: load several maps, compare switch 0.
2. **`input`** (2.6k lines, real per-frame Lux callers): needs a design for
   virtual device classes (`iKeyboard` etc. implemented by SDL). Propose
   the design before porting; not authorized as the automatic next target.
3. Remaining phase 1 AI pieces: leave alone unless they block something.

## Open decisions (owner)

- FileSearcher bridge-size exception or revised scope, as above.
- Phase 2 for Lux-subclassed classes (virtual dispatch into overrides).
- `cxx` remains unadopted; plain C ABI plus facades so far.

## Log (newest first; 3–5 lines per pass)

- 2026-10-25 GPT: verified `ae2250d` on Windows; all four full solution
  builds, six native harnesses, 233 tests and strict Clippy passed.
  ai-nav: 683,569 lines 4-way vs `e91d65c`; VirtualAlloc runner added.
  Production Rust/C++ glue added: 0/0. No mutations rerun. FileSearcher
  stopped before code: estimated bridge exceeds its 107-line implementation.
- 2026-10-25 Claude, `ae2250d`: Rust-owned AI navigation replaced ~700
  phase 1 bridge lines/four harnesses. Linux: 683,543 lines 4-way,
  12/12 mutations; all Linux harnesses pass. Windows verified above.
- `8cd7087`..`c17b6ab` GPT: timer/video, VoxelMap, AI method slices;
  each verified natively. Navigation bridges later superseded by phase 2.
- `7d675c3`, `5cc7a98` Claude: math, strings and preprocessor foundation.
