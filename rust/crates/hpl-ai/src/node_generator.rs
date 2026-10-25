//! Placement decisions from `cAINodeGenerator::Generate` and `OnIntersect`.
//!
//! Original audit: all bodies contribute bounds, using two virtual reads each.
//! Existing nodes survive and IDs restart at zero. Bounds and wall ray offsets
//! are captured in separate phases; grid steps use the original parameter object,
//! while wall thresholds use the generator's live parameters. Wall rays keep a
//! native reference to the node position, and corrections read it after the ray.
//! Zero/negative/nonfinite steps can hang, but callbacks can repair them; no guard
//! changes that behavior. Invalidated iterators/pointers and signed ID overflow
//! remain invalid native inputs. This slice introduces no deviations.

use hpl_math::Vector3f;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackFailed;

/// Native ownership and virtual calls stay behind this non-owning host. Failed
/// callbacks stop immediately so C++ can rethrow its saved exception after Rust
/// returns. None denotes exhaustion/no hit, distinct from a callback failure.
pub trait PlacementHost {
    type Node: Copy;
    fn next_bounds(&mut self) -> Result<Option<(Vector3f, Vector3f)>, CallbackFailed>;
    fn grid_size(&self) -> f32;
    fn minimum(&self) -> Vector3f;
    fn maximum(&self) -> Vector3f;
    fn cast_down(&mut self, start: Vector3f, end: Vector3f) -> bool;
    fn step_size(&self) -> f32;
    /// Advance the native iterator only after the preceding node's four rays.
    fn next_node(&mut self) -> Option<Self::Node>;
    fn wall_distance(&self) -> f32;
    fn cast_wall(&mut self, node: Self::Node, offset: Vector3f) -> Result<Option<f32>, CallbackFailed>;
    fn position(&self, node: Self::Node) -> Vector3f;
    fn set_position(&mut self, node: Self::Node, position: Vector3f);
}

pub fn place_nodes<H: PlacementHost>(host: &mut H) {
    let mut minimum = Vector3f::splat(100000.0);
    let mut maximum = Vector3f::splat(-100000.0);
    loop {
        let bounds = match host.next_bounds() { Ok(bounds) => bounds, Err(CallbackFailed) => return };
        let Some((low, high)) = bounds else { break; };
        // Faithful: ordered comparisons ignore NaN components and keep the first
        // equal value (including its signed zero), unlike f32::min/max.
        if minimum.x > low.x { minimum.x = low.x; }
        if maximum.x < high.x { maximum.x = high.x; }
        if minimum.y > low.y { minimum.y = low.y; }
        if maximum.y < high.y { maximum.y = high.y; }
        if minimum.z > low.z { minimum.z = low.z; }
        if maximum.z < high.z { maximum.z = high.z; }
    }
    minimum.x += host.grid_size();
    minimum.z += host.grid_size();
    maximum.x -= host.grid_size();
    maximum.z -= host.grid_size();

    if minimum.x < host.minimum().x { minimum.x = host.minimum().x; }
    if maximum.x > host.maximum().x { maximum.x = host.maximum().x; }
    if minimum.y < host.minimum().y { minimum.y = host.minimum().y; }
    if maximum.y > host.maximum().y { maximum.y = host.maximum().y; }
    if minimum.z < host.minimum().z { minimum.z = host.minimum().z; }
    if maximum.z > host.maximum().z { maximum.z = host.maximum().z; }

    let mut position = Vector3f::new(minimum.x, 0.0, minimum.z);
    // Faithful: only z gates entry; even an inverted x interval casts once per row.
    while position.z <= maximum.z {
        let start = Vector3f::new(position.x, maximum.y, position.z);
        let end = Vector3f::new(position.x, minimum.y, position.z);
        if !host.cast_down(start, end) { return; }
        position.x += host.step_size();
        if position.x > maximum.x {
            position.x = minimum.x;
            position.z += host.step_size();
        }
    }

    // Faithful: offsets are captured after all placement callbacks; later wall
    // thresholds stay live. Native ray starts remain references into each node.
    let ends = [Vector3f::new(host.wall_distance(), 0.0, 0.0), Vector3f::new(-host.wall_distance(), 0.0, 0.0),
        Vector3f::new(0.0, 0.0, host.wall_distance()), Vector3f::new(0.0, 0.0, -host.wall_distance())];
    let directions = [Vector3f::new(-1.0, 0.0, 0.0), Vector3f::new(1.0, 0.0, 0.0),
        Vector3f::new(0.0, 0.0, -1.0), Vector3f::new(0.0, 0.0, 1.0)];
    while let Some(node) = host.next_node() {
        for index in 0..4 {
            let hit = match host.cast_wall(node, ends[index]) { Ok(hit) => hit, Err(CallbackFailed) => return };
            if let Some(distance) = hit {
                if distance < host.wall_distance() {
                    // Faithful: compute every vector component, including 0*Inf
                    // and signed zero, then add to the live post-ray position.
                    let push = directions[index] * (host.wall_distance() - distance);
                    host.set_position(node, host.position(node) + push);
                }
            }
        }
    }
}

