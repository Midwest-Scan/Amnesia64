//! `cLogicTimer` and the OS-independent `VideoComp` predicate.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_system::{logic_timer::LogicTimer, platform};
use std::ffi::c_void;

/// Platform clock callback. C++ converts its native unsigned long to double.
pub type HplLogicTimerClock = Option<unsafe extern "C" fn(ctx: *mut c_void) -> f64>;

/// The engine clock stays in C++; Rust controls when it is read.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HplLogicTimerHost {
    pub ctx: *mut c_void,
    pub application_time: HplLogicTimerClock,
}

impl HplLogicTimerHost {
    unsafe fn time(self) -> f64 {
        // A missing host clock represents a stopped clock at zero.
        self.application_time.map_or(0.0, |clock| unsafe { clock(self.ctx) })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_init(rate: i32, host: HplLogicTimerHost, out: *mut HplLogicTimerState) {
    let timer = LogicTimer::new(rate, || unsafe { host.time() });
    unsafe { write_opt(out, timer) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_reset(state: *mut HplLogicTimerState, host: HplLogicTimerHost) {
    let mut timer: LogicTimer = unsafe { read(state) };
    timer.reset(|| unsafe { host.time() });
    unsafe { write_opt(state, timer) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_want_update(state: *mut HplLogicTimerState, host: HplLogicTimerHost) -> bool {
    let mut timer: LogicTimer = unsafe { read(state) };
    let result = timer.want_update(|| unsafe { host.time() });
    unsafe { write_opt(state, timer) };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_end_update_loop(state: *mut HplLogicTimerState, host: HplLogicTimerHost) {
    let mut timer: LogicTimer = unsafe { read(state) };
    timer.end_update_loop(|| unsafe { host.time() });
    unsafe { write_opt(state, timer) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_set_updates_per_sec(state: *mut HplLogicTimerState, rate: i32, host: HplLogicTimerHost) {
    let mut timer: LogicTimer = unsafe { read(state) };
    timer.set_updates_per_sec(rate, || unsafe { host.time() });
    unsafe { write_opt(state, timer) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_set_max_updates(state: *mut HplLogicTimerState, max: i32) {
    let mut timer: LogicTimer = unsafe { read(state) };
    timer.set_max_updates(max);
    unsafe { write_opt(state, timer) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_get_updates_per_sec(state: *const HplLogicTimerState) -> i32 {
    let timer: LogicTimer = unsafe { read(state) };
    timer.updates_per_sec()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_get_step_size(state: *const HplLogicTimerState) -> f32 {
    let timer: LogicTimer = unsafe { read(state) };
    timer.step_size()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_logic_timer_update(state: *mut HplLogicTimerState) {
    let mut timer: LogicTimer = unsafe { read(state) };
    timer.update();
    unsafe { write_opt(state, timer) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_platform_video_mode_less(display_a: i32, size_a: HplVector2l, display_b: i32, size_b: HplVector2l) -> bool {
    platform::video_mode_less(display_a, size_a.to_rs(), display_b, size_b.to_rs())
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn clock(ctx: *mut c_void) -> f64 {
        let reads = unsafe { &mut *ctx.cast::<u32>() };
        *reads += 1;
        *reads as f64
    }

    #[test]
    fn state_roundtrip_and_lazy_clock_through_ffi() {
        let mut reads = 0u32;
        let host = HplLogicTimerHost { ctx: (&mut reads as *mut u32).cast(), application_time: Some(clock) };
        let mut state = std::mem::MaybeUninit::<HplLogicTimerState>::uninit();
        unsafe { hpl_logic_timer_init(10, host, state.as_mut_ptr()) };
        let mut state = unsafe { state.assume_init() };
        assert_eq!(state.local_time, 1.0);
        unsafe {
            assert!(hpl_logic_timer_want_update(&mut state, host));
            assert!(!hpl_logic_timer_want_update(&mut state, host));
            assert_eq!(reads, 2);
            hpl_logic_timer_end_update_loop(&mut state, host);
        }
        assert_eq!(reads, 3);
        assert_eq!(state.local_time, 3.0);
        assert_eq!(state.update_count, 0);
    }

    #[test]
    fn initialization_accepts_null_output_and_absent_clock() {
        let host = HplLogicTimerHost { ctx: std::ptr::null_mut(), application_time: None };
        unsafe { hpl_logic_timer_init(60, host, std::ptr::null_mut()) };
        let mut state = HplLogicTimerState::from_rs(LogicTimer::new(60, || 123.0));
        unsafe { hpl_logic_timer_reset(&mut state, host) };
        assert_eq!(state.local_time, 0.0);
    }
}
