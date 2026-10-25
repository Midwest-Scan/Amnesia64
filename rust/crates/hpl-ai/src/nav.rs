//! Rust-owned AI navigation (phase 2): the data of `cAINodeContainer`,
//! `cAINode`, `cAINodeIterator` and `cAStarHandler` (`ai/AINodeContainer.*`,
//! `ai/AStar.*`). See `rust/PHASE2.md`.
//!
//! The C++ classes remain as facades. Each `cAINode` facade is a C++ object
//! allocated by `AddNode`; Rust knows it only as an opaque [`Handle`] (its
//! address). Handles identify nodes in edges and order the search sets, as
//! the original `std::set<cAStarNode*>` ordered by `cAINode*`.
//!
//! The algorithms are the verified ones from [`node_container`],
//! [`node_compile`], [`astar`] and [`astar_path`]. Their host traits are
//! implemented here on Rust-owned data instead of C++ callbacks. Only
//! physics visibility (`FreePath`), `iAStarCallback::CanAddNode`, the edge
//! sort (MSVC `std::sort` tie order) and the path list append call back into
//! C++, through [`CompileCallbacks`] and [`PathCallbacks`].
//!
//! Callbacks may use the graph as the original allows: read positions and
//! edges, move nodes, add nodes or edges, change properties, even run another
//! search. So code that calls back holds only raw pointers and re-borrows for
//! each access; no Rust reference lives across a callback, and every value is
//! read again where the original reads it. Where the original then has
//! undefined behaviour (an edge pointer or vector iterator invalidated by a
//! reallocation), the Rust indices simply stay valid. Two cases would free
//! memory still in use and abort instead: deleting a graph or handler during
//! its own operation, and a nested search on the same handler resetting the
//! search sets of the outer one (see [`AStar::get_path`]).

use crate::astar::{self, SearchHost};
use crate::astar_path::{self, Endpoint, PathHost};
use crate::node_compile::{self, CompileHost};
use crate::node_container::{self, GridDimensions, GridHost};
use hpl_math::{Vector2f, Vector2l, Vector3f};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Address of a node's C++ facade (`cAINode*`). Never dereferenced in Rust.
/// 0 is NULL.
pub type Handle = usize;

/// `cAINodeEdge`, with the same layout: `{float, float, cAINode*}`. C++ reads
/// these in place through `cAINode::GetEdge`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub distance: f32,
    pub sqr_distance: f32,
    pub node: Handle,
}

/// One node. Boxed by the graph, so its address (and the position C++
/// references through `GetPosition`) stays stable.
#[derive(Debug)]
pub struct Node {
    pub id: i32,
    pub position: Vector3f,
    /// `mpUserData`: stored as in the original; nothing reads it.
    pub user_data: usize,
    pub edges: Vec<Edge>,
    pub handle: Handle,
}

/// Grid cursor of a `cAINodeIterator`; the C++ value type stores it inline.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IterState {
    pub start: Vector2l,
    pub end: Vector2l,
    pub position: Vector2l,
    /// Current grid cell, or -1 when exhausted (the original's NULL list).
    pub cell: i32,
    /// Index into the current cell's list (the original's list iterator).
    pub item: u32,
}

#[derive(Debug, Default)]
struct Grid {
    dimensions: GridDimensions,
    cell: Vector2f,
    cells: Vec<Vec<usize>>,
}