pub trait IntersectionHost {
    fn mass(&mut self) -> Option<f32>;
    fn point(&self) -> Vector3f;
    fn height(&self) -> f32;
    /// Native append captures the current ID in its temporary before allocation,
    /// then increments the live counter only after push_back succeeds.
    fn append(&mut self, position: Vector3f) -> bool;
}

pub fn on_intersect<H: IntersectionHost>(host: &mut H) -> bool {
    let Some(mass) = host.mass() else { return false; };
    // Faithful: nonzero, NaN and infinite masses skip; either signed zero accepts.
    if mass != 0.0 { return true; }
    // Faithful: x/z also add zero, preserving the original signed-zero behavior.
    let position = host.point() + Vector3f::new(0.0, host.height(), 0.0);
    host.append(position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn bits(v: Vector3f) -> [u32; 3] { [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()] }
    fn v(x: f32, y: f32, z: f32) -> Vector3f { Vector3f::new(x, y, z) }

    #[derive(Debug, PartialEq)]
    enum Event {
        Bounds(usize), Down([u32; 3], [u32; 3]), Node(usize),
        Wall(usize, [u32; 3], [u32; 3]), Set(usize, [u32; 3]),
    }
    struct Fixture {
        bounds: Vec<(Vector3f, Vector3f)>, bounds_cursor: usize, fail_bounds: Option<usize>,
        grid: f32, step: f32, minimum: Vector3f, maximum: Vector3f,
        nodes: Vec<Vector3f>, node_cursor: usize, wall_distance: f32,
        hits: Vec<Result<Option<f32>, CallbackFailed>>, down_count: usize, wall_count: usize,
        fail_down: Option<usize>, on_down: Option<fn(&mut Fixture, usize)>,
        on_wall: Option<fn(&mut Fixture, usize, usize)>, events: Vec<Event>,
    }
    impl Fixture {
        fn grid() -> Self {
            Self { bounds: vec![(v(-1.0, -2.0, -1.0), v(3.0, 3.0, 2.0))], bounds_cursor: 0, fail_bounds: None,
                grid: 1.0, step: 1.0, minimum: Vector3f::splat(-10000.0), maximum: Vector3f::splat(10000.0),
                nodes: Vec::new(), node_cursor: 0, wall_distance: 1.0, hits: Vec::new(), down_count: 0,
                wall_count: 0, fail_down: None, on_down: None, on_wall: None, events: Vec::new() }
        }
        fn walls() -> Self {
            let mut fixture = Self::grid();
            fixture.bounds.clear(); fixture.nodes.push(Vector3f::splat(0.0));
            fixture
        }
        fn rays(&self) -> Vec<(Vector3f, Vector3f)> {
            self.events.iter().filter_map(|event| match event {
                Event::Down(a, b) => Some((v(f32::from_bits(a[0]), f32::from_bits(a[1]), f32::from_bits(a[2])),
                    v(f32::from_bits(b[0]), f32::from_bits(b[1]), f32::from_bits(b[2])))), _ => None,
            }).collect()
        }
        fn offsets(&self) -> Vec<[u32; 3]> {
            self.events.iter().filter_map(|event| match event { Event::Wall(_, _, offset) => Some(*offset), _ => None }).collect()
        }
        fn set_count(&self) -> usize { self.events.iter().filter(|event| matches!(event, Event::Set(_, _))).count() }
    }
    impl PlacementHost for Fixture {
        type Node = usize;
        fn next_bounds(&mut self) -> Result<Option<(Vector3f, Vector3f)>, CallbackFailed> {
            let index = self.bounds_cursor;
            self.events.push(Event::Bounds(index)); self.bounds_cursor += 1;
            if self.fail_bounds == Some(index) { Err(CallbackFailed) } else { Ok(self.bounds.get(index).copied()) }
        }
        fn grid_size(&self) -> f32 { self.grid }
        fn minimum(&self) -> Vector3f { self.minimum }
        fn maximum(&self) -> Vector3f { self.maximum }
        fn cast_down(&mut self, start: Vector3f, end: Vector3f) -> bool {
            let index = self.down_count;
            assert!(index < 1000, "fixture unexpectedly entered a nonterminating placement loop");
            self.down_count += 1; self.events.push(Event::Down(bits(start), bits(end)));
            if let Some(action) = self.on_down { action(self, index); }
            self.fail_down != Some(index)
        }
        fn step_size(&self) -> f32 { self.step }
        fn next_node(&mut self) -> Option<usize> {
            let index = self.node_cursor;
            if index == self.nodes.len() { None } else {
                self.node_cursor += 1; self.events.push(Event::Node(index)); Some(index)
            }
        }
        fn wall_distance(&self) -> f32 { self.wall_distance }
        fn cast_wall(&mut self, node: usize, offset: Vector3f) -> Result<Option<f32>, CallbackFailed> {
            let index = self.wall_count;
            self.wall_count += 1; self.events.push(Event::Wall(node, bits(self.nodes[node]), bits(offset)));
            if let Some(action) = self.on_wall { action(self, node, index); }
            self.hits.get(index).copied().unwrap_or(Ok(None))
        }
        fn position(&self, node: usize) -> Vector3f { self.nodes[node] }
        fn set_position(&mut self, node: usize, position: Vector3f) {
            self.events.push(Event::Set(node, bits(position))); self.nodes[node] = position;
        }
    }

    #[test]
    fn grid_visits_x_before_z_and_includes_both_boundaries() {
        let mut fixture = Fixture::grid(); place_nodes(&mut fixture);
        let expected: Vec<_> = [0.0, 1.0].into_iter().flat_map(|z| [0.0, 1.0, 2.0].map(|x|
            (v(x, 3.0, z), v(x, -2.0, z)))).collect();
        assert_eq!(fixture.rays(), expected);
        assert_eq!(fixture.wall_count, 0);
    }

    #[test]
    fn bounds_ignore_nan_and_keep_the_first_equal_signed_zero() {
        let mut fixture = Fixture::grid();
        fixture.bounds = vec![(v(-2.0, -0.0, -2.0), v(2.0, -0.0, 2.0)),
            (v(f32::NAN, 0.0, f32::NAN), v(f32::NAN, 0.0, f32::NAN))];
        place_nodes(&mut fixture);
        let rays = fixture.rays();
        assert_eq!(rays.len(), 9);
        assert_eq!(rays[0].0.x, -1.0); assert_eq!(rays[8].0.x, 1.0);
        for (start, end) in rays { assert_eq!(start.y.to_bits(), (-0.0f32).to_bits()); assert_eq!(end.y.to_bits(), (-0.0f32).to_bits()); }
    }

    #[test]
    fn user_limits_clamp_all_components_without_snapping_to_grid() {
        let mut fixture = Fixture::grid();
        fixture.minimum = v(0.5, -1.0, 0.5); fixture.maximum = v(1.5, 2.0, 1.0);
        place_nodes(&mut fixture);
        assert_eq!(fixture.rays(), vec![(v(0.5, 2.0, 0.5), v(0.5, -1.0, 0.5)),
            (v(1.5, 2.0, 0.5), v(1.5, -1.0, 0.5))]);
    }

    #[test]
    fn inverted_x_still_casts_once_per_row_and_inverted_y_casts_upward() {
        let mut fixture = Fixture::grid();
        fixture.minimum = v(3.0, 5.0, -10000.0); fixture.maximum = v(-1.0, -4.0, 10000.0);
        place_nodes(&mut fixture);
        assert_eq!(fixture.rays(), vec![(v(3.0, -4.0, 0.0), v(3.0, 5.0, 0.0)),
            (v(3.0, -4.0, 1.0), v(3.0, 5.0, 1.0))]);
    }

    #[test]
    fn empty_world_still_checks_preexisting_nodes() {
        let mut fixture = Fixture::walls(); fixture.nodes[0] = v(2.0, 3.0, 4.0);
        place_nodes(&mut fixture);
        assert_eq!(fixture.down_count, 0); assert_eq!(fixture.wall_count, 4);
        assert_eq!(fixture.nodes, vec![v(2.0, 3.0, 4.0)]);
    }

    #[test]
    fn placement_callbacks_change_live_steps_but_not_captured_bounds() {
        let mut fixture = Fixture::grid();
        fixture.on_down = Some(|host, index| if index == 0 {
            host.grid = 99.0; host.minimum = Vector3f::splat(1000.0); host.maximum = Vector3f::splat(-1000.0);
            host.step = 0.5;
        });
        place_nodes(&mut fixture);
        let expected: Vec<_> = [0.0, 0.5, 1.0].into_iter().flat_map(|z| [0.0, 0.5, 1.0, 1.5, 2.0].map(|x|
            (v(x, 3.0, z), v(x, -2.0, z)))).collect();
        assert_eq!(fixture.rays(), expected);
    }

    #[test]
    fn zero_and_negative_steps_can_be_repaired_by_later_callbacks() {
        for step in [0.0, -1.0] {
            let mut fixture = Fixture::grid(); fixture.bounds = vec![(v(-1.0, 0.0, -1.0), v(1.0, 0.0, 1.0))];
            fixture.step = step;
            fixture.on_down = Some(|host, index| if index == 1 { host.step = 2.0; });
            place_nodes(&mut fixture);
            assert_eq!(fixture.rays(), vec![(Vector3f::splat(0.0), Vector3f::splat(0.0)),
                (v(step, 0.0, 0.0), v(step, 0.0, 0.0))]);
        }
    }

    #[test]
    fn nonfinite_step_can_be_repaired_before_use() {
        for step in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut fixture = Fixture::grid(); fixture.step = step;
            fixture.on_down = Some(|host, _| host.step = 10.0);
            place_nodes(&mut fixture);
            assert_eq!(fixture.rays(), vec![(v(0.0, 3.0, 0.0), v(0.0, -2.0, 0.0))]);
        }
    }

    #[test]
    fn finite_f32_stagnation_can_be_repaired_without_changing_precision() {
        let mut fixture = Fixture::grid();
        fixture.bounds = vec![(v(16777216.0, 0.0, 0.0), v(16777216.0, 0.0, 0.0))];
        fixture.grid = 0.0; fixture.minimum = Vector3f::splat(-f32::INFINITY); fixture.maximum = Vector3f::splat(f32::INFINITY);
        // The original 100000 minimum sentinel needs a user clamp at this scale.
        fixture.minimum.x = 16777216.0;
        fixture.on_down = Some(|host, index| if index == 1 { host.step = 2.0; });
        place_nodes(&mut fixture);
        assert_eq!(fixture.rays(), vec![(v(16777216.0, 0.0, 0.0), v(16777216.0, 0.0, 0.0)); 2]);
    }

    #[test]
    fn bounds_failure_stops_before_any_placement_or_wall_work() {
        let mut fixture = Fixture::grid(); fixture.fail_bounds = Some(1); fixture.nodes.push(v(1.0, 2.0, 3.0));
        place_nodes(&mut fixture);
        assert_eq!(fixture.events, vec![Event::Bounds(0), Event::Bounds(1)]);
        assert_eq!(fixture.nodes, vec![v(1.0, 2.0, 3.0)]);
    }

    #[test]
    fn placement_failure_keeps_callback_changes_and_skips_wall_pass() {
        let mut fixture = Fixture::grid(); fixture.fail_down = Some(1);
        fixture.on_down = Some(|host, index| host.nodes.push(v(index as f32, 2.0, 3.0)));
        place_nodes(&mut fixture);
        assert_eq!(fixture.down_count, 2); assert_eq!(fixture.wall_count, 0);
        assert_eq!(fixture.nodes, vec![v(0.0, 2.0, 3.0), v(1.0, 2.0, 3.0)]);
    }

    #[test]
    fn wall_directions_and_corrections_run_in_original_order() {
        let mut fixture = Fixture::walls(); fixture.nodes[0] = v(2.0, 3.0, 4.0);
        fixture.hits = vec![Ok(Some(0.25)), Ok(Some(0.5)), Ok(Some(0.75)), Ok(Some(0.0))];
        place_nodes(&mut fixture);
        assert_eq!(fixture.offsets(), [v(1.0, 0.0, 0.0), v(-1.0, 0.0, 0.0), v(0.0, 0.0, 1.0), v(0.0, 0.0, -1.0)].map(bits));
        assert_eq!(fixture.nodes, vec![v(1.75, 3.0, 4.75)]);
        let positions: Vec<_> = fixture.events.iter().filter_map(|event| match event { Event::Wall(_, position, _) => Some(*position), _ => None }).collect();
        assert_eq!(positions, [v(2.0, 3.0, 4.0), v(1.25, 3.0, 4.0), v(1.75, 3.0, 4.0), v(1.75, 3.0, 3.75)].map(bits));
    }

    #[test]
    fn wall_threshold_and_node_position_are_read_after_callback() {
        let mut fixture = Fixture::walls(); fixture.hits = vec![Ok(Some(0.5))];
        fixture.on_wall = Some(|host, node, index| if index == 0 { host.wall_distance = 2.0; host.nodes[node] = v(10.0, 20.0, 30.0); });
        place_nodes(&mut fixture);
        assert_eq!(fixture.nodes, vec![v(8.5, 20.0, 30.0)]);
        assert_eq!(fixture.offsets(), [v(1.0, 0.0, 0.0), v(-1.0, 0.0, 0.0), v(0.0, 0.0, 1.0), v(0.0, 0.0, -1.0)].map(bits));
    }

    #[test]
    fn wall_offsets_capture_changes_from_placement_callbacks() {
        let mut fixture = Fixture::grid(); fixture.nodes.push(Vector3f::splat(0.0));
        fixture.on_down = Some(|host, _| host.wall_distance = 3.0);
        place_nodes(&mut fixture);
        assert_eq!(fixture.offsets(), [v(3.0, 0.0, 0.0), v(-3.0, 0.0, 0.0), v(0.0, 0.0, 3.0), v(0.0, 0.0, -3.0)].map(bits));
    }

    #[test]
    fn equal_greater_nan_and_missing_wall_hits_do_not_move_nodes() {
        let mut fixture = Fixture::walls(); fixture.nodes[0] = v(2.0, 3.0, 4.0);
        fixture.hits = vec![Ok(Some(1.0)), Ok(Some(2.0)), Ok(Some(f32::NAN)), Ok(None)];
        place_nodes(&mut fixture);
        assert_eq!(fixture.nodes, vec![v(2.0, 3.0, 4.0)]); assert_eq!(fixture.set_count(), 0);
    }

    #[test]
    fn infinite_wall_push_computes_zero_components_too() {
        let mut fixture = Fixture::walls(); fixture.wall_distance = f32::INFINITY; fixture.hits = vec![Ok(Some(0.0))];
        place_nodes(&mut fixture);
        assert_eq!(fixture.nodes[0].x, f32::NEG_INFINITY);
        assert!(fixture.nodes[0].y.is_nan()); assert!(fixture.nodes[0].z.is_nan());
    }

    #[test]
    fn wall_push_adds_positive_zero_to_untouched_axes() {
        let mut fixture = Fixture::walls(); fixture.nodes[0] = Vector3f::splat(-0.0); fixture.hits = vec![Ok(Some(0.0))];
        place_nodes(&mut fixture);
        assert_eq!(bits(fixture.nodes[0]), bits(v(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn wall_callbacks_can_append_a_node_during_the_previously_last_node() {
        let mut fixture = Fixture::walls();
        fixture.on_wall = Some(|host, _, index| if index == 0 { host.nodes.push(v(2.0, 3.0, 4.0)); });
        place_nodes(&mut fixture);
        assert_eq!(fixture.wall_count, 8); assert_eq!(fixture.node_cursor, 2);
        assert_eq!(fixture.nodes, vec![Vector3f::splat(0.0), v(2.0, 3.0, 4.0)]);
    }

    #[test]
    fn wall_failure_preserves_earlier_push_and_stops_before_later_rays() {
        let mut fixture = Fixture::walls(); fixture.hits = vec![Ok(Some(0.5)), Err(CallbackFailed)];
        fixture.nodes.push(v(2.0, 3.0, 4.0));
        place_nodes(&mut fixture);
        assert_eq!(fixture.nodes, vec![v(-0.5, 0.0, 0.0), v(2.0, 3.0, 4.0)]);
        assert_eq!(fixture.wall_count, 2); assert_eq!(fixture.node_cursor, 1); assert_eq!(fixture.set_count(), 1);
    }

    struct Intersection {
        mass: Option<f32>, point: Vector3f, height: f32, point_reads: Cell<usize>, height_reads: Cell<usize>,
        on_mass: Option<fn(&mut Intersection)>, output: Vec<Vector3f>, append_ok: bool,
    }
    impl Intersection {
        fn new(mass: Option<f32>) -> Self {
            Self { mass, point: v(-0.0, 2.0, -0.0), height: 0.25, point_reads: Cell::new(0), height_reads: Cell::new(0),
                on_mass: None, output: Vec::new(), append_ok: true }
        }
    }
    impl IntersectionHost for Intersection {
        fn mass(&mut self) -> Option<f32> { if let Some(action) = self.on_mass { action(self); } self.mass }
        fn point(&self) -> Vector3f { self.point_reads.set(self.point_reads.get() + 1); self.point }
        fn height(&self) -> f32 { self.height_reads.set(self.height_reads.get() + 1); self.height }
        fn append(&mut self, position: Vector3f) -> bool { if self.append_ok { self.output.push(position); } self.append_ok }
    }

    #[test]
    fn intersection_accepts_either_zero_mass_and_adds_zero_to_x_and_z() {
        for mass in [0.0, -0.0] {
            let mut host = Intersection::new(Some(mass)); assert!(on_intersect(&mut host));
            assert_eq!(host.output.len(), 1); assert_eq!(bits(host.output[0]), bits(v(0.0, 2.25, 0.0)));
        }
    }

    #[test]
    fn nonzero_and_nonfinite_mass_continue_without_reading_placement_data() {
        for mass in [1.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut host = Intersection::new(Some(mass)); assert!(on_intersect(&mut host));
            assert!(host.output.is_empty()); assert_eq!(host.point_reads.get(), 0); assert_eq!(host.height_reads.get(), 0);
        }
    }

    #[test]
    fn mass_callback_changes_point_and_height_before_intersection_placement() {
        let mut host = Intersection::new(Some(0.0));
        host.on_mass = Some(|host| { host.point = v(3.0, -4.0, 5.0); host.height = 0.5; });
        assert!(on_intersect(&mut host)); assert_eq!(host.output, vec![v(3.0, -3.5, 5.0)]);
    }

    #[test]
    fn intersection_failures_stop_at_mass_or_append() {
        let mut host = Intersection::new(None); assert!(!on_intersect(&mut host));
        assert_eq!(host.point_reads.get(), 0); assert_eq!(host.height_reads.get(), 0); assert!(host.output.is_empty());
        let mut host = Intersection::new(Some(0.0)); host.append_ok = false;
        assert!(!on_intersect(&mut host)); assert!(host.output.is_empty());
        assert_eq!(host.point_reads.get(), 1); assert_eq!(host.height_reads.get(), 1);
    }
}
