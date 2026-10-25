//! FLOAT OPERATIONS

use crate::consts::{PI, PI_2, TWO_PI};
use crate::util::{f32_to_i32, f64_to_i32};
use crate::vector::{Vector2f, Vector2l, Vector3f, Vector3l};

/// `FastPositiveFloatToInt`: bias trick, rounds to nearest. Only valid for
/// non-negative values below 2^22.
#[inline]
pub fn fast_positive_float_to_int(val: f32) -> i32 {
    let bias: i32 = (23 + 127) << 23;
    let biased = (val + f32::from_bits(bias as u32)).to_bits() as i32;
    biased.wrapping_sub(bias)
}

/// `FastNegativeFloatToInt`
#[inline]
pub fn fast_negative_float_to_int(val: f32) -> i32 {
    let bias: i32 = ((23 + 127) << 23) + (1 << 22);
    let biased = (val + f32::from_bits(bias as u32)).to_bits() as i32;
    biased.wrapping_sub(bias)
}

/// `FastPosAndNegFloatToInt`
#[inline]
pub fn fast_pos_and_neg_float_to_int(val: f32) -> i32 {
    if val >= 0.0 { fast_positive_float_to_int(val) } else { fast_negative_float_to_int(val) }
}

/// `RoundToInt(float)`: `floor(x + 0.5)` computed in double precision.
#[inline]
pub fn round_to_int(val: f32) -> i32 {
    f64_to_i32((val as f64 + 0.5).floor())
}

/// `RoundToInt(cVector2f)`
#[inline]
pub fn round_to_int_vec2(v: Vector2f) -> Vector2l {
    Vector2l::new(round_to_int(v.x), round_to_int(v.y))
}

/// `RoundToInt(cVector3f)`
#[inline]
pub fn round_to_int_vec3(v: Vector3f) -> Vector3l {
    Vector3l::new(round_to_int(v.x), round_to_int(v.y), round_to_int(v.z))
}

/// `RoundFloatToDecimals`
pub fn round_float_to_decimals(mut val: f32, precision: i32) -> f32 {
    // Faithful: C++ `pow(10.0f, int)` promotes to the double `pow`.
    let pow = 10.0f64.powf(precision as f64) as f32;
    let inv_pow = 1.0 / pow;
    let add = 0.5 * inv_pow;
    val += add;
    let mut floor = (pow * val).floor();
    floor *= inv_pow;
    floor
}

/// `GetFraction`
#[inline]
pub fn get_fraction(val: f32) -> f32 {
    val - val.floor()
}

/// `Modulus`. Faithful: the result is always non-negative.
#[inline]
pub fn modulus(dividend: f32, divisor: f32) -> f32 {
    let num = (dividend / divisor).abs().floor();
    dividend.abs() - divisor.abs() * num
}

/// `ToRad`
#[inline]
pub fn to_rad(angle: f32) -> f32 {
    (angle / 360.0) * TWO_PI
}

/// `ToDeg`
#[inline]
pub fn to_deg(angle: f32) -> f32 {
    (angle / TWO_PI) * 360.0
}

/// `Log2ToInt`
pub fn log2_to_int(x: i32) -> i32 {
    match x {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        16 => 4,
        32 => 5,
        64 => 6,
        128 => 7,
        256 => 8,
        512 => 9,
        _ => f32_to_i32(((x as f32).ln() / 2.0f32.ln() + 0.5).floor()),
    }
}

/// `Pow2`: `1 << x`.
#[inline]
pub fn pow2(x: i32) -> i32 {
    1i32.wrapping_shl(x as u32)
}

/// `IsPow2`. Faithful: true for 0, false above 8192.
pub fn is_pow2(x: i32) -> bool {
    matches!(x, 0 | 1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 | 256 | 512 | 1024 | 2048 | 4096 | 8192)
}

/// `Wrap`: wraps `x` into `[min, max]`. Example: `wrap(-1, 0, 10) == 9`.
pub fn wrap(mut x: f32, mut min: f32, mut max: f32) -> f32 {
    if x >= min && x <= max {
        return x;
    }
    max -= min;
    let offset = min;
    min = 0.0;
    x -= offset;

    let num_of_max = (x / max).abs().floor();
    if x >= max {
        x -= num_of_max * max;
    } else if x < min {
        x += (num_of_max + 1.0) * max;
    }
    x + offset
}

/// `Clamp`
#[inline]
pub fn clamp(mut x: f32, min: f32, max: f32) -> f32 {
    if x > max {
        x = max;
    } else if x < min {
        x = min;
    }
    x
}

/// `Max(float, float)`. Faithful: returns `y` when either is NaN.
#[inline]
pub fn max_f(x: f32, y: f32) -> f32 {
    if x > y { x } else { y }
}

/// `Min(float, float)`. Faithful: returns `y` when either is NaN.
#[inline]
pub fn min_f(x: f32, y: f32) -> f32 {
    if x < y { x } else { y }
}

/// `Max(int, int)`
#[inline]
pub fn max_i(x: i32, y: i32) -> i32 {
    if x > y { x } else { y }
}

/// `Min(int, int)`
#[inline]
pub fn min_i(x: i32, y: i32) -> i32 {
    if x < y { x } else { y }
}

/// `Sign(float)`: -1, 0 or 1.
#[inline]
pub fn sign_f(x: f32) -> f32 {
    ((x > 0.0) as i32 - (x < 0.0) as i32) as f32
}