/// `cAINodeContainer`'s graph: nodes, lookups, grid map and properties.
#[derive(Debug)]
pub struct NavGraph {
    // Boxed on purpose: C++ keeps pointers to nodes and their positions
    // while the vector grows.
    #[allow(clippy::vec_box)]
    nodes: Vec<Box<Node>>,
    by_name: HashMap<Vec<u8>, usize>,
    by_id: HashMap<i32, usize>,
    by_handle: HashMap<Handle, usize>,
    grid: Grid,
    pub max_edges: i32,
    pub min_edges: i32,
    pub max_edge_distance: f32,
    pub max_height: f32,
    pub nodes_per_grid: i32,
    /// Compiles and searches running on this graph (they may nest).
    active: u32,
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

impl Default for NavGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl NavGraph {
    /// The constructor's defaults.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            by_name: HashMap::new(),
            by_id: HashMap::new(),
            by_handle: HashMap::new(),
            grid: Grid::default(),
            max_edges: 5,
            min_edges: 2,
            max_edge_distance: 3.0,
            max_height: 0.1,
            nodes_per_grid: 6,
            active: 0,
        }
    }

    /// `ReserveSpace`
    /// True while a compile or search uses this graph.
    ///
    /// # Safety
    /// `this` must point to a live graph. Reads without creating a reference.
    pub unsafe fn is_active(this: *const NavGraph) -> bool {
        unsafe { std::ptr::addr_of!((*this).active).read() != 0 }
    }

    pub fn reserve(&mut self, count: usize) {
        self.nodes.reserve(count);
    }

    /// `AddNode`. Faithful: a duplicate name or ID keeps the first node in its
    /// lookup (`std::map::insert`), while every node is still added.
    pub fn add_node(&mut self, name: &[u8], id: i32, position: Vector3f, user_data: usize, handle: Handle) -> *mut Node {
        let index = self.nodes.len();
        self.nodes.push(Box::new(Node { id, position, user_data, edges: Vec::new(), handle }));
        self.by_name.entry(name.to_vec()).or_insert(index);
        self.by_id.entry(id).or_insert(index);
        self.by_handle.entry(handle).or_insert(index);
        &mut *self.nodes[index]
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// `GetNode`. Deviation: an index out of range returns NULL (the
    /// original indexes past its vector).
    pub fn node_handle(&self, index: i32) -> Handle {
        usize::try_from(index).ok().and_then(|i| self.nodes.get(i)).map_or(0, |n| n.handle)
    }

    pub fn node(&self, index: usize) -> &Node {
        &self.nodes[index]
    }

    /// `GetNodeFromName`
    pub fn from_name(&self, name: &[u8]) -> Handle {
        self.by_name.get(name).map_or(0, |&i| self.nodes[i].handle)
    }

    /// `GetNodeFromID`
    pub fn from_id(&self, id: i32) -> Handle {
        self.by_id.get(&id).map_or(0, |&i| self.nodes[i].handle)
    }

    /// Node index for a facade handle; None for NULL or a foreign handle.
    pub fn index_of(&self, handle: Handle) -> Option<usize> {
        if handle == 0 { None } else { self.by_handle.get(&handle).copied() }
    }

    /// `BuildNodeGridMap`. Faithful: cells are resized, not cleared, so a
    /// rebuild appends every node again to the retained lists.
    ///
    /// Deviation: the original reads `mvNodes[0]` of an empty container
    /// (undefined behaviour); here the grid is left as it is.
    pub fn build_grid(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        node_container::build_grid(&mut GridAccess { graph: self });
    }

    // -- iterator ------------------------------------------------------------

    /// `cAINodeIterator` constructor.
    ///
    /// Deviation: before the grid is built the original indexes an empty
    /// vector; here the iterator is simply empty.
    pub fn iter_begin(&self, position: Vector3f, radius: f32) -> IterState {
        let grid = &self.grid;
        let mut state = IterState { cell: -1, ..IterState::default() };
        if grid.cells.is_empty() {
            return state;
        }
        let local = Vector2f::new(position.x, position.z) - grid.dimensions.minimum;
        let local_start = local - Vector2f::splat(radius);
        let local_end = local + Vector2f::splat(radius);
        state.start = node_container::grid_position(local_start, grid.cell, grid.dimensions.map_size);
        state.end = node_container::grid_position(local_end, grid.cell, grid.dimensions.map_size);
        state.position = state.start;
        state.cell = self.cell_index(state.position);
        self.skip_empty(&mut state);
        state
    }

    fn cell_index(&self, position: Vector2l) -> i32 {
        node_container::grid_index(position, self.grid.dimensions.map_size, self.grid.cells.len()) as i32
    }

    fn cell(&self, state: &IterState) -> Option<&Vec<usize>> {
        usize::try_from(state.cell).ok().and_then(|c| self.grid.cells.get(c))
    }

    /// Advance from an empty current cell to the next non-empty one.
    fn skip_empty(&self, state: &mut IterState) {
        while self.cell(state).is_some_and(|c| c.is_empty()) {
            if node_container::increment_grid(&mut state.position, state.start, state.end) {
                state.cell = self.cell_index(state.position);
            } else {
                state.cell = -1;
                break;
            }
        }
        if state.cell >= 0 {
            state.item = 0;
        }
    }

    /// `HasNext`: the current list exists and is not empty (checked live).
    pub fn iter_has_next(&self, state: &IterState) -> bool {
        self.cell(state).is_some_and(|c| !c.is_empty())
    }

    /// `Next`, as a node index. Deviation: past the end (undefined in the
    /// original) returns None.
    pub fn iter_next(&self, state: &mut IterState) -> Option<usize> {
        let list = self.cell(state)?;
        let node = *list.get(state.item as usize)?;
        state.item += 1;
        if state.item as usize == list.len() {
            if node_container::increment_grid(&mut state.position, state.start, state.end) {
                state.cell = self.cell_index(state.position);
                self.skip_empty(state);
            } else {
                state.cell = -1;
            }
        }
        Some(node)
    }

    // -- edges ---------------------------------------------------------------

    /// `cAINode::AddEdge`: both metrics from the current positions.
    ///
    /// # Safety
    /// Both pointers must be nodes of live graphs, and no reference to the
    /// source node's edges may be held.
    pub unsafe fn add_edge(node: *mut Node, target: *const Node) {
        let (start, end, handle) = unsafe { ((*node).position, (*target).position, (*target).handle) };
        let (distance, sqr_distance) = node_container::edge_distances(start, end);
        unsafe { (*node).edges.push(Edge { distance, sqr_distance, node: handle }) };
    }

    /// An edge read from a `.nodes` file: metrics as stored; the target may be
    /// NULL when the file names a node that no longer exists.
    ///
    /// # Safety
    /// `node` must be a node of a live graph.
    pub unsafe fn push_edge(node: *mut Node, edge: Edge) {
        unsafe { (*node).edges.push(edge) };
    }

    // -- compile -------------------------------------------------------------

    /// `Compile`: rebuild the grid, then connect every node to the visible
    /// nodes around it, sort, cap and trim (see [`node_compile`]).
    ///
    /// # Safety
    /// `graph` must be valid for the whole call; callbacks may only read it.
    pub unsafe fn compile(graph: *mut NavGraph, callbacks: &mut impl CompileCallbacks) {
        unsafe {
            (*graph).build_grid();
            (*graph).active += 1;
        }
        // Faithful: the original walks mvNodes with a live end, so nodes added
        // by a callback are compiled too (its vector iterator would then be
        // invalidated by the reallocation; indices are not).
        let mut source = 0;
        while source < unsafe { (*graph).nodes.len() } {
            let iter = unsafe {
                let g = &*graph;
                // Faithful: the radius is computed in f32 from the live property.
                g.iter_begin(g.nodes[source].position, g.max_edge_distance * 1.5)
            };
            let mut host = CompileAccess { graph, source, iter, callbacks: &mut *callbacks, failed: false };
            node_compile::compile_node(&mut host, source);
            if host.failed {
                break;
            }
            source += 1;
        }
        unsafe { (*graph).active -= 1 };
    }
}

