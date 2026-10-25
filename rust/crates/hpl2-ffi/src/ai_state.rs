//! AI transitions, dispatch and scheduling. Objects, timers and vtables stay C++.

#![allow(clippy::missing_safety_doc)]

use hpl_ai::state_machine::{self, ScheduleHost, TransitionHost, UpdateHost};
use std::ffi::c_void;

/// Every callback is required and must not throw across Rust. Null state
/// handles represent absence. Leave/enter/warning return false on a saved
/// exception; C++ rethrows only after this invocation returns.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAIStateTransitionHost {
    pub ctx: *mut c_void,
    pub current: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    pub state_id: unsafe extern "C" fn(ctx: *mut c_void, state: *mut c_void) -> i32,
    pub find_state: unsafe extern "C" fn(ctx: *mut c_void, id: i32) -> *mut c_void,
    pub warn_missing: unsafe extern "C" fn(ctx: *mut c_void, id: i32) -> bool,
    pub leave: unsafe extern "C" fn(ctx: *mut c_void, state: *mut c_void, next_id: i32) -> bool,
    pub enter: unsafe extern "C" fn(ctx: *mut c_void, state: *mut c_void, previous_id: i32) -> bool,
    pub set_current: unsafe extern "C" fn(ctx: *mut c_void, state: *mut c_void),
}

/// Dispatches the selected state once into its fixed-step update loop.
/// All callbacks are required; update catches and reports native exceptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAIStateUpdateHost {
    pub ctx: *mut c_void,
    pub active: unsafe extern "C" fn(ctx: *mut c_void) -> bool,
    pub current: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    pub update: unsafe extern "C" fn(ctx: *mut c_void, state: *mut c_void, time: f32) -> bool,
}

fn state_option(state: *mut c_void) -> Option<*mut c_void> {
    if state.is_null() { None } else { Some(state) }
}

struct Transition(HplAIStateTransitionHost);

impl TransitionHost for Transition {
    type State = *mut c_void;
    fn current(&self) -> Option<Self::State> { state_option(unsafe { (self.0.current)(self.0.ctx) }) }
    fn state_id(&self, state: Self::State) -> i32 { unsafe { (self.0.state_id)(self.0.ctx, state) } }
    fn find_state(&self, id: i32) -> Option<Self::State> { state_option(unsafe { (self.0.find_state)(self.0.ctx, id) }) }
    fn warn_missing(&mut self, id: i32) -> bool { unsafe { (self.0.warn_missing)(self.0.ctx, id) } }
    fn leave(&mut self, state: Self::State, next_id: i32) -> bool { unsafe { (self.0.leave)(self.0.ctx, state, next_id) } }
    fn enter(&mut self, state: Self::State, previous_id: i32) -> bool { unsafe { (self.0.enter)(self.0.ctx, state, previous_id) } }
    fn set_current(&mut self, state: Self::State) { unsafe { (self.0.set_current)(self.0.ctx, state) } }
}

struct Update(HplAIStateUpdateHost);

