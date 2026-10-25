//! `cQuaternion` methods and `cCRC`.

#![allow(clippy::missing_safety_doc)]

use crate::types::*;
use hpl_math::crc::{self, CrcTable};
use hpl_math::{Matrixf, Quaternion};

// ---------------------------------------------------------------------------
// cQuaternion
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_quaternion_normalize(q: *mut HplQuaternion) {
    let mut x: Quaternion = unsafe { read(q) };
    x.normalize();
    unsafe { write_opt(q, x) };
}

/// `ToRotationMatrix`: writes only the upper 3x3 of `dest`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_quaternion_to_rotation_matrix(q: HplQuaternion, dest: *mut HplMatrixf) {
    // `dest` may be an uninitialised cMatrixf, so it is never read.
    let mut m = Matrixf::IDENTITY;
    q.to_rs().to_rotation_matrix(&mut m);
    let dest = unsafe { std::ptr::addr_of_mut!((*dest).m) } as *mut [f32; 4];
    for r in 0..3 {
        for c in 0..3 {
            unsafe { (*dest.add(r))[c] = m.m[r][c] };
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_quaternion_from_rotation_matrix(rot: *const HplMatrixf) -> HplQuaternion {
    let m: Matrixf = unsafe { read(rot) };
    HplQuaternion::from_rs(Quaternion::from_rotation_matrix(&m))
}

/// `FromAngleAxis`. The axis must be unit length.
#[unsafe(no_mangle)]
pub extern "C" fn hpl_quaternion_from_angle_axis(angle: f32, axis: HplVector3f) -> HplQuaternion {
    HplQuaternion::from_rs(Quaternion::from_angle_axis(angle, axis.to_rs()))
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_quaternion_add(a: HplQuaternion, b: HplQuaternion) -> HplQuaternion {
    HplQuaternion::from_rs(a.to_rs() + b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_quaternion_sub(a: HplQuaternion, b: HplQuaternion) -> HplQuaternion {
    HplQuaternion::from_rs(a.to_rs() - b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_quaternion_mul(a: HplQuaternion, b: HplQuaternion) -> HplQuaternion {
    HplQuaternion::from_rs(a.to_rs() * b.to_rs())
}

/// Component-wise division (as the C++ `operator/`).
#[unsafe(no_mangle)]
pub extern "C" fn hpl_quaternion_div(a: HplQuaternion, b: HplQuaternion) -> HplQuaternion {
    HplQuaternion::from_rs(a.to_rs() / b.to_rs())
}

#[unsafe(no_mangle)]
pub extern "C" fn hpl_quaternion_mul_scalar(a: HplQuaternion, s: f32) -> HplQuaternion {
    HplQuaternion::from_rs(a.to_rs() * s)
}

// ---------------------------------------------------------------------------
// cCRC
// ---------------------------------------------------------------------------

/// `cCRCTable::Init`: fill the 256-entry `table` for polynomial `key`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_crc_init_table(table: *mut u32, key: u32) {
    let t = unsafe { &mut *(table as *mut [u32; 256]) };
    CrcTable::fill(t, key);
}

/// `cCRC::PutByte` on a raw register; returns the new register.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_crc_put_byte(table: *const u32, register: u32, byte: u32) -> u32 {
    let t = unsafe { &*(table as *const [u32; 256]) };
    crc::put_byte(t, register, byte)
}

/// `cCRC::PutData` on a raw register; returns the new register.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hpl_crc_put_data(table: *const u32, register: u32, data: *const u8, len: usize) -> u32 {
    let t = unsafe { &*(table as *const [u32; 256]) };
    crc::put_data(t, register, unsafe { raw_slice(data, len) })
}