/// `Sign(int)`
#[inline]
pub fn sign_i(x: i32) -> i32 {
    (x > 0) as i32 - (x < 0) as i32
}

/// `GetAngleDistance`: shortest signed distance from `angle1` to `angle2`
/// on a circle of size `max_angle`.
pub fn get_angle_distance(angle1: f32, angle2: f32, max_angle: f32) -> f32 {
    let angle1 = wrap(angle1, 0.0, max_angle);
    let angle2 = wrap(angle2, 0.0, max_angle);

    if angle1 == angle2 {
        return 0.0;
    }
    let dist1 = angle2 - angle1;
    let mut dist2 = max_angle - dist1.abs();
    if dist1 > 0.0 {
        dist2 = -dist2;
    }
    if dist1.abs() < dist2.abs() { dist1 } else { dist2 }
}

/// `GetAngleDistanceRad`
#[inline]
pub fn get_angle_distance_rad(angle1: f32, angle2: f32) -> f32 {
    get_angle_distance(angle1, angle2, TWO_PI)
}

/// `GetAngleDistanceDeg`
#[inline]
pub fn get_angle_distance_deg(angle1: f32, angle2: f32) -> f32 {
    get_angle_distance(angle1, angle2, 360.0)
}

/// `TurnAngle`: step `angle` towards `final_angle` by `speed`.
pub fn turn_angle(mut angle: f32, final_angle: f32, speed: f32, max_angle: f32) -> f32 {
    if angle != final_angle {
        let angle_dist = final_angle - angle;
        if (final_angle > angle && angle_dist < max_angle) || (final_angle < angle && angle_dist < -max_angle) {
            angle += speed;
        } else {
            angle += -speed;
        }
    }
    // Faithful: `afSpeed * 1.5` is a double expression in C++.
    if (get_angle_distance(angle, final_angle, max_angle * 2.0).abs() as f64) <= (speed as f64 * 1.5) {
        angle = final_angle;
    }
    angle
}

/// `TurnAngleRad`
#[inline]
pub fn turn_angle_rad(angle: f32, final_angle: f32, speed: f32) -> f32 {
    turn_angle(angle, final_angle, speed, PI)
}

/// `TurnAngleDeg`
#[inline]
pub fn turn_angle_deg(angle: f32, final_angle: f32, speed: f32) -> f32 {
    turn_angle(angle, final_angle, speed, 180.0)
}

/// `Interpolate`: linear blend.
#[inline]
pub fn interpolate(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// `GetCorrectSignOfSpeed`
#[inline]
pub fn get_correct_sign_of_speed(current: f32, dest: f32, speed: f32) -> f32 {
    if dest < current && speed > 0.0 {
        return -speed;
    }
    if dest > current && speed < 0.0 {
        return -speed;
    }
    speed
}

/// `IncreaseTo`: move `x` towards `dest` by `add`, without overshooting.
#[inline]
pub fn increase_to(mut x: f32, add: f32, dest: f32) -> f32 {
    if x < dest {
        x += add;
        if x > dest {
            x = dest;
        }
    } else if x > dest {
        x -= add;
        if x < dest {
            x = dest;
        }
    }
    x
}

/// `SmoothCurve`: `x^2 * (3 - 2x)`.
#[inline]
pub fn smooth_curve(x: f32) -> f32 {
    x * x * (3.0 - x * 2.0)
}

/// `GetAngleFromPoints2D`: the angle a vector at `start` needs to look at `goal`.
pub fn get_angle_from_points_2d(start: Vector2f, goal: Vector2f) -> f32 {
    let mut dx = goal.x - start.x;
    let mut dy = goal.y - start.y;
    if dx == 0.0 {
        dx = 0.00001;
    }
    if dy == 0.0 {
        dy = 0.00001;
    }

    if dx >= 0.0 && dy < 0.0 {
        (dx / (-dy)).atan()
    } else if dx >= 0.0 && dy >= 0.0 {
        (dy / dx).atan() + PI_2
    } else if dx < 0.0 && dy >= 0.0 {
        ((-dx) / dy).atan() + PI
    } else if dx < 0.0 && dy < 0.0 {
        ((-dy) / (-dx)).atan() + PI_2 + PI
    } else {
        // Only reachable with NaN input; the C++ returns an uninitialised value.
        f32::NAN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_examples() {
        assert_eq!(wrap(-1.0, 0.0, 10.0), 9.0);
        assert_eq!(wrap(25.0, 0.0, 10.0), 5.0);
        assert_eq!(wrap(5.0, 0.0, 10.0), 5.0);
    }

    #[test]
    fn rounding() {
        assert_eq!(round_to_int(2.5), 3);
        assert_eq!(round_to_int(-2.5), -2);
        assert_eq!(round_float_to_decimals(1.23456, 2), 1.23);
        assert_eq!(fast_positive_float_to_int(12.4), 12);
        assert_eq!(fast_negative_float_to_int(-12.4), -12);
    }

    #[test]
    fn angles() {
        assert_eq!(get_angle_distance_deg(350.0, 10.0), 20.0);
        assert_eq!(get_angle_distance_deg(10.0, 350.0), -20.0);
        assert_eq!(log2_to_int(1024), 10);
        assert!(is_pow2(8192) && !is_pow2(16384));
    }
}