struct GridAccess<'a> {
    graph: &'a mut NavGraph,
}

impl GridHost for GridAccess<'_> {
    type Node = usize;
    fn node_count(&self) -> usize { self.graph.nodes.len() }
    fn node(&self, index: usize) -> usize { index }
    fn position(&self, node: usize) -> Vector3f { self.graph.nodes[node].position }
    fn nodes_per_grid(&self) -> i32 { self.graph.nodes_per_grid }
    fn set_bounds(&mut self, minimum: Vector2f, maximum: Vector2f) {
        self.graph.grid.dimensions.minimum = minimum;
        self.graph.grid.dimensions.maximum = maximum;
    }
    fn set_map_size(&mut self, size: Vector2l) { self.graph.grid.dimensions.map_size = size; }
    fn resize_grids(&mut self, count: usize) -> bool {
        // Faithful: std::vector::resize keeps the existing lists.
        self.graph.grid.cells.resize_with(count, Vec::new);
        true
    }
    fn dimensions(&self) -> GridDimensions { self.graph.grid.dimensions }
    fn set_cell_size(&mut self, size: Vector2f) { self.graph.grid.cell = size; }
    fn cell_size(&self) -> Vector2f { self.graph.grid.cell }
    fn grid_count(&self) -> usize { self.graph.grid.cells.len() }
    fn push_node(&mut self, index: usize, node: usize) -> bool {
        self.graph.grid.cells[index].push(node);
        true
    }
}

/// What `Compile` needs from C++.
pub trait CompileCallbacks {
    /// `FreePath(source, candidate, -1, SkipDynamic | SkipVolatile)`; None if
    /// it threw (stop immediately).
    fn free_path(&mut self, start: Vector3f, end: Vector3f) -> Option<bool>;
    /// `std::sort` with the original comparator, in place. False if it threw.
    fn sort_edges(&mut self, edges: &mut [Edge]) -> bool;
}

struct CompileAccess<'a, C> {
    graph: *mut NavGraph,
    source: usize,
    iter: IterState,
    callbacks: &'a mut C,
    failed: bool,
}

impl<C> CompileAccess<'_, C> {
    fn graph(&self) -> &NavGraph {
        // SAFETY: valid for the compile; no mutable borrow is live here.
        unsafe { &*self.graph }
    }
    fn edges(&mut self) -> &mut Vec<Edge> {
        // SAFETY: short borrow, never held across a callback.
        unsafe {
            let graph = &mut *self.graph;
            &mut graph.nodes[self.source].edges
        }
    }
}

impl<C: CompileCallbacks> CompileHost for CompileAccess<'_, C> {
    type Node = usize;
    fn next_candidate(&mut self) -> Option<usize> {
        let graph = self.graph;
        // SAFETY: as above.
        unsafe { (*graph).iter_next(&mut self.iter) }
    }
    fn position(&self, node: usize) -> Vector3f { self.graph().nodes[node].position }
    fn max_distance(&self) -> f32 { self.graph().max_edge_distance }
    fn max_height(&self) -> f32 { self.graph().max_height }
    fn visible(&mut self, candidate: usize) -> Option<bool> {
        let (start, end) = (self.position(self.source), self.position(candidate));
        let result = self.callbacks.free_path(start, end);
        if result.is_none() {
            self.failed = true;
        }
        result
    }
    fn add_edge(&mut self, candidate: usize) -> bool {
        // SAFETY: both nodes are boxed in the live graph; no borrow is held.
        unsafe {
            let graph = &mut *self.graph;
            let target: *const Node = &*graph.nodes[candidate];
            let node: *mut Node = &mut *graph.nodes[self.source];
            NavGraph::add_edge(node, target);
        }
        true
    }
    fn sort_edges(&mut self) -> bool {
        let edges: *mut Vec<Edge> = self.edges();
        // SAFETY: C++ sorts exactly this buffer; nothing else borrows it.
        let ok = self.callbacks.sort_edges(unsafe { &mut *edges });
        if !ok {
            self.failed = true;
        }
        ok
    }
    fn max_edges(&self) -> i32 { self.graph().max_edges }
    fn edge_count(&self) -> usize { self.graph().nodes[self.source].edges.len() }
    fn resize_edges(&mut self, count: usize) -> bool {
        // Only ever shrinks here.
        self.edges().truncate(count);
        true
    }
    fn edge_distance(&self, index: usize) -> f32 { self.graph().nodes[self.source].edges[index].distance }
    fn min_edges(&self) -> i32 { self.graph().min_edges }
}

