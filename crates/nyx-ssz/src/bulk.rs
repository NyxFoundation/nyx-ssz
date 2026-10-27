//! Audited native bulk copies for primitive integer sequences.
#![allow(unsafe_code)]

use crate::error::CodecError;

/// Primitive integers whose every bit pattern is valid and whose byte width is exact.
///
/// # Safety
///
/// Implementors must have no padding or invalid bit patterns. This private boundary is
/// intentionally limited to Rust's unsigned integer primitives.
pub(crate) unsafe trait NativeInteger: Copy {}

unsafe impl NativeInteger for u8 {}
unsafe impl NativeInteger for u16 {}
unsafe impl NativeInteger for u32 {}
unsafe impl NativeInteger for u64 {}
unsafe impl NativeInteger for u128 {}

/// Copy little-endian bytes into a correctly aligned integer allocation.
///
/// The caller validates byte divisibility and invokes this only on little-endian targets.
pub(crate) fn copy_integer_sequence<T: NativeInteger>(input: &[u8]) -> Result<Vec<T>, CodecError> {
    let width = core::mem::size_of::<T>();
    if width == 0 || !input.len().is_multiple_of(width) {
        return Err(CodecError::InvalidLength);
    }
    let count = input.len() / width;
    let mut output = Vec::<T>::with_capacity(count);
    // SAFETY: `output` owns capacity for exactly `input.len()` bytes, the fresh
    // allocation cannot overlap `input`, and every bit pattern is valid for `T`.
    // The copy initializes every element before the vector length becomes visible.
    unsafe {
        core::ptr::copy_nonoverlapping(
            input.as_ptr(),
            output.as_mut_ptr().cast::<u8>(),
            input.len(),
        );
        output.set_len(count);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_reject_input_that_cannot_fill_complete_integers() {
        assert_eq!(
            copy_integer_sequence::<u16>(&[0]),
            Err(CodecError::InvalidLength)
        );
    }
}
