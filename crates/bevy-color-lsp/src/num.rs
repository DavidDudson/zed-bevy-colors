//! Narrow, auditable numeric conversions.
//!
//! Cast-related clippy lints are allowed only within this module, with
//! localized justifications. Call sites use the named helpers.
//!
//! Target assumption: `usize::BITS >= 32`. The LSP binary ships for
//! 64-bit targets only; asserted at compile time below.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]

const _: () = assert!(usize::BITS >= 32, "32-bit targets not supported");

/// Widen a `u32` to `usize`. Lossless on all supported targets.
#[inline]
#[must_use]
pub fn u32_to_usize(x: u32) -> usize {
    x as usize
}

/// Narrow a `usize` to `u32`, saturating at `u32::MAX`.
#[inline]
#[must_use]
pub fn usize_to_u32_sat(x: usize) -> u32 {
    if x > u32::MAX as usize {
        u32::MAX
    } else {
        x as u32
    }
}

/// Convert a normalized `f32` channel value to a `u8`, rounding to
/// nearest and clamping to `[0, 255]`. Does not panic.
///
/// `NaN` inputs yield `0` (Rust's `f32::clamp` is NaN-passthrough; the
/// subsequent `as u8` cast saturates NaN to 0 on all supported
/// platforms).
#[inline]
#[must_use]
pub fn f32_to_u8_clamped(x: f32) -> u8 {
    x.round().clamp(0.0, 255.0) as u8
}

/// Convert a non-negative `f32` to `u32`, flooring and clamping to
/// `max`.
#[inline]
#[must_use]
pub fn f32_to_u32_floor_clamped(x: f32, max: u32) -> u32 {
    x.floor().clamp(0.0, max as f32) as u32
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn u32_to_usize_roundtrip() {
        assert_eq!(u32_to_usize(0), 0);
        assert_eq!(u32_to_usize(u32::MAX), u32::MAX as usize);
    }

    #[test]
    fn usize_to_u32_saturates() {
        assert_eq!(usize_to_u32_sat(0), 0);
        assert_eq!(usize_to_u32_sat(12345), 12345);
        // On 64-bit usize > u32::MAX saturates.
        assert_eq!(usize_to_u32_sat(u32::MAX as usize + 1), u32::MAX);
        assert_eq!(usize_to_u32_sat(usize::MAX), u32::MAX);
    }

    #[test]
    fn f32_to_u8_clamps_low_high() {
        assert_eq!(f32_to_u8_clamped(-1.0), 0);
        assert_eq!(f32_to_u8_clamped(0.0), 0);
        assert_eq!(f32_to_u8_clamped(127.5), 128);
        assert_eq!(f32_to_u8_clamped(255.0), 255);
        assert_eq!(f32_to_u8_clamped(1_000_000.0), 255);
    }

    #[test]
    fn f32_to_u8_handles_nan_and_inf() {
        // clamp(NaN) is implementation-defined in IEEE-754 but f32::clamp
        // picks the NaN path = passthrough. round() of NaN is NaN, and
        // NaN as u8 is 0 — we rely on this being saturating-to-0.
        // Goal: never panic.
        let _ = f32_to_u8_clamped(f32::NAN);
        assert_eq!(f32_to_u8_clamped(f32::INFINITY), 255);
        assert_eq!(f32_to_u8_clamped(f32::NEG_INFINITY), 0);
    }

    #[test]
    fn f32_to_u32_floor_clamped_bounds() {
        assert_eq!(f32_to_u32_floor_clamped(-1.0, 6), 0);
        assert_eq!(f32_to_u32_floor_clamped(0.0, 6), 0);
        assert_eq!(f32_to_u32_floor_clamped(5.9, 6), 5);
        assert_eq!(f32_to_u32_floor_clamped(6.0, 6), 6);
        assert_eq!(f32_to_u32_floor_clamped(100.0, 6), 6);
    }
}
