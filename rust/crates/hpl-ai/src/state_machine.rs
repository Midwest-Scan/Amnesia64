//! Transitions, update dispatch and fixed-step scheduling from `ai/StateMachine.cpp`.
//!
//! Original audit: IDs can differ from map keys; self transitions compare the
//! live current ID before lookup. Leave precedes enter, and current is assigned
//! only after enter returns. Reentrant callbacks can change the current state
//! and IDs; failures keep those side effects without rollback. Update checks
//! active once, then dispatches the selected state's fixed-step loop. Scheduling
//! publishes elapsed time before callbacks and rereads both count and step after
//! each callback. Nested updates share the native counter; failures skip the
//! subtraction. Zero/negative/nonfinite steps may terminate through callbacks,
//! so no cap or validation changes the original loops. Registration, ownership,
//! timer storage and virtual callbacks stay C++; scheduling adds no deviations.
//! The original ChangeState dereferences null when there is no current state;
//! stale/destroyed objects and unregistered/uninitialized state fields are also
//! invalid caller input. Only the initial null dereference is handled below.

/// Live non-owning state handles. Callbacks must not unwind through the bridge;
/// false signals a saved native exception, to be rethrown after Rust returns.
pub trait TransitionHost {
    type State: Copy;
    fn current(&self) -> Option<Self::State>;
    fn state_id(&self, state: Self::State) -> i32;
    fn find_state(&self, id: i32) -> Option<Self::State>;
    fn warn_missing(&mut self, id: i32) -> bool;
    fn leave(&mut self, state: Self::State, next_id: i32) -> bool;
    fn enter(&mut self, state: Self::State, previous_id: i32) -> bool;
    fn set_current(&mut self, state: Self::State);
}

pub fn change_state<H: TransitionHost>(host: &mut H, id: i32) {
    // Deviation: original ChangeState dereferences null before its later guard.
    // Abort with the engine panic diagnostic instead of invoking C++ UB.
    let current = host.current().expect("AI ChangeState requires a current state");
    if id == host.state_id(current) { return; }

    let Some(target) = host.find_state(id) else {
        host.warn_missing(id);
        return;
    };
    if let Some(current) = host.current() {
        let next_id = host.state_id(target);
        if !host.leave(current, next_id) { return; }
    }
    // Faithful: OnLeaveState may change current or its ID. Do not reuse the
    // pre-callback state when supplying OnEnterState's previous-state argument.
    let previous_id = host.current().map_or(-1, |state| host.state_id(state));
    if !host.enter(target, previous_id) { return; }
    // Faithful: outer assignment overwrites a nested transition from OnEnter.
    host.set_current(target);
}

/// Machine-level dispatch only; the selected state's scheduling owns its backlog.
pub trait UpdateHost {
    type State: Copy;
    fn active(&self) -> bool;
    fn current(&self) -> Option<Self::State>;
    fn update(&mut self, state: Self::State, time: f32) -> bool;
}

pub fn update<H: UpdateHost>(host: &mut H, time: f32) {
    if !host.active() { return; }
    let Some(current) = host.current() else { return; };
    // Faithful: do not gate each tick or switch the selected state mid-backlog.
    host.update(current, time);
}

/// Live access to one state's native timer. No snapshot or Rust reference to
/// timer storage survives a virtual callback. False from on_update means C++
/// saved an exception and must rethrow after this invocation returns.
pub trait ScheduleHost {
    fn time_count(&self) -> f32;
    fn set_time_count(&mut self, count: f32);
    fn update_step(&self) -> f32;
    fn on_update(&mut self, step: f32) -> bool;
}

