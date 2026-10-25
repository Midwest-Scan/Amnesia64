//! `cPidController<T>` (`math/PidController.h`).

use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Debug, Default)]
pub struct PidController<T> {
    pub p: f32,
    pub i: f32,
    pub d: f32,
    errors: Vec<T>,
    time_steps: Vec<f32>,
    integral: T,
    derivative: T,
    error_num: usize,
    last_num: Option<usize>,
}

impl<T> PidController<T>
where
    T: Copy + Default + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T> + Div<f32, Output = T>,
{
    pub fn new(p: f32, i: f32, d: f32, error_num: usize) -> Self {
        let mut c = Self { p, i, d, ..Self::default() };
        c.set_error_num(error_num);
        c.reset();
        c
    }

    /// `Output`
    pub fn output(&mut self, error: T, time_step: f32) -> T {
        self.errors[self.error_num] = error;
        self.time_steps[self.error_num] = time_step;

        self.integral = T::default();
        for (e, t) in self.errors.iter().zip(&self.time_steps) {
            self.integral = self.integral + *e * *t;
        }

        self.derivative = T::default();
        if let Some(last) = self.last_num {
            self.derivative = (self.errors[self.error_num] - self.errors[last]) / time_step;
        }

        let last = self.error_num;
        self.last_num = Some(last);
        self.error_num += 1;
        if self.error_num >= self.errors.len() {
            self.error_num = 0;
        }

        self.errors[last] * self.p + self.integral * self.i + self.derivative * self.d
    }

    /// `SetErrorNum`: grows or shrinks the history. New slots are zero.
    pub fn set_error_num(&mut self, n: usize) {
        self.errors.resize(n, T::default());
        self.time_steps.resize(n, 0.0);
    }

    /// `Reset`. Faithful: the error history is kept, only time steps are cleared.
    pub fn reset(&mut self) {
        self.error_num = 0;
        self.last_num = None;
        self.time_steps.iter_mut().for_each(|t| *t = 0.0);
        self.integral = T::default();
        self.derivative = T::default();
    }

    /// `GetLastError`
    pub fn last_error(&self) -> T {
        self.last_num.map(|i| self.errors[i]).unwrap_or_default()
    }

    /// `GetLastDerivative`
    pub fn last_derivative(&self) -> T {
        self.derivative
    }

    /// `GetLastIntegral`
    pub fn last_integral(&self) -> T {
        self.integral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proportional_only() {
        let mut pid = PidController::<f32>::new(2.0, 0.0, 0.0, 4);
        assert_eq!(pid.output(3.0, 0.1), 6.0);
        assert_eq!(pid.last_error(), 3.0);
    }

    #[test]
    fn derivative_uses_previous_error() {
        let mut pid = PidController::<f32>::new(0.0, 0.0, 1.0, 4);
        pid.output(1.0, 0.5);
        assert_eq!(pid.output(2.0, 0.5), 2.0);
    }
}