impl UpdateHost for Update {
    type State = *mut c_void;
    fn active(&self) -> bool { unsafe { (self.0.active)(self.0.ctx) } }
    fn current(&self) -> Option<Self::State> { state_option(unsafe { (self.0.current)(self.0.ctx) }) }
    fn update(&mut self, state: Self::State, time: f32) -> bool { unsafe { (self.0.update)(self.0.ctx, state, time) } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_state_change(id: i32, host: *const HplAIStateTransitionHost) {
    let host = unsafe { host.read() };
    state_machine::change_state(&mut Transition(host), id);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_state_update(time: f32, host: *const HplAIStateUpdateHost) {
    let host = unsafe { host.read() };
    state_machine::update(&mut Update(host), time);
}

/// Every callback is required. Native timer storage is read/written at each
/// original operation; on_update catches exceptions and returns false on failure.
/// Each nested call uses its own host/exception slot and the same live state.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplAIStateScheduleHost {
    pub ctx: *mut c_void,
    pub time_count: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub set_time_count: unsafe extern "C" fn(ctx: *mut c_void, count: f32),
    pub update_step: unsafe extern "C" fn(ctx: *mut c_void) -> f32,
    pub on_update: unsafe extern "C" fn(ctx: *mut c_void, step: f32) -> bool,
}

struct Schedule(HplAIStateScheduleHost);

impl ScheduleHost for Schedule {
    fn time_count(&self) -> f32 { unsafe { (self.0.time_count)(self.0.ctx) } }
    fn set_time_count(&mut self, count: f32) { unsafe { (self.0.set_time_count)(self.0.ctx, count) } }
    fn update_step(&self) -> f32 { unsafe { (self.0.update_step)(self.0.ctx) } }
    fn on_update(&mut self, step: f32) -> bool { unsafe { (self.0.on_update)(self.0.ctx, step) } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_ai_state_schedule(time: f32, host: *const HplAIStateScheduleHost) {
    let host = unsafe { host.read() };
    state_machine::schedule(&mut Schedule(host), time);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq)]
    enum Failure { None, Warning, Leave, Enter, Update }

    #[derive(Debug, PartialEq)]
    enum Event {
        Warning(i32),
        Leave(usize, i32, Option<usize>),
        Enter(usize, i32, Option<usize>),
        Current(usize),
        Update(usize, u32),
    }

    struct Native {
        states: [u8; 3],
        ids: [i32; 3],
        current: Option<usize>,
        active: bool,
        change_on_leave: bool,
        current_reads: usize,
        failure: Failure,
        events: Vec<Event>,
    }

    impl Native {
        fn new() -> Self {
            Self { states: [0; 3], ids: [10, 20, 30], current: Some(0), active: true, change_on_leave: false,
                current_reads: 0, failure: Failure::None, events: Vec::new() }
        }
        fn index(&self, state: *mut c_void) -> usize {
            // Every non-null fixture handle points into this live byte array.
            unsafe { state.cast::<u8>().offset_from(self.states.as_ptr()) as usize }
        }
        fn state(&mut self, index: usize) -> *mut c_void { unsafe { self.states.as_mut_ptr().add(index).cast() } }
    }

    unsafe extern "C" fn current(ctx: *mut c_void) -> *mut c_void {
        let native = unsafe { &mut *ctx.cast::<Native>() };
        native.current_reads += 1;
        native.current.map_or(std::ptr::null_mut(), |index| native.state(index))
    }
    unsafe extern "C" fn state_id(ctx: *mut c_void, state: *mut c_void) -> i32 {
        let native = unsafe { &*ctx.cast::<Native>() };
        native.ids[native.index(state)]
    }
    unsafe extern "C" fn find_state(ctx: *mut c_void, id: i32) -> *mut c_void {
        let native = unsafe { &mut *ctx.cast::<Native>() };
        let index = native.ids.iter().position(|&state_id| state_id == id);
        index.map_or(std::ptr::null_mut(), |index| native.state(index))
    }
    unsafe extern "C" fn warn_missing(ctx: *mut c_void, id: i32) -> bool {
        let native = unsafe { &mut *ctx.cast::<Native>() };
        native.events.push(Event::Warning(id));
        native.failure != Failure::Warning
    }
    unsafe extern "C" fn leave(ctx: *mut c_void, state: *mut c_void, next: i32) -> bool {
        let native = unsafe { &mut *ctx.cast::<Native>() };
        native.events.push(Event::Leave(native.index(state), next, native.current));
        if native.change_on_leave {
            native.current = Some(2);
            native.ids[1] = 200;
            native.ids[2] = -7;
        }
        native.failure != Failure::Leave
    }
    unsafe extern "C" fn enter(ctx: *mut c_void, state: *mut c_void, previous: i32) -> bool {
        let native = unsafe { &mut *ctx.cast::<Native>() };
        native.events.push(Event::Enter(native.index(state), previous, native.current));
        native.failure != Failure::Enter
    }
    unsafe extern "C" fn set_current(ctx: *mut c_void, state: *mut c_void) {
        let native = unsafe { &mut *ctx.cast::<Native>() };
        let index = native.index(state);
        native.events.push(Event::Current(index));
        native.current = Some(index);
    }
    unsafe extern "C" fn active(ctx: *mut c_void) -> bool { unsafe { (*ctx.cast::<Native>()).active } }
    unsafe extern "C" fn update(ctx: *mut c_void, state: *mut c_void, time: f32) -> bool {
        let native = unsafe { &mut *ctx.cast::<Native>() };
        native.events.push(Event::Update(native.index(state), time.to_bits()));
        native.failure != Failure::Update
    }
    fn transition_host(native: &mut Native) -> HplAIStateTransitionHost {
        HplAIStateTransitionHost { ctx: (native as *mut Native).cast(), current, state_id, find_state,
            warn_missing, leave, enter, set_current }
    }
    fn update_host(native: &mut Native) -> HplAIStateUpdateHost {
        HplAIStateUpdateHost { ctx: (native as *mut Native).cast(), active, current, update }
    }

    #[test]
    fn transition_ffi_keeps_target_identity_and_rereads_current_after_leave() {
        let mut native = Native::new();
        native.change_on_leave = true;
        let host = transition_host(&mut native);
        unsafe { hpl_ai_state_change(20, &host); }
        assert_eq!(native.events, [Event::Leave(0, 20, Some(0)), Event::Enter(1, -7, Some(2)), Event::Current(1)]);
        assert_eq!(native.current, Some(1));
    }

    #[test]
    fn null_lookup_warns_and_returns_even_if_the_native_warning_fails() {
        let mut native = Native::new();
        native.failure = Failure::Warning;
        let host = transition_host(&mut native);
        unsafe { hpl_ai_state_change(i32::MIN, &host); }
        assert_eq!(native.events, [Event::Warning(i32::MIN)]);
        assert_eq!(native.current, Some(0));
    }

    #[test]
    fn native_leave_and_enter_failure_stop_before_assignment_without_rollback() {
        for failure in [Failure::Leave, Failure::Enter] {
            let mut native = Native::new();
            native.failure = failure;
            native.change_on_leave = true;
            let host = transition_host(&mut native);
            unsafe { hpl_ai_state_change(20, &host); }
            let mut expected = vec![Event::Leave(0, 20, Some(0))];
            if failure == Failure::Enter { expected.push(Event::Enter(1, -7, Some(2))); }
            assert_eq!(native.events, expected);
            assert_eq!(native.current, Some(2));
        }
    }

    #[test]
    fn inactive_and_null_current_dispatch_never_call_native_update() {
        let mut native = Native::new();
        native.active = false;
        let host = update_host(&mut native);
        unsafe { hpl_ai_state_update(1.0, &host); }
        assert_eq!(native.current_reads, 0);
        native.active = true;
        native.current = None;
        unsafe { hpl_ai_state_update(1.0, &host); }
        assert_eq!(native.current_reads, 1);
        assert!(native.events.is_empty());
    }

    #[test]
    fn dispatch_ffi_forwards_handle_and_float_bits_once_even_on_failure() {
        for bits in [0x80000000, 1, 0x3eaaaaab, 0x7f800000, 0xff800000, 0x7fc12345, 0xffc54321, 0x7f800001] {
            let mut native = Native::new();
            native.current = Some(2);
            native.failure = Failure::Update;
            let host = update_host(&mut native);
            unsafe { hpl_ai_state_update(f32::from_bits(bits), &host); }
            assert_eq!(native.events, [Event::Update(2, bits)]);
            assert_eq!(native.current, Some(2));
        }
    }
}

#[cfg(test)]
mod schedule_tests {
    use super::*;

    #[derive(Clone, Copy)]
    enum Action {
        None,
        Change { count: f32, step: f32, success: bool },
        Nested { time: f32, fail_inner: bool },
    }
    #[derive(Debug, PartialEq)]
    enum Event {
        Store(u32, u32),
        Tick { step: u32, count: u32, depth: u32 },
        NestedReturn(u32),
    }
    struct Native {
        count: f32, step: f32, depth: u32, calls: usize, guarded: bool,
        action: Action, events: Vec<Event>,
    }
    impl Native {
        fn new(count: f32, step: f32) -> Self {
            Self { count, step, depth: 0, calls: 0, guarded: false, action: Action::None, events: Vec::new() }
        }
        fn ticks(&self) -> Vec<(u32, u32, u32)> {
            self.events.iter().filter_map(|event| match event {
                Event::Tick { step, count, depth } => Some((*step, *count, *depth)), _ => None,
            }).collect()
        }
    }
    fn host(ctx: *mut c_void) -> HplAIStateScheduleHost {
        unsafe extern "C" fn time_count(ctx: *mut c_void) -> f32 { unsafe { (*ctx.cast::<Native>()).count } }
        unsafe extern "C" fn set_time_count(ctx: *mut c_void, count: f32) {
            let native = unsafe { &mut *ctx.cast::<Native>() };
            native.count = count;
            native.events.push(Event::Store(count.to_bits(), native.depth));
        }
        unsafe extern "C" fn update_step(ctx: *mut c_void) -> f32 { unsafe { (*ctx.cast::<Native>()).step } }
        unsafe extern "C" fn on_update(ctx: *mut c_void, step: f32) -> bool {
            let action = {
                let native = unsafe { &mut *ctx.cast::<Native>() };
                native.calls += 1;
                // Bound a broken fixture without panicking across extern "C".
                if native.calls > 64 { native.guarded = true; return false; }
                native.events.push(Event::Tick { step: step.to_bits(), count: native.count.to_bits(), depth: native.depth });
                std::mem::replace(&mut native.action, Action::None)
            };
            match action {
                Action::None => true,
                Action::Change { count, step, success } => {
                    let native = unsafe { &mut *ctx.cast::<Native>() };
                    native.count = count; native.step = step;
                    success
                }
                Action::Nested { time, fail_inner } => {
                    {
                        let native = unsafe { &mut *ctx.cast::<Native>() };
                        native.depth += 1;
                        if fail_inner { native.action = Action::Change { count: 0.875, step: 0.25, success: false }; }
                    }
                    // No Rust reference to Native survives this nested FFI call.
                    // Returning true afterward models a native callback catching
                    // the inner invocation's saved exception and continuing.
                    unsafe { hpl_ai_state_schedule(time, &host(ctx)); }
                    {
                        let native = unsafe { &mut *ctx.cast::<Native>() };
                        native.depth -= 1;
                        native.events.push(Event::NestedReturn(native.count.to_bits()));
                    }
                    true
                }
            }
        }
        HplAIStateScheduleHost { ctx, time_count, set_time_count, update_step, on_update }
    }
    fn run(native: &mut Native, time: f32) {
        unsafe { hpl_ai_state_schedule(time, &host((native as *mut Native).cast())); }
        assert!(!native.guarded, "fixture exceeded its callback budget");
    }

    #[test]
    fn schedule_ffi_publishes_accumulation_before_ticks_and_keeps_the_remainder() {
        let mut native = Native::new(0.125, 0.5);
        run(&mut native, 1.125);
        assert_eq!(native.events, [Event::Store(1.25f32.to_bits(), 0),
            Event::Tick { step: 0.5f32.to_bits(), count: 1.25f32.to_bits(), depth: 0 },
            Event::Store(0.75f32.to_bits(), 0),
            Event::Tick { step: 0.5f32.to_bits(), count: 0.75f32.to_bits(), depth: 0 },
            Event::Store(0.25f32.to_bits(), 0)]);
        run(&mut native, 0.25);
        assert_eq!(native.count.to_bits(), 0.0f32.to_bits());
        assert_eq!(native.calls, 3);
    }

    #[test]
    fn schedule_ffi_rereads_count_and_step_after_a_successful_callback() {
        let mut native = Native::new(0.0, 0.5);
        native.action = Action::Change { count: 0.875, step: 0.25, success: true };
        run(&mut native, 1.0);
        assert_eq!(native.ticks(), [(0.5f32.to_bits(), 1.0f32.to_bits(), 0),
            (0.25f32.to_bits(), 0.625f32.to_bits(), 0), (0.25f32.to_bits(), 0.375f32.to_bits(), 0)]);
        assert_eq!(native.count, 0.125);
    }

    #[test]
    fn schedule_ffi_failure_preserves_callback_edits_without_subtracting() {
        let mut native = Native::new(0.0, 0.5);
        native.action = Action::Change { count: 0.875, step: 0.25, success: false };
        run(&mut native, 1.0);
        assert_eq!(native.events, [Event::Store(1.0f32.to_bits(), 0),
            Event::Tick { step: 0.5f32.to_bits(), count: 1.0f32.to_bits(), depth: 0 }]);
        assert_eq!((native.count, native.step), (0.875, 0.25));
        run(&mut native, 0.0);
        assert_eq!((native.calls, native.count), (4, 0.125));
    }

    #[test]
    fn schedule_ffi_nested_update_shares_the_published_count() {
        let mut native = Native::new(0.0, 0.5);
        native.action = Action::Nested { time: 0.25, fail_inner: false };
        run(&mut native, 1.0);
        assert_eq!(native.ticks(), [(0.5f32.to_bits(), 1.0f32.to_bits(), 0),
            (0.5f32.to_bits(), 1.25f32.to_bits(), 1), (0.5f32.to_bits(), 0.75f32.to_bits(), 1)]);
        assert!(native.events.contains(&Event::NestedReturn(0.25f32.to_bits())));
        assert_eq!(native.count, -0.25);
    }

    #[test]
    fn schedule_ffi_caught_inner_failure_does_not_stop_outer_subtraction() {
        let mut native = Native::new(0.0, 0.5);
        native.action = Action::Nested { time: 0.25, fail_inner: true };
        run(&mut native, 1.0);
        assert_eq!(native.ticks(), [(0.5f32.to_bits(), 1.0f32.to_bits(), 0),
            (0.5f32.to_bits(), 1.25f32.to_bits(), 1), (0.25f32.to_bits(), 0.625f32.to_bits(), 0),
            (0.25f32.to_bits(), 0.375f32.to_bits(), 0)]);
        assert!(native.events.contains(&Event::NestedReturn(0.875f32.to_bits())));
        assert_eq!(native.count, 0.125);
    }

    #[test]
    fn schedule_ffi_forwards_step_bits_including_signed_zero_and_rescued_extremes() {
        for bits in [0, 0x80000000, 1, 0x80000001, 0x3eaaaaab, 0xbeaaaaab, 0x7f800000, 0xff800000] {
            let step = f32::from_bits(bits);
            let mut native = Native::new(step, step);
            native.action = Action::Change { count: 0.0, step: 1.0, success: true };
            run(&mut native, -0.0);
            assert_eq!(native.calls, 1);
            assert_eq!(native.ticks()[0].0, bits);
            assert_eq!(native.count, -1.0);
        }
    }

    #[test]
    fn schedule_ffi_ordered_comparison_and_infinite_subtraction_terminate() {
        let nan = f32::from_bits(0x7fc12345);
        for (count, step, elapsed) in [(nan, 0.25, 1.0), (0.5, nan, 0.0), (0.5, 0.25, nan)] {
            let mut native = Native::new(count, step);
            run(&mut native, elapsed);
            assert_eq!(native.calls, 0);
            assert_eq!(native.events.len(), 1);
        }
        for infinity in [f32::INFINITY, f32::NEG_INFINITY] {
            let mut native = Native::new(infinity, infinity);
            run(&mut native, 0.0);
            assert_eq!(native.calls, 1);
            assert_eq!(native.ticks()[0].0, infinity.to_bits());
            assert!(native.count.is_nan());
        }
    }

    #[test]
    fn schedule_ffi_preserves_negative_elapsed_and_f32_rounding_at_each_store() {
        let mut native = Native::new(0.125, 0.5);
        run(&mut native, -0.375);
        assert_eq!(native.count, -0.25);
        assert_eq!(native.calls, 0);
        run(&mut native, 0.75);
        assert_eq!((native.count, native.calls), (0.0, 1));

        let mut native = Native::new(16777216.0, f32::INFINITY);
        run(&mut native, 1.0); run(&mut native, 1.0);
        assert_eq!(native.count.to_bits(), 16777216.0f32.to_bits());
        assert_eq!(native.calls, 0);

        let mut native = Native::new(0.0, 0.1);
        run(&mut native, 0.3);
        assert_eq!(native.calls, 3);
        assert_eq!(native.count.to_bits(), 0x32800000);
    }
}