// ---------------------------------------------------------------------------
// A*
// ---------------------------------------------------------------------------

/// `cAStarNode`
#[derive(Clone, Copy, Debug)]
struct Search {
    node: usize,
    cost: f32,
    distance: f32,
    parent: Option<Handle>,
}

/// `cAStarHandler`'s search state. The open and closed sets are keyed by the
/// node's facade address, the original `cAStarNodeCompare` order.
#[derive(Debug)]
pub struct AStar {
    open: BTreeMap<Handle, Search>,
    closed: BTreeMap<Handle, Search>,
    goals: BTreeSet<Handle>,
    goal_position: Vector3f,
    goal: Option<Handle>,
    pub max_iterations: i32,
    /// GetPath calls running on this handler (a callback may start another).
    depth: u32,
    /// Set by the call whose search nodes are live (after its reset).
    searching: bool,
}

impl Default for AStar {
    fn default() -> Self {
        Self::new()
    }
}

/// What `GetPath` needs from C++.
pub trait PathCallbacks {
    /// `mpContainer->FreePath(start, end, -1, SkipDynamic)`; None if it threw.
    fn free_path(&mut self, start: Vector3f, end: Vector3f) -> Option<bool>;
    /// `mpCallback == NULL || mpCallback->CanAddNode(parent, child)`, read
    /// live; None if it threw.
    fn can_add_node(&mut self, parent: Handle, child: Handle) -> Option<bool>;
    /// `apNodeList->push_back(node)`; false if it threw.
    fn append(&mut self, node: Handle) -> bool;
}

impl AStar {
    pub fn new() -> Self {
        Self {
            open: BTreeMap::new(),
            closed: BTreeMap::new(),
            goals: BTreeSet::new(),
            goal_position: Vector3f::default(),
            goal: None,
            max_iterations: -1,
            depth: 0,
            searching: false,
        }
    }

    /// True while a `get_path` on this handler is running.
    ///
    /// # Safety
    /// `this` must point to a live `AStar`. Reads without creating a
    /// reference, so it is safe to call while an outer search is running.
    pub unsafe fn is_busy(this: *const AStar) -> bool {
        unsafe { std::ptr::addr_of!((*this).depth).read() != 0 }
    }

    /// `GetPath`. Endpoints are taken by value at entry. Returns false if no
    /// path was found or a callback threw (the caller rethrows).
    ///
    /// A callback may start another search on the same handler, as in the
    /// original: one that ends at the direct free-path check only clears the
    /// goal. Deviation: one that goes on to reset the search sets while the
    /// outer search is using them (the original deletes those nodes and keeps
    /// using them: undefined behaviour) aborts.
    ///
    /// # Safety
    /// Both pointers must be valid for the whole call; callbacks may read the
    /// graph and set properties but not change its structure.
    pub unsafe fn get_path(
        this: *mut AStar,
        graph: *mut NavGraph,
        start: Vector3f,
        goal: Vector3f,
        has_output: bool,
        callbacks: &mut impl PathCallbacks,
    ) -> bool {
        unsafe {
            (*this).depth += 1;
            (*graph).active += 1;
        }
        let mut host = PathAccess {
            astar: this,
            graph,
            endpoints: [start, goal],
            iters: [IterState::default(); 2],
            callbacks,
            has_output,
            failed: false,
            owns_search: false,
        };
        let found = astar_path::get_path(&mut host);
        unsafe {
            if host.owns_search {
                (*this).searching = false;
            }
            (*this).depth -= 1;
            (*graph).active -= 1;
        }
        found && !host.failed
    }
}

struct PathAccess<'a, C> {
    astar: *mut AStar,
    graph: *mut NavGraph,
    endpoints: [Vector3f; 2],
    iters: [IterState; 2],
    callbacks: &'a mut C,
    has_output: bool,
    failed: bool,
    /// This call reset the sets and is using their search nodes.
    owns_search: bool,
}

/// Node index meaning "edge target missing" (NULL or unknown handle).
const MISSING: usize = usize::MAX;

