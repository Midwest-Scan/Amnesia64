//! Native AI state transitions and dispatch. Objects and vtables stay in C++.

#![allow(clippy::missing_safety_doc)]

use hpl_ai::state_machine::{self, TransitionHost, UpdateHost};
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

/// Dispatches the selected state once into its native fixed-step update loop.
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
