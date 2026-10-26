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

- HEAD `2b8f159` verified AI navigation on Windows; no agent commits.
- Uncommitted: whole-class `cFileSearcher` port (owner approved the
  bridge-size exception), tests, VS entries and docs. No new dependencies.
- Rust owns FileSearcher and AI navigation (`cAINodeContainer`, `cAINode`,
  `cAINodeIterator`, `cAStarHandler`). Phase 1 forwards remain for math,
  strings/preprocessor, timer/video, VoxelMap, AI state machine/generator.
- Switches: `HPL2/core/include/system/RustConfig.h`; new
  `HPL2_RUST_RESOURCES=1`. AI remains 1. Both final executables use 1.
- Resources verification: Debug/Release x64 with switch 0 and 1 all pass;
  271 unit tests, strict Clippy clean; 359,553 lines 4-way vs `2b8f159`,
  15-line real Windows filesystem 4-way; 12/12 mutations, 0 equivalent.
- **Awaiting owner:** reload VS projects; load several maps and a save,
  check textures/sounds/map scripts, compare resources 0, then restore 1.
  Navigation gameplay checks (patrol/chase/cache rebuild) also remain.

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

1. **`resources/cFileSearcher`: completed with the approved exception.**
   - Rust replaces native multimap, duplicate/ranking and recursive scan
     decisions. Original C++ remains at switch 0.
   - C++ keeps platform files/folders/full-path calls and stable deque
     strings plus `msNull`. Copies are deep; reference lifetime is preserved.
   - Filename-only lowercase, first-entry duplicate checking, score ties
     and untouched count outputs retain the original quirks.
   - 176 Rust code lines (99 pure/module +77 ABI), 92 C++ facade;
     bridge 169. Excludes comments/tests/header additions/generated code.
   - Lifetime token permits defined terminal destruction; only subsequent
     member use aborts. Signed component-index overflow also aborts (UB).
2. **`input`** (2.6k lines, real per-frame Lux callers): needs a design for
   virtual device classes (`iKeyboard` etc. implemented by SDL). Propose
   the design before porting; not authorized as the automatic next target.
3. Remaining phase 1 AI pieces: leave alone unless they block something.

## Open decisions (owner)

- Phase 2 for Lux-subclassed classes (virtual dispatch into overrides).
- `cxx` remains unadopted; plain C ABI plus facades so far.

## Log (newest first; 3–5 lines per pass)

- 2026-10-25 GPT: FileSearcher data/decisions moved to Rust, stable native
  strings/deep copies retained. Approved exception: 176 Rust /92 C++ code.
  271 tests, strict Clippy, 359,553-line public +15-line native filesystem
  4-way tests, 12/12 mutants, all four resources 0/1 solution builds pass.
  README/PHASE2 detail the audit, two original-UB guards and lifetime fix.
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