impl<C> PathAccess<'_, C> {
    fn graph(&self) -> &NavGraph {
        // SAFETY: valid for the search; callbacks never change its structure.
        unsafe { &*self.graph }
    }
    fn astar(&self) -> &AStar {
        // SAFETY: only this host accesses the handler during the search.
        unsafe { &*self.astar }
    }
    fn astar_mut(&mut self) -> &mut AStar {
        // SAFETY: as above; never held across a callback.
        unsafe { &mut *self.astar }
    }
    fn search(&self, key: Handle) -> Search {
        let a = self.astar();
        *a.closed.get(&key).or_else(|| a.open.get(&key)).expect("search node")
    }
    fn handle(&self, node: usize) -> Handle {
        self.graph().nodes[node].handle
    }
}

impl<C: PathCallbacks> SearchHost for PathAccess<'_, C> {
    type SearchNode = Handle;
    type Node = usize;
    type Edge = (usize, i32);

    fn open_empty(&self) -> bool { self.astar().open.is_empty() }
    fn max_iterations(&self) -> i32 { self.astar().max_iterations }
    fn best_node(&mut self) -> Option<Handle> {
        let a = self.astar_mut();
        let key = astar::best_node(a.open.iter().map(|(&k, s)| (k, s.cost)))?;
        let search = a.open.remove(&key).expect("best node");
        a.closed.insert(key, search);
        Some(key)
    }
    fn graph_node(&self, search: Handle) -> usize { self.search(search).node }
    fn is_goal(&self, node: usize) -> bool { self.astar().goals.contains(&self.handle(node)) }
    fn set_goal(&mut self, search: Handle) { self.astar_mut().goal = Some(search); }
    fn edge_count(&self, node: usize) -> i32 { self.graph().nodes[node].edges.len() as i32 }
    fn edge(&self, node: usize, index: i32) -> (usize, i32) { (node, index) }
    fn edge_node(&self, (node, index): (usize, i32)) -> usize {
        let target = self.graph().nodes[node].edges[index as usize].node;
        self.graph().index_of(target).unwrap_or(MISSING)
    }
    fn can_add(&mut self, parent: usize, child: usize) -> Option<bool> {
        if child == MISSING {
            // Deviation: an edge to a node missing from a stale .nodes file
            // has a NULL target; the original passes it on and dereferences it.
            return Some(false);
        }
        let (parent, child) = (self.handle(parent), self.handle(child));
        let result = self.callbacks.can_add_node(parent, child);
        if result.is_none() {
            self.failed = true;
        }
        result
    }
    fn edge_distance(&self, (node, index): (usize, i32)) -> f32 {
        self.graph().nodes[node].edges[index as usize].distance
    }
    fn search_distance(&self, search: Handle) -> f32 { self.search(search).distance }
    fn add_open(&mut self, node: usize, parent: Handle, distance: f32) -> bool {
        self.add_open_node(node, Some(parent), distance);
        true
    }
}

impl<C> PathAccess<'_, C> {
    /// `AddOpenNode`. Faithful: a node already open is never improved, and a
    /// closed node is never reopened.
    fn add_open_node(&mut self, node: usize, parent: Option<Handle>, distance: f32) {
        let key = self.handle(node);
        let a = self.astar();
        if a.closed.contains_key(&key) || a.open.contains_key(&key) {
            return;
        }
        let position = self.graph().nodes[node].position;
        let parent_y = parent.map(|p| self.graph().nodes[self.search(p).node].position.y);
        let cost = astar::cost(distance, position.y, parent_y) + astar::heuristic(position, a.goal_position);
        self.astar_mut().open.insert(key, Search { node, cost, distance, parent });
    }
}

