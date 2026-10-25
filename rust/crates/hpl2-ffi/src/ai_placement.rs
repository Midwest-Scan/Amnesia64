//! AI node placement; native iterators, bodies, ray callbacks and lists stay C++.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_ai::node_generator::{self, CallbackFailed, IntersectionHost, PlacementHost};
use hpl_math::Vector3f;
use std::ffi::c_void;

/// All callbacks are required. False success denotes a saved native exception;
/// presence is a separate out bool. Null node handles mean iterator exhaustion.
/// No native reference is borrowed into Rust across a callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAINodePlacementHost {
    pub ctx: *mut c_void,
    pub next_bounds: unsafe extern "C" fn(ctx: *mut c_void, present: *mut bool, minimum: *mut HplVector3f, maximum: *mut HplVector3f) -> bool,
    pub grid_size: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub minimum: unsafe extern "C" fn(ctx: *mut c_void) -> HplVector3f,
    pub maximum: unsafe extern "C" fn(ctx: *mut c_void) -> HplVector3f,
    pub cast_down: unsafe extern "C" fn(ctx: *mut c_void, start: HplVector3f, end: HplVector3f) -> bool,
    pub step_size: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub next_node: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    pub wall_distance: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub cast_wall: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void, offset: HplVector3f, hit: *mut bool, distance: *mut f32) -> bool,
    pub position: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void) -> HplVector3f,
    pub set_position: unsafe extern "C" fn(ctx: *mut c_void, node: *mut c_void, position: HplVector3f),
}

struct Placement(HplAINodePlacementHost);

