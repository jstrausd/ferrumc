//! Immutable raw original f32 values; no transcendental regeneration.
include!("sin_table.rs");
pub(super) fn sin(value: f32) -> f32 {
    f32::from_bits(ORIGINAL_SIN_BITS[((value * 10430.378_f32) as i32 & 65535) as usize])
}
pub(super) fn cos(value: f32) -> f32 {
    f32::from_bits(
        ORIGINAL_SIN_BITS[((value * 10430.378_f32 + 16384.0_f32) as i32 & 65535) as usize],
    )
}
