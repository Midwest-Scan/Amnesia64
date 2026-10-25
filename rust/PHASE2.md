# Phase 2: Rust-owned classes

Phase 1 moved free functions and value types to Rust. That covered math and
strings well. For engine classes it led to a pattern where Rust ran the
*control flow* of a method while all data stayed in C++, connected by a
callback for every field read, container push and iterator step (see the
AI bridges up to `c17b6ab`). Each such slice added code without moving
ownership, and would have to be rewritten once the class itself moves.

Phase 2 moves the **data and ownership** of a class into Rust. The C++ class
remains as a thin facade with the same public API, so the rest of HPL2 and
Lux compile unchanged against it.

## The pattern

1. **Facade.** The C++ class keeps its name, public methods and semantics.
   With the module switch on, its private data is replaced by an opaque
   pointer to the Rust object (`HplXxx *mpRs`, a Rust `Box`) plus only the
   members that must stay C++. A member stays C++ when it is returned by
   reference as a C++ type (a `tString`), or when it is a pointer into a
   subsystem that is not ported yet (the physics world). The constructor
   calls `hpl_xxx_new`; the destructor calls `hpl_xxx_free`.

2. **Switch in the header.** Public headers include
   `system/RustConfig.h`, which defines only the switch macros. They never
   include `hpl2_rs.h`, which is on HPL2's include path only, not Lux's. Rust
   types are forward-declared as opaque structs. The original private members
   and inline accessors remain under `#else`, so switch 0 still compiles the
   untouched original. Inline accessors that read moved data become
   out-of-line in Rust mode.

3. **Child objects handed out by pointer** (`cAINode*`) remain C++ facade
   objects. They are allocated where the original allocated them, so their
   addresses stay stable for the owner's lifetime. Rust stores each facade
   pointer as an opaque handle. Where the original ordered objects by
   address (a `std::set` of pointers), Rust orders by the same facade
   address, so the rule is identical.

4. **Data exposed by pointer or reference** (a position, an edge array) is
   stored `#[repr(C)]` in Rust, with layout checks on both sides. The same
   invalidation rules apply as for the original container. For example, an
   edge pointer is invalidated by adding an edge, just as with
   `std::vector::push_back`.

5. **Unported dependencies** (physics, file I/O, XML, user virtual
   callbacks) are reached through one host struct of callbacks per
   operation. The exception convention is unchanged: the C++ trampoline
   catches, Rust stops, and C++ rethrows after Rust returns. Callbacks may
   use the Rust-owned object as the original allows: read it, change
   properties, add items, even start another operation on it. So Rust holds
   no references across a callback (raw pointers, re-borrowed per access) and
   reads each value where the original reads it. Only cases where the
   original would free memory still in use abort, with a clear message.

6. **Library behaviour that is observable stays native.** One example is
   the tie order of MSVC's unstable `std::sort`. Rust calls the C++ library
   on the `#[repr(C)]` data in place, rather than reimplementing it.

7. **Verification uses the public API only.** Allocation counts, private
   fields and native container internals are exactly what phase 2 changes,
   so harnesses that hook `operator new` or read private search nodes no
   longer apply. A phase 2 harness drives the public C++ API against real
   collaborators (e.g. a controlled physics world). It prints every result
   and callback, and compares original, switch-off, Rust release and Rust
   debug, as before.

8. **Superseded phase 1 bridges are removed** when their class moves:
   C++ forwards, FFI exports, `Hpl*Host` structs and the harnesses that
   observe private allocation. The pure Rust algorithms they drove are kept
   and reused. Each was written against a host trait, and the Rust-owned
   object now implements those traits natively.

Fidelity rules are unchanged: same results and operation order, `Faithful:`
for kept quirks, `Deviation:` only where the original has undefined
behaviour.

## Pilot: AI navigation (`ai/AINodeContainer`, `ai/AStar`)

Callers: `cWorld` (creation, `.nodes` cache load/save) and Lux's enemy
pathfinder. The pathfinder runs `GetPath`, node iterators, `FreePath`,
patrol nodes stored as `cAINode*`, saved games by node ID, and
`GetEdge(i)->mpNode` in debug drawing.

| | Rust (`hpl-ai/src/nav.rs`) | C++ facade |
|---|---|---|
| `cAINode` | ID, position, user data, edges (`#[repr(C)]`, same layout as `cAINodeEdge`) | `msName` (returned as `const tString&`), handle |
| `cAINodeContainer` | Nodes, name/ID lookup, grid map, properties, `Compile`, `AddEdge` metrics | Names, world, collide size, node-at-center, ray callback, **`FreePath`** (physics), XML **save/load** (TinyXML, file I/O) |
| `cAINodeIterator` | Grid cursor logic | Value type: container pointer + POD cursor state |
| `cAStarHandler` | Open/closed/goal sets, search nodes, `GetPath`, iteration limit | Container pointer, `iAStarCallback*` |

Host callbacks: `FreePath` (visibility), `CanAddNode`, the edge sort
(`std::sort` with the original comparator), and the path append
(`tAINodeList::push_back`). Everything else runs in Rust without
callbacks.

Deviations, all from original undefined behaviour:
- `GetNode` and `cAINode::GetEdge` out of range return NULL.
- `cAINodeIterator::Next` at the end returns NULL.
- A grid built with no nodes is empty.
- An edge whose target node no longer exists (a stale `.nodes` file
  naming a missing node) is skipped by the search, where the original
  dereferences NULL. The edge itself is still listed for `GetEdge`.
- A `.nodes` entry with an unknown node ID is skipped, where the original
  dereferences NULL.
- A nested `GetPath` on the same handler that resets the search sets the
  outer search is using aborts, and so does deleting a container or handler
  during its own operation. The original frees memory still in use.

In-game checks:
- Enemies patrol, hunt and follow paths.
- Saves restore patrol nodes.
- Delete a map's `*.nodes` cache to force the `Compile` path.
- Compare against `HPL2_RUST_AI 0` in `system/RustConfig.h`.