impl PlacementHost for Placement {
    type Node = *mut c_void;
    fn next_bounds(&mut self) -> Result<Option<(Vector3f, Vector3f)>, CallbackFailed> {
        let mut present = false;
        let mut minimum = HplVector3f::from_rs(Vector3f::splat(0.0));
        let mut maximum = minimum;
        if !unsafe { (self.0.next_bounds)(self.0.ctx, &mut present, &mut minimum, &mut maximum) } { return Err(CallbackFailed); }
        Ok(if present { Some((minimum.to_rs(), maximum.to_rs())) } else { None })
    }
    fn grid_size(&self) -> f32 { unsafe { (self.0.grid_size)(self.0.ctx) } }
    fn minimum(&self) -> Vector3f { unsafe { (self.0.minimum)(self.0.ctx).to_rs() } }
    fn maximum(&self) -> Vector3f { unsafe { (self.0.maximum)(self.0.ctx).to_rs() } }
    fn cast_down(&mut self, start: Vector3f, end: Vector3f) -> bool {
        unsafe { (self.0.cast_down)(self.0.ctx, HplVector3f::from_rs(start), HplVector3f::from_rs(end)) }
    }
    fn step_size(&self) -> f32 { unsafe { (self.0.step_size)(self.0.ctx) } }
    fn next_node(&mut self) -> Option<Self::Node> {
        let node = unsafe { (self.0.next_node)(self.0.ctx) };
        if node.is_null() { None } else { Some(node) }
    }
    fn wall_distance(&self) -> f32 { unsafe { (self.0.wall_distance)(self.0.ctx) } }
    fn cast_wall(&mut self, node: Self::Node, offset: Vector3f) -> Result<Option<f32>, CallbackFailed> {
        let mut hit = false;
        let mut distance = 0.0;
        if !unsafe { (self.0.cast_wall)(self.0.ctx, node, HplVector3f::from_rs(offset), &mut hit, &mut distance) } {
            return Err(CallbackFailed);
        }
        Ok(if hit { Some(distance) } else { None })
    }
    fn position(&self, node: Self::Node) -> Vector3f { unsafe { (self.0.position)(self.0.ctx, node).to_rs() } }
    fn set_position(&mut self, node: Self::Node, position: Vector3f) {
        unsafe { (self.0.set_position)(self.0.ctx, node, HplVector3f::from_rs(position)) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_generator_place(host: *const HplAINodePlacementHost) {
    let host = unsafe { host.read() };
    node_generator::place_nodes(&mut Placement(host));
}

/// GetMass remains a virtual native call; point/height are read only afterward.
/// Append retains native temporary construction, insertion and ID increment.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAINodeIntersectionHost {
    pub ctx: *mut c_void,
    pub mass: unsafe extern "C" fn(ctx: *mut c_void, mass: *mut f32) -> bool,
    pub point: unsafe extern "C" fn(ctx: *mut c_void) -> HplVector3f,
    pub height: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub append: unsafe extern "C" fn(ctx: *mut c_void, position: HplVector3f) -> bool,
}

struct Intersection(HplAINodeIntersectionHost);

impl IntersectionHost for Intersection {
    fn mass(&mut self) -> Option<f32> {
        let mut mass = 0.0;
        if unsafe { (self.0.mass)(self.0.ctx, &mut mass) } { Some(mass) } else { None }
    }
    fn point(&self) -> Vector3f { unsafe { (self.0.point)(self.0.ctx).to_rs() } }
    fn height(&self) -> f32 { unsafe { (self.0.height)(self.0.ctx) } }
    fn append(&mut self, position: Vector3f) -> bool { unsafe { (self.0.append)(self.0.ctx, HplVector3f::from_rs(position)) } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_node_generator_intersect(host: *const HplAINodeIntersectionHost) -> bool {
    let host = unsafe { host.read() };
    node_generator::on_intersect(&mut Intersection(host))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector(x: f32, y: f32, z: f32) -> HplVector3f { HplVector3f { x, y, z } }
    fn bits(v: HplVector3f) -> [u32; 3] { [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()] }

    #[derive(Clone, Debug, PartialEq)]
    enum Event { Bounds, Down([u32; 3], [u32; 3]), Next, Wall(usize, [u32; 3]), Set(usize, [u32; 3]) }
    struct NativePlacement {
        bounds: Vec<(HplVector3f, HplVector3f)>,
        bounds_index: usize,
        fail_bounds: Option<usize>,
        step: f32,
        changed_step: Option<f32>,
        fail_down: bool,
        nodes: [HplVector3f; 2],
        node_count: usize,
        node_index: usize,
        wall_distance: f32,
        wall_hits: [Option<f32>; 4],
        wall_count: usize,
        fail_wall: Option<usize>,
        change_wall: bool,
        position_reads: usize,
        events: Vec<Event>,
    }
    impl NativePlacement {
        fn new() -> Self {
            Self { bounds: vec![(vector(-1.0, -2.0, -1.0), vector(2.0, 3.0, 1.0))], bounds_index: 0,
                fail_bounds: None, step: 1.0, changed_step: None, fail_down: false,
                nodes: [vector(1.0, 2.0, 3.0), vector(4.0, 5.0, 6.0)], node_count: 1, node_index: 0,
                wall_distance: 1.0, wall_hits: [None; 4], wall_count: 0, fail_wall: None, change_wall: false,
                position_reads: 0, events: Vec::new() }
        }
        fn node_index(&self, node: *mut c_void) -> usize {
            unsafe { node.cast::<HplVector3f>().offset_from(self.nodes.as_ptr()) as usize }
        }
    }
    fn placement_host(native: &mut NativePlacement) -> HplAINodePlacementHost {
        unsafe extern "C" fn next_bounds(ctx: *mut c_void, present: *mut bool, low: *mut HplVector3f, high: *mut HplVector3f) -> bool {
            let native = unsafe { &mut *ctx.cast::<NativePlacement>() };
            native.events.push(Event::Bounds);
            if native.fail_bounds == Some(native.bounds_index) {
                // A failed callback may already have changed its out parameters.
                unsafe { *present = true; *low = vector(-10.0, -10.0, -10.0); *high = vector(10.0, 10.0, 10.0); }
                return false;
            }
            if let Some(&(minimum, maximum)) = native.bounds.get(native.bounds_index) {
                native.bounds_index += 1;
                unsafe { *present = true; *low = minimum; *high = maximum; }
            } else {
                // Absent bounds need not initialize the unused vector outputs.
                unsafe { *present = false; }
            }
            true
        }
        unsafe extern "C" fn grid_size(_: *mut c_void) -> f32 { 1.0 }
        unsafe extern "C" fn minimum(_: *mut c_void) -> HplVector3f { vector(-10000.0, -10000.0, -10000.0) }
        unsafe extern "C" fn maximum(_: *mut c_void) -> HplVector3f { vector(10000.0, 10000.0, 10000.0) }
        unsafe extern "C" fn cast_down(ctx: *mut c_void, start: HplVector3f, end: HplVector3f) -> bool {
            let native = unsafe { &mut *ctx.cast::<NativePlacement>() };
            native.events.push(Event::Down(bits(start), bits(end)));
            if let Some(step) = native.changed_step { native.step = step; }
            !native.fail_down
        }
        unsafe extern "C" fn step_size(ctx: *mut c_void) -> f32 { unsafe { (*ctx.cast::<NativePlacement>()).step } }
        unsafe extern "C" fn next_node(ctx: *mut c_void) -> *mut c_void {
            let native = unsafe { &mut *ctx.cast::<NativePlacement>() };
            native.events.push(Event::Next);
            if native.node_index == native.node_count { return std::ptr::null_mut(); }
            let index = native.node_index;
            native.node_index += 1;
            unsafe { native.nodes.as_mut_ptr().add(index).cast() }
        }
        unsafe extern "C" fn wall_distance(ctx: *mut c_void) -> f32 { unsafe { (*ctx.cast::<NativePlacement>()).wall_distance } }
        unsafe extern "C" fn cast_wall(ctx: *mut c_void, node: *mut c_void, offset: HplVector3f, hit: *mut bool, distance: *mut f32) -> bool {
            let native = unsafe { &mut *ctx.cast::<NativePlacement>() };
            let index = native.node_index(node);
            let call = native.wall_count;
            native.wall_count += 1;
            native.events.push(Event::Wall(index, bits(offset)));
            if native.fail_wall == Some(call) {
                unsafe { *hit = true; *distance = 0.0; }
                return false;
            }
            if native.change_wall && call == 0 {
                native.nodes[index] = vector(9.0, 8.0, 7.0);
                native.wall_distance = 2.0;
            }
            if let Some(value) = native.wall_hits[call % 4] {
                unsafe { *hit = true; *distance = value; }
            } else {
                // No hit intentionally leaves distance untouched.
                unsafe { *hit = false; }
            }
            true
        }
        unsafe extern "C" fn position(ctx: *mut c_void, node: *mut c_void) -> HplVector3f {
            let native = unsafe { &mut *ctx.cast::<NativePlacement>() };
            native.position_reads += 1;
            native.nodes[native.node_index(node)]
        }
        unsafe extern "C" fn set_position(ctx: *mut c_void, node: *mut c_void, value: HplVector3f) {
            let native = unsafe { &mut *ctx.cast::<NativePlacement>() };
            let index = native.node_index(node);
            native.nodes[index] = value;
            native.events.push(Event::Set(index, bits(value)));
        }
        HplAINodePlacementHost { ctx: (native as *mut NativePlacement).cast(), next_bounds, grid_size, minimum, maximum,
            cast_down, step_size, next_node, wall_distance, cast_wall, position, set_position }
    }

    #[test]
    fn placement_ffi_preserves_vectors_ray_order_and_distinct_native_node_handles() {
        let mut native = NativePlacement::new();
        native.node_count = 2;
        native.wall_hits[0] = Some(0.25);
        unsafe { hpl_ai_node_generator_place(&placement_host(&mut native)); }
        assert_eq!(&native.events[..4], [Event::Bounds, Event::Bounds,
            Event::Down(bits(vector(0.0, 3.0, 0.0)), bits(vector(0.0, -2.0, 0.0))),
            Event::Down(bits(vector(1.0, 3.0, 0.0)), bits(vector(1.0, -2.0, 0.0)))]);
        assert_eq!(bits(native.nodes[0]), bits(vector(0.25, 2.0, 3.0)));
        assert_eq!(bits(native.nodes[1]), bits(vector(3.25, 5.0, 6.0)));
        assert_eq!((native.wall_count, native.position_reads), (8, 2));
        assert_eq!(native.events.last(), Some(&Event::Next));
    }

    #[test]
    fn bounds_presence_and_failure_are_independent() {
        let mut native = NativePlacement::new();
        native.bounds.clear(); native.node_count = 0;
        unsafe { hpl_ai_node_generator_place(&placement_host(&mut native)); }
        assert_eq!(native.events, [Event::Bounds, Event::Next]);
        for failure in [0, 1] {
            let mut native = NativePlacement::new();
            native.fail_bounds = Some(failure);
            unsafe { hpl_ai_node_generator_place(&placement_host(&mut native)); }
            assert_eq!(native.events, vec![Event::Bounds; failure + 1]);
        }
    }

    #[test]
    fn downcast_failure_stops_before_steps_or_wall_iteration() {
        let mut native = NativePlacement::new();
        native.fail_down = true;
        unsafe { hpl_ai_node_generator_place(&placement_host(&mut native)); }
        assert_eq!(native.events.len(), 3);
        assert_eq!((native.wall_count, native.node_index, native.position_reads), (0, 0, 0));
    }

    #[test]
    fn wall_failure_ignores_partial_hit_outputs_and_stops_without_advancing() {
        let mut native = NativePlacement::new();
        native.fail_wall = Some(1);
        unsafe { hpl_ai_node_generator_place(&placement_host(&mut native)); }
        assert_eq!((native.wall_count, native.position_reads, native.node_index), (2, 0, 1));
        assert_eq!(bits(native.nodes[0]), bits(vector(1.0, 2.0, 3.0)));
        assert_eq!(native.events.last(), Some(&Event::Wall(0, bits(vector(-1.0, 0.0, 0.0)))));
    }

    #[test]
    fn wall_callback_changes_live_position_and_threshold_but_not_captured_offsets() {
        let mut native = NativePlacement::new();
        native.change_wall = true;
        native.wall_hits = [Some(1.5), None, Some(2.0), None];
        unsafe { hpl_ai_node_generator_place(&placement_host(&mut native)); }
        assert_eq!(bits(native.nodes[0]), bits(vector(8.5, 8.0, 7.0)));
        assert_eq!(native.position_reads, 1);
        let offsets: Vec<_> = native.events.iter().filter_map(|e| if let Event::Wall(_, v) = e { Some(*v) } else { None }).collect();
        assert_eq!(offsets, [bits(vector(1.0, 0.0, 0.0)), bits(vector(-1.0, 0.0, 0.0)),
            bits(vector(0.0, 0.0, 1.0)), bits(vector(0.0, 0.0, -1.0))]);
    }

    #[test]
    fn downcast_changes_the_next_step_and_null_node_skips_wall_callbacks() {
        let mut native = NativePlacement::new();
        native.changed_step = Some(2.0); native.node_count = 0;
        unsafe { hpl_ai_node_generator_place(&placement_host(&mut native)); }
        assert_eq!(native.events.len(), 4);
        assert_eq!(native.events.last(), Some(&Event::Next));
        assert_eq!(native.wall_count, 0);
    }

    struct NativeIntersection {
        mass: Option<f32>, point: HplVector3f, height: f32, mutate_mass: bool, fail_append: bool,
        events: Vec<&'static str>, output: Vec<HplVector3f>,
    }
    impl NativeIntersection {
        fn new(mass: Option<f32>) -> Self {
            Self { mass, point: vector(-0.0, 1.0, -0.0), height: 0.25, mutate_mass: false,
                fail_append: false, events: Vec::new(), output: Vec::new() }
        }
    }
    fn intersection_host(native: &mut NativeIntersection) -> HplAINodeIntersectionHost {
        unsafe extern "C" fn mass(ctx: *mut c_void, out: *mut f32) -> bool {
            let native = unsafe { &mut *ctx.cast::<NativeIntersection>() };
            native.events.push("mass");
            if native.mutate_mass { native.point = vector(4.0, 5.0, 6.0); native.height = 2.0; }
            unsafe { *out = native.mass.unwrap_or(0.0); }
            native.mass.is_some()
        }
        unsafe extern "C" fn point(ctx: *mut c_void) -> HplVector3f {
            let native = unsafe { &mut *ctx.cast::<NativeIntersection>() };
            native.events.push("point"); native.point
        }
        unsafe extern "C" fn height(ctx: *mut c_void) -> f32 {
            let native = unsafe { &mut *ctx.cast::<NativeIntersection>() };
            native.events.push("height"); native.height
        }
        unsafe extern "C" fn append(ctx: *mut c_void, position: HplVector3f) -> bool {
            let native = unsafe { &mut *ctx.cast::<NativeIntersection>() };
            native.events.push("append");
            if native.fail_append { return false; }
            native.output.push(position); true
        }
        HplAINodeIntersectionHost { ctx: (native as *mut NativeIntersection).cast(), mass, point, height, append }
    }

    #[test]
    fn intersection_mass_gate_preserves_signed_zero_and_reads_live_values_after_virtual_call() {
        for mass in [1.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut native = NativeIntersection::new(Some(mass));
            assert!(unsafe { hpl_ai_node_generator_intersect(&intersection_host(&mut native)) });
            assert_eq!(native.events, ["mass"]);
            assert!(native.output.is_empty());
        }
        for mass in [0.0, -0.0] {
            let mut native = NativeIntersection::new(Some(mass));
            assert!(unsafe { hpl_ai_node_generator_intersect(&intersection_host(&mut native)) });
            assert_eq!(bits(native.output[0]), bits(vector(0.0, 1.25, 0.0)));
        }
        let mut native = NativeIntersection::new(Some(0.0));
        native.mutate_mass = true;
        assert!(unsafe { hpl_ai_node_generator_intersect(&intersection_host(&mut native)) });
        assert_eq!(bits(native.output[0]), bits(vector(4.0, 7.0, 6.0)));
        assert_eq!(native.events, ["mass", "point", "height", "append"]);
    }

    #[test]
    fn intersection_failure_stops_and_propagates_without_inventing_a_hit() {
        let mut native = NativeIntersection::new(None);
        assert!(!unsafe { hpl_ai_node_generator_intersect(&intersection_host(&mut native)) });
        assert_eq!(native.events, ["mass"]);
        assert!(native.output.is_empty());
        let mut native = NativeIntersection::new(Some(0.0));
        native.fail_append = true;
        assert!(!unsafe { hpl_ai_node_generator_intersect(&intersection_host(&mut native)) });
        assert_eq!(native.events, ["mass", "point", "height", "append"]);
        assert!(native.output.is_empty());
    }
}