pub fn schedule<H: ScheduleHost>(host: &mut H, time: f32) {
    // Faithful: publish the f32 addition even if no tick runs or a callback fails.
    host.set_time_count(host.time_count() + time);
    while host.time_count() >= host.update_step() {
        if !host.on_update(host.update_step()) { return; }
        // Faithful: a callback can change the step or run nested updates against
        // this same counter. Subtract only afterward, rereading both live values.
        host.set_time_count(host.time_count() - host.update_step());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug, PartialEq)]
    enum Event {
        Warning(i32),
        Leave { state: usize, next: i32, current: Option<usize> },
        Enter { state: usize, previous: i32, current: Option<usize> },
        Current(usize),
        Update(usize, u32),
    }

    struct Machine {
        ids: [i32; 3],
        keys: Vec<(i32, usize)>,
        current: Option<usize>,
        active: bool,
        lookups: Cell<usize>,
        current_reads: Cell<usize>,
        on_leave: Option<fn(&mut Machine)>,
        on_enter: Option<fn(&mut Machine)>,
        on_update: Option<fn(&mut Machine)>,
        fail_leave: Option<usize>,
        fail_enter: Option<usize>,
        fail_warning: bool,
        fail_update: bool,
        events: Vec<Event>,
    }

    impl Machine {
        fn new() -> Self {
            Self { ids: [10, 20, 30], keys: vec![(10, 0), (20, 1), (30, 2)], current: Some(0), active: true,
                lookups: Cell::new(0), current_reads: Cell::new(0), on_leave: None, on_enter: None, on_update: None,
                fail_leave: None, fail_enter: None, fail_warning: false, fail_update: false, events: Vec::new() }
        }
        fn read_current(&self) -> Option<usize> {
            self.current_reads.set(self.current_reads.get() + 1);
            self.current
        }
    }

    impl TransitionHost for Machine {
        type State = usize;
        fn current(&self) -> Option<usize> { self.read_current() }
        fn state_id(&self, state: usize) -> i32 { self.ids[state] }
        fn find_state(&self, id: i32) -> Option<usize> {
            self.lookups.set(self.lookups.get() + 1);
            self.keys.iter().find_map(|&(key, state)| if key == id { Some(state) } else { None })
        }
        fn warn_missing(&mut self, id: i32) -> bool {
            self.events.push(Event::Warning(id));
            !self.fail_warning
        }
        fn leave(&mut self, state: usize, next: i32) -> bool {
            self.events.push(Event::Leave { state, next, current: self.current });
            if let Some(action) = self.on_leave.take() { action(self); }
            self.fail_leave != Some(state)
        }
        fn enter(&mut self, state: usize, previous: i32) -> bool {
            self.events.push(Event::Enter { state, previous, current: self.current });
            if let Some(action) = self.on_enter.take() { action(self); }
            self.fail_enter != Some(state)
        }
        fn set_current(&mut self, state: usize) {
            self.events.push(Event::Current(state));
            self.current = Some(state);
        }
    }

    impl UpdateHost for Machine {
        type State = usize;
        fn active(&self) -> bool { self.active }
        fn current(&self) -> Option<usize> { self.read_current() }
        fn update(&mut self, state: usize, time: f32) -> bool {
            self.events.push(Event::Update(state, time.to_bits()));
            if let Some(action) = self.on_update.take() { action(self); }
            !self.fail_update
        }
    }

    #[test]
    fn same_live_id_returns_before_lookup_even_when_no_matching_map_key_exists() {
        let mut machine = Machine::new();
        machine.ids[0] = 77;
        change_state(&mut machine, 77);
        assert_eq!(machine.lookups.get(), 0);
        assert!(machine.events.is_empty());
        assert_eq!(machine.current, Some(0));
    }

    #[test]
    fn a_map_key_resolving_to_current_still_transitions_when_its_live_id_differs() {
        let mut machine = Machine::new();
        machine.ids[0] = 77;
        change_state(&mut machine, 10);
        assert_eq!(machine.events, [Event::Leave { state: 0, next: 77, current: Some(0) },
            Event::Enter { state: 0, previous: 77, current: Some(0) }, Event::Current(0)]);
    }

    #[test]
    fn missing_ids_warn_once_without_callbacks_or_assignment_even_when_warning_fails() {
        for (id, fails) in [(i32::MIN, false), (-1, true), (i32::MAX, true)] {
            let mut machine = Machine::new();
            machine.fail_warning = fails;
            change_state(&mut machine, id);
            assert_eq!(machine.events, [Event::Warning(id)]);
            assert_eq!(machine.current, Some(0));
        }
    }

    #[test]
    fn inactive_transitions_leave_then_enter_before_assigning_current() {
        let mut machine = Machine::new();
        machine.active = false;
        change_state(&mut machine, 20);
        assert_eq!(machine.events, [Event::Leave { state: 0, next: 20, current: Some(0) },
            Event::Enter { state: 1, previous: 10, current: Some(0) }, Event::Current(1)]);
        assert_eq!(machine.current, Some(1));
    }

    #[test]
    fn leave_edits_are_live_but_the_target_handle_is_not_looked_up_again() {
        let mut machine = Machine::new();
        machine.on_leave = Some(|machine| {
            machine.ids[1] = 200;
            machine.keys[1] = (20, 2);
            machine.current = Some(2);
            machine.ids[2] = 333;
        });
        change_state(&mut machine, 20);
        assert_eq!(machine.events, [Event::Leave { state: 0, next: 20, current: Some(0) },
            Event::Enter { state: 1, previous: 333, current: Some(2) }, Event::Current(1)]);
        assert_eq!(machine.lookups.get(), 1);
        assert_eq!(machine.current, Some(1));
    }

    #[test]
    fn enter_reads_the_changed_id_even_when_leave_keeps_the_current_handle() {
        let mut machine = Machine::new();
        machine.on_leave = Some(|machine| machine.ids[0] = -8);
        change_state(&mut machine, 20);
        assert_eq!(machine.events[1], Event::Enter { state: 1, previous: -8, current: Some(0) });
    }

    #[test]
    fn nested_leave_transition_supplies_its_result_to_outer_enter() {
        let mut machine = Machine::new();
        machine.on_leave = Some(|machine| change_state(machine, 30));
        change_state(&mut machine, 20);
        assert_eq!(machine.events, [Event::Leave { state: 0, next: 20, current: Some(0) },
            Event::Leave { state: 0, next: 30, current: Some(0) },
            Event::Enter { state: 2, previous: 10, current: Some(0) }, Event::Current(2),
            Event::Enter { state: 1, previous: 30, current: Some(2) }, Event::Current(1)]);
    }

    #[test]
    fn outer_assignment_overwrites_a_nested_enter_transition() {
        let mut machine = Machine::new();
        machine.on_enter = Some(|machine| change_state(machine, 30));
        change_state(&mut machine, 20);
        assert_eq!(machine.events, [Event::Leave { state: 0, next: 20, current: Some(0) },
            Event::Enter { state: 1, previous: 10, current: Some(0) },
            Event::Leave { state: 0, next: 30, current: Some(0) },
            Event::Enter { state: 2, previous: 10, current: Some(0) }, Event::Current(2), Event::Current(1)]);
    }

    #[test]
    fn failed_leave_keeps_its_side_effects_and_skips_enter_and_assignment() {
        let mut machine = Machine::new();
        machine.fail_leave = Some(0);
        machine.on_leave = Some(|machine| machine.current = Some(2));
        change_state(&mut machine, 20);
        assert_eq!(machine.events, [Event::Leave { state: 0, next: 20, current: Some(0) }]);
        assert_eq!(machine.current, Some(2));
    }

    #[test]
    fn failed_enter_keeps_a_successful_nested_transition_without_outer_assignment() {
        let mut machine = Machine::new();
        machine.fail_enter = Some(1);
        machine.on_enter = Some(|machine| change_state(machine, 30));
        change_state(&mut machine, 20);
        assert_eq!(machine.events.last(), Some(&Event::Current(2)));
        assert_eq!(machine.current, Some(2));
        assert_eq!(machine.events.len(), 5);
    }

    #[test]
    fn a_host_with_no_current_after_leave_supplies_the_original_minus_one_sentinel() {
        // Exercises the explicit post-callback null branch independently of
        // native registration/lifetime APIs. Initial current is still valid.
        let mut machine = Machine::new();
        machine.on_leave = Some(|machine| machine.current = None);
        change_state(&mut machine, 20);
        assert_eq!(machine.events[1], Event::Enter { state: 1, previous: -1, current: None });
        assert_eq!(machine.current, Some(1));
    }

    #[test]
    #[should_panic(expected = "AI ChangeState requires a current state")]
    fn initial_null_transition_reports_the_documented_undefined_behavior_deviation() {
        let mut machine = Machine::new();
        machine.current = None;
        change_state(&mut machine, 20);
    }

    #[test]
    fn inactive_dispatch_does_not_read_current_and_empty_dispatch_does_not_update() {
        let mut machine = Machine::new();
        machine.active = false;
        update(&mut machine, 1.0);
        assert_eq!(machine.current_reads.get(), 0);
        assert!(machine.events.is_empty());
        machine.active = true;
        machine.current = None;
        update(&mut machine, 1.0);
        assert_eq!(machine.current_reads.get(), 1);
        assert!(machine.events.is_empty());
    }

    #[test]
    fn dispatch_forwards_all_time_bits_once_without_timer_arithmetic() {
        for bits in [0, 0x80000000, 1, 0x80000001, 0x3eaaaaab, 0xbf800000, 0x7f7fffff,
                     0x7f800000, 0xff800000, 0x7fc12345, 0xffc54321, 0x7f800001] {
            let mut machine = Machine::new();
            machine.current = Some(1);
            update(&mut machine, f32::from_bits(bits));
            assert_eq!(machine.events, [Event::Update(1, bits)]);
        }
    }

    #[test]
    fn dispatch_failure_keeps_callback_changes_without_redispatch_or_rollback() {
        let mut machine = Machine::new();
        machine.fail_update = true;
        machine.on_update = Some(|machine| { machine.current = Some(2); machine.active = false; });
        update(&mut machine, 0.5);
        assert_eq!(machine.events, [Event::Update(0, 0.5_f32.to_bits())]);
        assert_eq!(machine.current, Some(2));
        assert!(!machine.active);
    }
}

