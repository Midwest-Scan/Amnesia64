//! `cLogicTimer` (`system/LogicTimer.cpp`). The clock is supplied by the
//! caller so the engine keeps its platform clock and its exact read order.

use hpl_math::util::f64_to_i32;

/// Value state of the timer, without the unused C++ low-level-system pointer.
/// C++ copies its members into this snapshot; the class itself is never cast.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LogicTimer {
    pub local_time: f64,
    pub local_time_add: f64,
    pub speed_mul: f64,
    pub max_updates: i32,
    pub update_count: i32,
}

impl LogicTimer {
    pub fn new(updates_per_sec: i32, clock: impl FnOnce() -> f64) -> Self {
        let mut timer = Self {
            local_time: 0.0,
            local_time_add: 0.0,
            speed_mul: 1.0,
            max_updates: updates_per_sec / 10,
            update_count: 0,
        };
        timer.set_updates_per_sec(updates_per_sec, clock);
        timer
    }

    pub fn reset(&mut self, clock: impl FnOnce() -> f64) {
        self.local_time = clock();
    }

    pub fn want_update(&mut self, clock: impl FnOnce() -> f64) -> bool {
        // Faithful: even a failed check consumes an update; exceeding the cap
        // skips the clock read altogether.
        // Deviation: signed counter overflow is C++ UB. Define it as wrapping
        // (the usual x86 result), including in Rust debug builds.
        self.update_count = self.update_count.wrapping_add(1);
        if self.update_count > self.max_updates { return false; }
        if self.local_time < clock() {
            self.update();
            return true;
        }
        false
    }

    pub fn end_update_loop(&mut self, clock: impl FnOnce() -> f64) {
        if self.update_count > self.max_updates {
            self.reset(clock);
        }
        self.update_count = 0;
    }

    pub fn set_updates_per_sec(&mut self, updates_per_sec: i32, clock: impl FnOnce() -> f64) {
        // Faithful: double division, including zero/negative rates. Changing
        // the rate resets time but does not change max_updates or update_count.
        self.local_time_add = 1000.0 / (updates_per_sec as f64);
        self.reset(clock);
    }

    pub fn set_max_updates(&mut self, max: i32) {
        self.max_updates = max;
    }

    pub fn updates_per_sec(&self) -> i32 {
        // Faithful: derive the rate from the rounded reciprocal, not the
        // original integer (e.g. 60 can round down to 59).
        // Deviation: a non-finite/out-of-range cast is C++ UB; use the existing
        // x86 conversion helper's INT_MIN result rather than Rust saturation.
        f64_to_i32(1000.0 / self.local_time_add)
    }

    pub fn step_size(&self) -> f32 {
        // Faithful: narrow before dividing, and ignore speed_mul.
        (self.local_time_add as f32) / 1000.0_f32
    }

    pub fn update(&mut self) {
        // Faithful: division is double precision even though SetSpeedMul
        // accepts a float. Do not clamp zero, negative, infinity or NaN.
        self.local_time += self.local_time_add / self.speed_mul;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_uses_truncated_cap_and_double_reciprocal() {
        for rate in [-101, -9, 0, 9, 60, 101] {
            let timer = LogicTimer::new(rate, || 123.0);
            assert_eq!(timer.local_time, 123.0);
            assert_eq!(timer.local_time_add.to_bits(), (1000.0 / rate as f64).to_bits());
            assert_eq!(timer.max_updates, rate / 10);
            assert_eq!(timer.update_count, 0);
            assert_eq!(timer.speed_mul, 1.0);
        }
    }

    #[test]
    fn failed_checks_consume_cap_and_skip_clock_after_cap() {
        let mut timer = LogicTimer::new(20, || 100.0);
        assert!(!timer.want_update(|| 100.0));
        assert!(!timer.want_update(|| 99.0));
        assert!(!timer.want_update(|| panic!("clock read after cap")));
        assert_eq!(timer.update_count, 3);
        timer.end_update_loop(|| 200.0);
        assert_eq!(timer.local_time, 200.0);
        assert_eq!(timer.update_count, 0);
    }

    #[test]
    fn rate_change_and_reset_preserve_cap_count_and_speed() {
        let mut timer = LogicTimer::new(100, || 0.0);
        timer.speed_mul = 0.5;
        assert!(timer.want_update(|| 1.0));
        assert_eq!(timer.local_time, 20.0);
        timer.set_updates_per_sec(25, || 9.0);
        assert_eq!(timer.max_updates, 10);
        assert_eq!(timer.update_count, 1);
        assert_eq!(timer.speed_mul, 0.5);
        assert_eq!(timer.step_size(), 0.04);
        timer.reset(|| 7.0);
        assert_eq!(timer.update_count, 1);
        timer.end_update_loop(|| panic!("clock read without exceeding cap"));
        assert_eq!(timer.local_time, 7.0);
    }

    #[test]
    fn zero_negative_and_nonfinite_speeds_are_not_clamped() {
        for speed in [0.0, -0.0, -2.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let mut timer = LogicTimer::new(100, || 0.0);
            timer.speed_mul = speed;
            assert!(timer.want_update(|| 1.0));
            let expected = 0.0 + 10.0 / speed;
            if expected.is_nan() { assert!(timer.local_time.is_nan()); }
            else { assert_eq!(timer.local_time.to_bits(), expected.to_bits()); }
            assert_eq!(timer.step_size(), 0.01);
        }
    }

    #[test]
    fn end_loop_uses_strict_cap_and_reset_does_not_clear_count() {
        let mut timer = LogicTimer::new(10, || 0.0);
        assert!(timer.want_update(|| 1.0));
        timer.end_update_loop(|| panic!("count equals cap"));
        assert_eq!(timer.local_time, 100.0);
        timer.set_max_updates(-1);
        timer.end_update_loop(|| 50.0);
        assert_eq!(timer.local_time, 50.0);
    }

    #[test]
    fn step_narrows_before_division_and_rate_truncates() {
        let timer = LogicTimer::new(60, || 0.0);
        assert_eq!(timer.updates_per_sec(), 59);
        assert_eq!(timer.step_size().to_bits(), 0x3c88_8888);
        let zero = LogicTimer::new(0, || 0.0);
        assert_eq!(zero.updates_per_sec(), 0);
        assert!(zero.step_size().is_infinite());
    }

    #[test]
    fn undefined_cpp_counter_and_cast_have_defined_results() {
        let mut timer = LogicTimer::new(10, || 0.0);
        timer.update_count = i32::MAX;
        assert!(timer.want_update(|| 1.0));
        assert_eq!(timer.update_count, i32::MIN);
        for increment in [0.0, -0.0, f64::NAN, 1.0e-30] {
            timer.local_time_add = increment;
            assert_eq!(timer.updates_per_sec(), i32::MIN);
        }
    }
}
