//! Transition and update dispatch decisions from `ai/StateMachine.cpp`.
//!
//! Original audit: IDs can differ from map keys; self transitions compare the
//! live current ID before lookup. Leave precedes enter, and current is assigned
//! only after enter returns. Reentrant callbacks can change the current state
//! and IDs; failures keep those side effects without rollback. Update checks
//! active once, then dispatches the selected state's native fixed-step loop.
//! Registration, object ownership, virtual callbacks and that loop stay C++.
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

/// Machine-level dispatch only; C++ owns the state's timer and virtual loop.
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