#[cfg(test)]
mod schedule_tests {
    use super::{ScheduleHost, schedule};

    #[derive(Debug, PartialEq)]
    enum Event { Store(u32), Tick { step: u32, count: u32, depth: usize } }

    struct Timer {
        count: f32,
        step: f32,
        calls: usize,
        depth: usize,
        fail_at: Option<usize>,
        action: Option<fn(&mut Timer, usize)>,
        events: Vec<Event>,
    }
    impl Timer {
        fn new(count: f32, step: f32) -> Self {
            Self { count, step, calls: 0, depth: 0, fail_at: None, action: None, events: Vec::new() }
        }
        fn nested(&mut self, time: f32) {
            self.depth += 1;
            schedule(self, time);
            self.depth -= 1;
        }
        fn ticks(&self) -> Vec<(u32, u32, usize)> {
            self.events.iter().filter_map(|event| match event {
                Event::Tick { step, count, depth } => Some((*step, *count, *depth)), _ => None,
            }).collect()
        }
        fn stores(&self) -> Vec<u32> {
            self.events.iter().filter_map(|event| match event { Event::Store(count) => Some(*count), _ => None }).collect()
        }
    }
    impl ScheduleHost for Timer {
        fn time_count(&self) -> f32 { self.count }
        fn set_time_count(&mut self, count: f32) { self.count = count; self.events.push(Event::Store(count.to_bits())); }
        fn update_step(&self) -> f32 { self.step }
        fn on_update(&mut self, step: f32) -> bool {
            let index = self.calls;
            assert!(index < 20000, "test fixture unexpectedly entered an unbounded scheduler loop");
            self.calls += 1;
            self.events.push(Event::Tick { step: step.to_bits(), count: self.count.to_bits(), depth: self.depth });
            if let Some(action) = self.action { action(self, index); }
            self.fail_at != Some(index)
        }
    }