impl<C: PathCallbacks> PathHost for PathAccess<'_, C> {
    type Node = usize;
    type Search = Handle;

    fn max_height(&self) -> f32 { self.graph().max_height }
    fn endpoint(&self, endpoint: Endpoint) -> Vector3f { self.endpoints[endpoint as usize] }
    fn direct_visible(&mut self) -> Option<bool> {
        let [start, goal] = self.endpoints;
        let result = self.callbacks.free_path(start, goal);
        if result.is_none() {
            self.failed = true;
        }
        result
    }
    fn clear_goal(&mut self) { self.astar_mut().goal = None; }
    fn reset_search(&mut self) -> bool {
        let a = self.astar_mut();
        assert!(
            !a.searching,
            "AI navigation: a nested GetPath on the same cAStarHandler reset the search the outer GetPath is using"
        );
        a.searching = true;
        self.owns_search = true;
        let a = self.astar_mut();
        a.closed.clear();
        a.open.clear();
        a.goals.clear();
        a.goal = None;
        true
    }
    fn set_goal_position(&mut self, goal: Vector3f) { self.astar_mut().goal_position = goal; }
    fn max_distance(&self) -> f32 { self.graph().max_edge_distance }
    fn begin_candidates(&mut self, endpoint: Endpoint, radius: f32) -> bool {
        let position = self.endpoints[endpoint as usize];
        self.iters[endpoint as usize] = self.graph().iter_begin(position, radius);
        true
    }
    fn next_candidate(&mut self, endpoint: Endpoint) -> Option<usize> {
        let graph = self.graph;
        // SAFETY: as for graph().
        unsafe { (*graph).iter_next(&mut self.iters[endpoint as usize]) }
    }
    fn position(&self, node: usize) -> Vector3f { self.graph().nodes[node].position }
    fn visible(&mut self, endpoint: Endpoint, node: usize) -> Option<bool> {
        let (start, end) = (self.endpoints[endpoint as usize], self.position(node));
        let result = self.callbacks.free_path(start, end);
        if result.is_none() {
            self.failed = true;
        }
        result
    }
    fn add_start(&mut self, node: usize, distance: f32) -> bool {
        self.add_open_node(node, None, distance);
        true
    }
    fn add_goal(&mut self, node: usize) -> bool {
        let key = self.handle(node);
        self.astar_mut().goals.insert(key);
        true
    }
    fn iterate(&mut self) -> bool {
        astar::iterate(self);
        !self.failed
    }
    fn goal_node(&self) -> Option<Handle> { self.astar().goal }
    fn has_output(&self) -> bool { self.has_output }
    fn append_path(&mut self, search: Handle) -> bool {
        let ok = self.callbacks.append(search);
        if !ok {
            self.failed = true;
        }
        ok
    }
    fn parent(&self, search: Handle) -> Option<Handle> { self.search(search).parent }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Handles in tests: 1000 + 16 * index, so address order = index order
    /// unless a test says otherwise.
    fn handle(i: usize) -> Handle {
        1000 + 16 * i
    }

    fn graph(points: &[(f32, f32, f32)]) -> NavGraph {
        let mut g = NavGraph::new();
        for (i, &(x, y, z)) in points.iter().enumerate() {
            g.add_node(format!("n{i}").as_bytes(), i as i32, Vector3f::new(x, y, z), 0, handle(i));
        }
        g
    }

    struct Physics {
        blocked: Vec<(Vector3f, Vector3f)>,
        calls: Vec<(Vector3f, Vector3f)>,
        fail_at: Option<usize>,
    }

    impl Physics {
        fn open() -> Self {
            Self { blocked: Vec::new(), calls: Vec::new(), fail_at: None }
        }
        fn free(&mut self, start: Vector3f, end: Vector3f) -> Option<bool> {
            self.calls.push((start, end));
            if self.fail_at == Some(self.calls.len()) {
                return None;
            }
            Some(!self.blocked.iter().any(|&(a, b)| (a == start && b == end) || (a == end && b == start)))
        }
    }

    impl CompileCallbacks for Physics {
        fn free_path(&mut self, start: Vector3f, end: Vector3f) -> Option<bool> { self.free(start, end) }
        fn sort_edges(&mut self, edges: &mut [Edge]) -> bool {
            edges.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
            true
        }
    }

    struct Search {
        physics: Physics,
        rejected: Vec<(Handle, Handle)>,
        asked: Vec<(Handle, Handle)>,
        path: Vec<Handle>,
    }

    impl PathCallbacks for Search {
        fn free_path(&mut self, start: Vector3f, end: Vector3f) -> Option<bool> { self.physics.free(start, end) }
        fn can_add_node(&mut self, parent: Handle, child: Handle) -> Option<bool> {
            self.asked.push((parent, child));
            Some(!self.rejected.contains(&(parent, child)))
        }
        fn append(&mut self, node: Handle) -> bool {
            self.path.push(node);
            true
        }
    }

    fn search() -> Search {
        Search { physics: Physics::open(), rejected: Vec::new(), asked: Vec::new(), path: Vec::new() }
    }

    fn line(n: usize) -> NavGraph {
        graph(&(0..n).map(|i| (i as f32, 0.0, 0.0)).collect::<Vec<_>>())
    }

    #[test]
    fn lookups_keep_the_first_duplicate() {
        let mut g = NavGraph::new();
        g.add_node(b"a", 7, Vector3f::default(), 0, handle(0));
        g.add_node(b"a", 7, Vector3f::default(), 0, handle(1));
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.from_name(b"a"), handle(0));
        assert_eq!(g.from_id(7), handle(0));
        assert_eq!(g.from_name(b"b"), 0);
        assert_eq!(g.node_handle(1), handle(1));
        assert_eq!(g.node_handle(2), 0);
        assert_eq!(g.node_handle(-1), 0);
    }

    fn collect(g: &NavGraph, at: Vector3f, radius: f32) -> Vec<usize> {
        let mut it = g.iter_begin(at, radius);
        let mut out = Vec::new();
        while g.iter_has_next(&it) {
            out.push(g.iter_next(&mut it).unwrap());
        }
        assert_eq!(g.iter_next(&mut it), None);
        out
    }

    #[test]
    fn iterator_before_build_is_empty_and_after_build_covers_the_cells() {
        let mut g = graph(&[(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (0.0, 0.0, 10.0), (10.0, 0.0, 10.0), (5.0, 0.0, 5.0)]);
        assert!(collect(&g, Vector3f::default(), 100.0).is_empty());
        g.build_grid();
        let mut all = collect(&g, Vector3f::new(5.0, 0.0, 5.0), 100.0);
        all.sort();
        assert_eq!(all, [0, 1, 2, 3, 4]);
        // A small radius only visits the corner cell.
        assert_eq!(collect(&g, Vector3f::default(), 0.1), [0]);
    }

    #[test]
    fn rebuilding_appends_nodes_again() {
        let mut g = graph(&[(0.0, 0.0, 0.0), (1.0, 0.0, 1.0)]);
        g.build_grid();
        g.build_grid();
        let mut all = collect(&g, Vector3f::default(), 100.0);
        all.sort();
        assert_eq!(all, [0, 0, 1, 1]);
    }

    #[test]
    fn empty_build_is_a_no_op() {
        let mut g = NavGraph::new();
        g.build_grid();
        assert!(collect(&g, Vector3f::default(), 1.0).is_empty());
    }

    #[test]
    fn compile_connects_sorts_caps_and_trims() {
        let mut g = line(6);
        g.max_edge_distance = 1.5;
        g.max_edges = 2;
        g.min_edges = 1;
        let mut physics = Physics::open();
        physics.blocked.push((g.node(2).position, g.node(3).position));
        unsafe { NavGraph::compile(&mut g, &mut physics) };
        let targets = |i: usize| g.node(i).edges.iter().map(|e| e.node).collect::<Vec<_>>();
        // Node 0 reaches 1 (d=1) and 2 (d=2, within 2*1.5 but beyond 1.5
        // after index min_edges=1 -> trimmed).
        assert_eq!(targets(0), [handle(1)]);
        // Node 2: 3 is blocked; 1 and 0/4 remain, capped at 2 by distance.
        assert_eq!(targets(2)[0], handle(1));
        assert!(!targets(2).contains(&handle(3)));
        assert_eq!(g.node(0).edges[0].distance, 1.0);
        assert_eq!(g.node(0).edges[0].sqr_distance, 1.0);
    }

    #[test]
    fn compile_stops_at_a_failed_callback() {
        let mut g = line(4);
        let mut physics = Physics::open();
        physics.fail_at = Some(2);
        unsafe { NavGraph::compile(&mut g, &mut physics) };
        assert_eq!(physics.calls.len(), 2);
        // The first visible edge was added; nothing was sorted or trimmed after.
        assert_eq!(g.node(0).edges.len(), 1);
    }

    fn connect(g: &mut NavGraph, pairs: &[(usize, usize)]) {
        for &(a, b) in pairs {
            unsafe {
                let target: *const Node = g.node(b);
                let node: *mut Node = &mut *g.nodes[a];
                NavGraph::add_edge(node, target);
            }
        }
    }

    fn path(g: &mut NavGraph, a: &mut AStar, s: &mut Search, start: Vector3f, goal: Vector3f) -> bool {
        s.path.clear();
        unsafe { AStar::get_path(a, g, start, goal, true, s) }
    }

    #[test]
    fn direct_visibility_skips_the_search() {
        let mut g = line(3);
        g.build_grid();
        let mut a = AStar::new();
        let mut s = search();
        assert!(path(&mut g, &mut a, &mut s, Vector3f::new(0.0, 0.0, 0.0), Vector3f::new(2.0, 0.0, 0.0)));
        assert!(s.path.is_empty());
        assert_eq!(s.physics.calls.len(), 1);
    }

    #[test]
    fn search_finds_the_goal_and_appends_goal_to_start() {
        // A corridor: 0 - 1 - 2 - 3 with a wall between the endpoints.
        let mut g = line(4);
        g.max_edge_distance = 0.6; // candidates within 1.2 of each endpoint
        connect(&mut g, &[(0, 1), (1, 2), (2, 3), (1, 0), (2, 1), (3, 2)]);
        g.build_grid();
        let mut a = AStar::new();
        let mut s = search();
        let (start, goal) = (Vector3f::new(-0.5, 0.0, 0.0), Vector3f::new(3.5, 0.0, 0.0));
        s.physics.blocked.push((start, goal));
        assert!(path(&mut g, &mut a, &mut s, start, goal));
        assert_eq!(s.path, [handle(3), handle(2), handle(1), handle(0)]);
    }

    #[test]
    fn rejected_edges_and_missing_targets_block_the_search() {
        let mut g = line(3);
        g.max_edge_distance = 0.6;
        connect(&mut g, &[(0, 1), (1, 2)]);
        // An edge from a stale .nodes file whose target no longer exists.
        unsafe { NavGraph::push_edge(&mut *g.nodes[0], Edge { distance: 1.0, sqr_distance: 1.0, node: 0 }) };
        g.build_grid();
        let mut a = AStar::new();
        let mut s = search();
        let (start, goal) = (Vector3f::new(-0.5, 0.0, 0.0), Vector3f::new(2.5, 0.0, 0.0));
        s.physics.blocked.push((start, goal));
        s.rejected.push((handle(1), handle(2)));
        assert!(!path(&mut g, &mut a, &mut s, start, goal));
        // The missing target was skipped without asking C++.
        assert!(s.asked.iter().all(|&(_, c)| c != 0));
    }

    #[test]
    fn equal_costs_follow_handle_order() {
        // Two equal routes 0 -> {1, 2} -> 3; the lower handle wins the tie.
        let mut g = graph(&[(0.0, 0.0, 0.0), (1.0, 0.0, 1.0), (1.0, 0.0, -1.0), (2.0, 0.0, 0.0)]);
        g.max_edge_distance = 0.3;
        connect(&mut g, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
        g.build_grid();
        let (start, goal) = (Vector3f::new(-0.2, 0.0, 0.0), Vector3f::new(2.2, 0.0, 0.0));
        for swap in [false, true] {
            if swap {
                // Swap the facade addresses of 1 and 2.
                let (h1, h2) = (g.nodes[1].handle, g.nodes[2].handle);
                g.nodes[1].handle = h2;
                g.nodes[2].handle = h1;
                g.by_handle.insert(h2, 1);
                g.by_handle.insert(h1, 2);
                for n in &mut g.nodes {
                    for e in &mut n.edges {
                        e.node = if e.node == h1 { h2 } else if e.node == h2 { h1 } else { e.node };
                    }
                }
            }
            let mut a = AStar::new();
            let mut s = search();
            s.physics.blocked.push((start, goal));
            assert!(path(&mut g, &mut a, &mut s, start, goal));
            let via = s.path[1];
            assert_eq!(via, handle(1), "lowest address first (swap={swap})");
        }
    }

    #[test]
    fn iteration_limit_stops_the_search() {
        let mut g = line(6);
        g.max_edge_distance = 0.6;
        connect(&mut g, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
        g.build_grid();
        let (start, goal) = (Vector3f::new(-0.5, 0.0, 0.0), Vector3f::new(5.5, 0.0, 0.0));
        let mut a = AStar::new();
        a.max_iterations = 3;
        let mut s = search();
        s.physics.blocked.push((start, goal));
        assert!(!path(&mut g, &mut a, &mut s, start, goal));
        a.max_iterations = -1;
        assert!(path(&mut g, &mut a, &mut s, start, goal));
    }

    /// Starts a nested search on the same handler from the first CanAddNode.
    struct Nested {
        inner: Search,
        graph: *mut NavGraph,
        astar: *mut AStar,
        inner_goal: Vector3f,
        nested: bool,
    }

    impl PathCallbacks for Nested {
        fn free_path(&mut self, start: Vector3f, end: Vector3f) -> Option<bool> { self.inner.free_path(start, end) }
        fn can_add_node(&mut self, parent: Handle, child: Handle) -> Option<bool> {
            if !self.nested {
                self.nested = true;
                let mut inner = search();
                inner.physics.blocked = self.inner.physics.blocked.clone();
                let start = Vector3f::new(-0.5, 0.0, 0.0);
                unsafe { AStar::get_path(self.astar, self.graph, start, self.inner_goal, true, &mut inner) };
            }
            self.inner.can_add_node(parent, child)
        }
        fn append(&mut self, node: Handle) -> bool { self.inner.append(node) }
    }

    fn nested(inner_goal: Vector3f) -> Vec<Handle> {
        let mut g = line(4);
        g.max_edge_distance = 0.6;
        connect(&mut g, &[(0, 1), (1, 2), (2, 3)]);
        g.build_grid();
        let mut a = AStar::new();
        let (start, goal) = (Vector3f::new(-0.5, 0.0, 0.0), Vector3f::new(3.5, 0.0, 0.0));
        let mut cb = Nested { inner: search(), graph: &mut g, astar: &mut a, inner_goal, nested: false };
        cb.inner.physics.blocked.push((start, goal));
        let (gp, ap) = (cb.graph, cb.astar);
        assert!(unsafe { AStar::get_path(ap, gp, start, goal, true, &mut cb) });
        assert!(!unsafe { AStar::is_busy(ap) || NavGraph::is_active(gp) });
        cb.inner.path
    }

    #[test]
    fn a_nested_direct_search_on_the_same_handler_is_allowed() {
        // The inner search sees a free direct path and only clears the goal;
        // the outer one still finds and appends its path.
        assert_eq!(nested(Vector3f::new(0.5, 0.0, 0.0)), [handle(3), handle(2), handle(1), handle(0)]);
    }

    #[test]
    #[should_panic(expected = "reset the search the outer GetPath is using")]
    fn a_nested_search_resetting_the_outer_sets_aborts() {
        // Blocked direct path: the inner search would delete the outer nodes.
        nested(Vector3f::new(3.5, 0.0, 0.0));
    }

    #[test]
    fn a_throwing_callback_stops_and_reports_failure() {
        let mut g = line(4);
        g.max_edge_distance = 0.6;
        connect(&mut g, &[(0, 1), (1, 2), (2, 3)]);
        g.build_grid();
        let (start, goal) = (Vector3f::new(-0.5, 0.0, 0.0), Vector3f::new(3.5, 0.0, 0.0));
        let mut a = AStar::new();
        let mut s = search();
        s.physics.blocked.push((start, goal));
        s.physics.fail_at = Some(2);
        assert!(!path(&mut g, &mut a, &mut s, start, goal));
        assert_eq!(s.physics.calls.len(), 2);
        assert!(!unsafe { AStar::is_busy(&a) });
    }
}