    #[test]
    fn elapsed_time_is_published_before_ticks_and_remainder_survives() {
        let mut timer = Timer::new(0.125, 0.25);
        schedule(&mut timer, 0.5);
        assert_eq!(timer.events, [Event::Store(0.625_f32.to_bits()),
            Event::Tick { step: 0.25_f32.to_bits(), count: 0.625_f32.to_bits(), depth: 0 }, Event::Store(0.375_f32.to_bits()),
            Event::Tick { step: 0.25_f32.to_bits(), count: 0.375_f32.to_bits(), depth: 0 }, Event::Store(0.125_f32.to_bits())]);
        schedule(&mut timer, 0.125);
        assert_eq!(timer.calls, 3); assert_eq!(timer.count.to_bits(), 0);
    }

    #[test]
    fn exact_step_runs_but_adjacent_lower_value_waits() {
        let step = 0.5_f32;
        for (time, expected_calls, expected_count) in [
            (f32::from_bits(step.to_bits() - 1), 0, f32::from_bits(step.to_bits() - 1)),
            (step, 1, 0.0), (f32::from_bits(step.to_bits() + 1), 1, f32::from_bits(0x33800000)),
        ] {
            let mut timer = Timer::new(0.0, step);
            schedule(&mut timer, time);
            assert_eq!(timer.calls, expected_calls); assert_eq!(timer.count.to_bits(), expected_count.to_bits());
        }
    }

    #[test]
    fn negative_elapsed_is_retained_and_later_time_repays_it() {
        let mut timer = Timer::new(0.125, 0.25);
        schedule(&mut timer, -0.5);
        assert_eq!(timer.count, -0.375); assert_eq!(timer.calls, 0);
        schedule(&mut timer, 0.625);
        assert_eq!(timer.count, 0.0); assert_eq!(timer.calls, 1);
    }

    #[test]
    fn every_subtraction_rounds_to_f32_before_the_next_callback() {
        let mut timer = Timer::new(0.0, 0.1);
        schedule(&mut timer, 0.3);
        assert_eq!(timer.stores(), [0x3e99999a, 0x3e4cccce, 0x3dcccccf, 0x32800000]);
        assert_eq!(timer.calls, 3);
    }

    #[test]
    fn initial_addition_rounds_before_comparison_and_callback() {
        let mut timer = Timer::new(16777216.0, 16777216.0);
        schedule(&mut timer, 1.0);
        assert_eq!(timer.ticks(), [(16777216.0_f32.to_bits(), 16777216.0_f32.to_bits(), 0)]);
        assert_eq!(timer.count.to_bits(), 0);
    }

    #[test]
    fn nan_counter_step_or_elapsed_prevents_ticks_without_discarding_accumulation() {
        let mut timer = Timer::new(f32::NAN, 0.25);
        schedule(&mut timer, 1.0); assert!(timer.count.is_nan()); assert_eq!(timer.calls, 0);
        let mut timer = Timer::new(0.125, f32::NAN);
        schedule(&mut timer, 0.25); assert_eq!(timer.count, 0.375); assert_eq!(timer.calls, 0);
        let mut timer = Timer::new(0.125, 0.25);
        schedule(&mut timer, f32::NAN); assert!(timer.count.is_nan()); assert_eq!(timer.calls, 0);
    }

    #[test]
    fn matching_infinite_counter_and_step_tick_once_then_become_nan() {
        for infinity in [f32::INFINITY, f32::NEG_INFINITY] {
            let mut timer = Timer::new(infinity, infinity);
            schedule(&mut timer, 0.0);
            assert_eq!(timer.ticks(), [(infinity.to_bits(), infinity.to_bits(), 0)]);
            assert!(timer.count.is_nan());
        }
        let mut timer = Timer::new(1.0, f32::INFINITY);
        schedule(&mut timer, 0.0); assert_eq!(timer.calls, 0); assert_eq!(timer.count, 1.0);
    }

    #[test]
    fn infinite_counter_can_be_rescued_by_an_infinite_callback_step() {
        let mut timer = Timer::new(0.0, 0.25);
        timer.action = Some(|timer, _| timer.step = f32::INFINITY);
        schedule(&mut timer, f32::INFINITY);
        assert_eq!(timer.calls, 1); assert!(timer.count.is_nan());
        assert_eq!(timer.ticks()[0].0, 0.25_f32.to_bits());
    }

    #[test]
    fn zero_and_negative_steps_can_be_repaired_after_multiple_callbacks() {
        for step in [0.0, -0.0, -0.25] {
            let mut timer = Timer::new(0.0, step);
            timer.action = Some(|timer, index| if index == 1 { timer.step = 1.0; });
            schedule(&mut timer, 0.0);
            assert_eq!(timer.calls, 2);
            assert_eq!(timer.count, if step < 0.0 { -0.75 } else { -1.0 });
            assert_eq!(timer.ticks()[0].0, step.to_bits()); assert_eq!(timer.ticks()[1].0, step.to_bits());
        }
    }

    #[test]
    fn negative_infinite_step_can_be_repaired_before_subtraction() {
        let mut timer = Timer::new(0.0, f32::NEG_INFINITY);
        timer.action = Some(|timer, _| timer.step = 1.0);
        schedule(&mut timer, 0.0);
        assert_eq!(timer.calls, 1); assert_eq!(timer.count, -1.0);
        assert_eq!(timer.ticks()[0].0, f32::NEG_INFINITY.to_bits());
    }

    #[test]
    fn finite_f32_stagnation_can_be_repaired_by_a_later_callback() {
        let mut timer = Timer::new(16777216.0, 0.5);
        timer.action = Some(|timer, index| if index == 1 { timer.step = 16777216.0; });
        schedule(&mut timer, 0.0);
        assert_eq!(timer.ticks(), vec![(0.5_f32.to_bits(), 16777216.0_f32.to_bits(), 0); 2]);
        assert_eq!(timer.count, 0.0);
    }

    #[test]
    fn callback_step_is_used_for_subtraction_and_next_comparison() {
        let mut timer = Timer::new(0.0, 0.25);
        timer.action = Some(|timer, _| timer.step = 0.5);
        schedule(&mut timer, 0.75);
        assert_eq!(timer.ticks(), [(0.25_f32.to_bits(), 0.75_f32.to_bits(), 0)]);
        assert_eq!(timer.count, 0.25);
    }

    #[test]
    fn callback_can_reduce_the_step_and_extend_the_backlog() {
        let mut timer = Timer::new(0.0, 0.5);
        timer.action = Some(|timer, index| if index == 0 { timer.step = 0.125; });
        schedule(&mut timer, 0.5);
        assert_eq!(timer.calls, 4); assert_eq!(timer.count, 0.0);
        assert_eq!(timer.ticks().iter().map(|tick| tick.0).collect::<Vec<_>>(),
            [0.5_f32.to_bits(), 0.125_f32.to_bits(), 0.125_f32.to_bits(), 0.125_f32.to_bits()]);
    }

    #[test]
    fn callback_nan_step_is_subtracted_before_the_loop_stops() {
        let mut timer = Timer::new(0.0, 0.25);
        timer.action = Some(|timer, _| timer.step = f32::NAN);
        schedule(&mut timer, 0.5);
        assert_eq!(timer.calls, 1); assert!(timer.count.is_nan());
    }

    #[test]
    fn same_state_nested_update_consumes_the_shared_pre_subtraction_counter() {
        let mut timer = Timer::new(0.0, 0.25);
        timer.action = Some(|timer, index| if index == 0 { timer.nested(0.25); });
        schedule(&mut timer, 0.5);
        assert_eq!(timer.ticks(), [(0.25_f32.to_bits(), 0.5_f32.to_bits(), 0),
            (0.25_f32.to_bits(), 0.75_f32.to_bits(), 1), (0.25_f32.to_bits(), 0.5_f32.to_bits(), 1),
            (0.25_f32.to_bits(), 0.25_f32.to_bits(), 1)]);
        assert_eq!(timer.count, -0.25);
    }

    #[test]
    fn nested_update_step_change_is_used_by_the_outer_subtraction() {
        let mut timer = Timer::new(0.0, 0.25);
        timer.action = Some(|timer, index| match index {
            0 => timer.nested(0.0), 1 => timer.step = 0.5, _ => {},
        });
        schedule(&mut timer, 0.5);
        assert_eq!(timer.ticks(), [(0.25_f32.to_bits(), 0.5_f32.to_bits(), 0), (0.25_f32.to_bits(), 0.5_f32.to_bits(), 1)]);
        assert_eq!(timer.count, -0.5); assert_eq!(timer.step, 0.5);
    }

    #[test]
    fn first_callback_failure_preserves_accumulation_and_callback_changes() {
        let mut timer = Timer::new(0.125, 0.25);
        timer.fail_at = Some(0);
        timer.action = Some(|timer, _| timer.step = 0.5);
        schedule(&mut timer, 0.375);
        assert_eq!(timer.count, 0.5); assert_eq!(timer.step, 0.5); assert_eq!(timer.stores(), [0.5_f32.to_bits()]);
        timer.fail_at = None; timer.action = None;
        schedule(&mut timer, 0.0);
        assert_eq!(timer.calls, 2); assert_eq!(timer.count, 0.0);
    }

    #[test]
    fn later_callback_failure_keeps_completed_deductions_only() {
        let mut timer = Timer::new(0.0, 0.25); timer.fail_at = Some(1);
        schedule(&mut timer, 0.75);
        assert_eq!(timer.calls, 2); assert_eq!(timer.count, 0.5);
        assert_eq!(timer.stores(), [0.75_f32.to_bits(), 0.5_f32.to_bits()]);
    }

    #[test]
    fn caught_nested_failure_leaves_live_count_for_outer_deduction() {
        let mut timer = Timer::new(0.0, 0.25);
        timer.action = Some(|timer, index| if index == 0 {
            timer.fail_at = Some(1); timer.nested(0.25); timer.fail_at = None;
        });
        schedule(&mut timer, 0.5);
        assert_eq!(timer.calls, 4); assert_eq!(timer.count, 0.0);
        assert_eq!(timer.ticks()[1], (0.25_f32.to_bits(), 0.75_f32.to_bits(), 1));
        assert_eq!(timer.ticks()[2], (0.25_f32.to_bits(), 0.5_f32.to_bits(), 0));
    }

    #[test]
    fn propagated_nested_failure_skips_both_unfinished_deductions() {
        let mut timer = Timer::new(0.0, 0.25);
        timer.action = Some(|timer, index| if index == 0 {
            timer.fail_at = Some(1); timer.nested(0.25); timer.fail_at = Some(0);
        });
        schedule(&mut timer, 0.5);
        assert_eq!(timer.calls, 2); assert_eq!(timer.count, 0.75);
        assert_eq!(timer.stores(), [0.5_f32.to_bits(), 0.75_f32.to_bits()]);
    }

    #[test]
    fn finite_backlog_has_no_arbitrary_tick_cap() {
        let mut timer = Timer::new(0.0, 0.125);
        schedule(&mut timer, 1025.0);
        assert_eq!(timer.calls, 8200); assert_eq!(timer.count, 0.0);
    }

    #[test]
    fn subnormal_steps_and_signed_zero_accumulation_keep_float_bits() {
        let mut timer = Timer::new(0.0, f32::from_bits(1));
        schedule(&mut timer, f32::from_bits(3));
        assert_eq!(timer.calls, 3); assert_eq!(timer.stores(), [3, 2, 1, 0]);
        let mut timer = Timer::new(-0.0, f32::NAN);
        schedule(&mut timer, -0.0);
        assert_eq!(timer.count.to_bits(), (-0.0_f32).to_bits()); assert_eq!(timer.calls, 0);
    }
}
