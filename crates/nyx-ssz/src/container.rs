//! Shared mixed fixed/variable layout, not a Container value codec.

use crate::{
    error::CodecError,
    sequence::{offset_spans, BodyRange, SequenceView},
};

/// Compile-time field positions for generated container decoders.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct DerivedContainerLayout<const N: usize> {
    header: usize,
    positions: [usize; N],
    widths: [Option<usize>; N],
    variable: bool,
}

/// One validated generated-container field range.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct DerivedFieldRange {
    pub start: usize,
    pub end: usize,
}

/// Build generated-container layout metadata during constant evaluation.
#[doc(hidden)]
pub const fn derived_container_layout<const N: usize>(
    widths: [Result<Option<usize>, CodecError>; N],
) -> Result<DerivedContainerLayout<N>, CodecError> {
    let mut variable = false;
    let mut index = 0;
    while index < N {
        if matches!(widths[index], Ok(None)) {
            variable = true;
        }
        index += 1;
    }

    let fault = if variable {
        CodecError::ScopeTooSmall
    } else {
        CodecError::InvalidLength
    };
    let mut header = 0usize;
    let mut positions = [0usize; N];
    let mut exact = [None; N];
    index = 0;
    while index < N {
        positions[index] = header;
        let width = match widths[index] {
            Ok(Some(width)) => {
                exact[index] = Some(width);
                width
            }
            Ok(None) => 4,
            Err(CodecError::SizeOverflow) => return Err(fault),
            Err(error) => return Err(error),
        };
        header = match header.checked_add(width) {
            Some(next) => next,
            None => return Err(fault),
        };
        index += 1;
    }
    Ok(DerivedContainerLayout {
        header,
        positions,
        widths: exact,
        variable,
    })
}

/// Validate generated-container offsets and return stack-allocated field ranges.
#[doc(hidden)]
#[inline]
pub fn derived_container_ranges<const N: usize>(
    input: &[u8],
    layout: Result<DerivedContainerLayout<N>, CodecError>,
) -> Result<[DerivedFieldRange; N], CodecError> {
    if input.len() as u128 >= 4_294_967_296 {
        return Err(CodecError::SizeOverflow);
    }
    let layout = layout?;
    validate_derived_scope(input.len(), &layout)?;
    let (mut ranges, first) = read_derived_slots(input, &layout);
    validate_derived_offsets(input.len(), &layout, &ranges, first)?;
    finish_derived_ranges(input.len(), &layout, &mut ranges);
    Ok(ranges)
}

#[inline]
fn validate_derived_scope<const N: usize>(
    input_len: usize,
    layout: &DerivedContainerLayout<N>,
) -> Result<(), CodecError> {
    if input_len < layout.header {
        return Err(if layout.variable {
            CodecError::ScopeTooSmall
        } else {
            CodecError::InvalidLength
        });
    }
    if !layout.variable && input_len != layout.header {
        return Err(CodecError::InvalidLength);
    }
    Ok(())
}

#[inline]
fn read_derived_slots<const N: usize>(
    input: &[u8],
    layout: &DerivedContainerLayout<N>,
) -> ([DerivedFieldRange; N], Option<usize>) {
    let mut ranges = [DerivedFieldRange { start: 0, end: 0 }; N];
    let mut first = None;
    let mut index = 0;
    while index < N {
        let position = layout.positions[index];
        if let Some(width) = layout.widths[index] {
            ranges[index] = DerivedFieldRange {
                start: position,
                end: position + width,
            };
        } else {
            let bytes = [
                input[position],
                input[position + 1],
                input[position + 2],
                input[position + 3],
            ];
            let offset = u32::from_le_bytes(bytes) as usize;
            first.get_or_insert(offset);
            ranges[index].start = offset;
        }
        index += 1;
    }
    (ranges, first)
}

#[inline]
fn validate_derived_offsets<const N: usize>(
    input_len: usize,
    layout: &DerivedContainerLayout<N>,
    ranges: &[DerivedFieldRange; N],
    first: Option<usize>,
) -> Result<(), CodecError> {
    if first.is_some_and(|offset| offset != layout.header) {
        return Err(CodecError::FirstOffset);
    }
    let mut previous = None;
    let mut index = 0;
    while index < N {
        if layout.widths[index].is_none() {
            let offset = ranges[index].start;
            if previous.is_some_and(|value| offset < value) {
                return Err(CodecError::OffsetUnordered);
            }
            previous = Some(offset);
        }
        index += 1;
    }
    if previous.is_some_and(|last| last > input_len) {
        return Err(CodecError::OffsetPastScope);
    }
    Ok(())
}

#[inline]
fn finish_derived_ranges<const N: usize>(
    input_len: usize,
    layout: &DerivedContainerLayout<N>,
    ranges: &mut [DerivedFieldRange; N],
) {
    let mut next = input_len;
    let mut index = N;
    while index > 0 {
        index -= 1;
        if layout.widths[index].is_none() {
            ranges[index].end = next;
            next = ranges[index].start;
        }
    }
}

/// Borrow field-order byte ranges using truthful child fixed-size metadata.
///
/// `Ok(None)` denotes a variable field. `Err(SizeOverflow)` denotes a fixed
/// natural width exceeding `usize`, as returned by `codec::fixed_size`.
/// This validates layout only, including internal zero-field/zero-width cases;
/// the parent codec must validate declarations and individual field values.
#[expect(clippy::question_mark, reason = "Explicit results support extraction")]
pub fn container_view<'a>(
    input: &'a [u8],
    widths: &[Result<Option<usize>, CodecError>],
) -> Result<SequenceView<'a>, CodecError> {
    if input.len() as u128 >= 4_294_967_296 {
        return Err(CodecError::SizeOverflow);
    }
    let header = match check_header(widths, input.len()) {
        Ok(header) => header,
        Err(error) => return Err(error),
    };
    let (ranges, offsets) = match read_slots(input, widths) {
        Ok(slots) => slots,
        Err(error) => return Err(error),
    };
    let offset_slice: &[usize] = &offsets;
    if offset_slice.is_empty() {
        return Ok(SequenceView { input, ranges });
    }
    if offsets[0] != header {
        return Err(CodecError::FirstOffset);
    }
    let spans = match offset_spans(&offsets, input.len()) {
        Ok(spans) => spans,
        Err(error) => return Err(error),
    };
    Ok(SequenceView {
        input,
        ranges: finish_ranges(widths, &ranges, &spans),
    })
}

#[expect(clippy::question_mark, reason = "Explicit results support extraction")]
fn check_header(
    widths: &[Result<Option<usize>, CodecError>],
    scope: usize,
) -> Result<usize, CodecError> {
    let variable = has_variable(widths);
    let header = match header_width(widths, scope, variable) {
        Ok(header) => header,
        Err(error) => return Err(error),
    };
    if !variable && header != scope {
        return Err(CodecError::InvalidLength);
    }
    Ok(header)
}

fn has_variable(widths: &[Result<Option<usize>, CodecError>]) -> bool {
    let mut index = 0;
    while index < widths.len() {
        if let Ok(None) = &widths[index] {
            return true;
        }
        index += 1;
    }
    false
}

fn header_width(
    widths: &[Result<Option<usize>, CodecError>],
    scope: usize,
    variable: bool,
) -> Result<usize, CodecError> {
    let fault = if variable {
        CodecError::ScopeTooSmall
    } else {
        CodecError::InvalidLength
    };
    let mut position = 0;
    let mut index = 0;
    while index < widths.len() {
        let width = match &widths[index] {
            Ok(Some(width)) => *width,
            Ok(None) => 4,
            Err(CodecError::SizeOverflow) => return Err(fault),
            Err(error) => return Err(*error),
        };
        if width > scope - position {
            return Err(fault);
        }
        position += width;
        index += 1;
    }
    Ok(position)
}

fn read_slots(
    input: &[u8],
    widths: &[Result<Option<usize>, CodecError>],
) -> Result<(Vec<BodyRange>, Vec<usize>), CodecError> {
    let mut ranges = Vec::new();
    let mut offsets = Vec::new();
    let mut position = 0;
    let mut index = 0;
    while index < widths.len() {
        let range = match &widths[index] {
            Ok(Some(width)) => {
                let end = position + *width;
                let range = BodyRange {
                    start: position,
                    end,
                };
                position = end;
                range
            }
            Ok(None) => {
                let end = position + 4;
                let start = match crate::scalar::decode_u32(&input[position..end]) {
                    Ok(offset) => offset as usize,
                    Err(_) => return Err(CodecError::InvalidLength),
                };
                offsets.push(start);
                position = end;
                BodyRange { start, end: start }
            }
            Err(error) => return Err(*error),
        };
        ranges.push(range);
        index += 1;
    }
    Ok((ranges, offsets))
}

fn finish_ranges(
    widths: &[Result<Option<usize>, CodecError>],
    slots: &[BodyRange],
    spans: &[usize],
) -> Vec<BodyRange> {
    let mut ranges = Vec::new();
    let mut index = 0;
    let mut body = 0;
    while index < widths.len() {
        let slot = &slots[index];
        let start = slot.start;
        let end = if let Ok(None) = &widths[index] {
            let end = start + spans[body];
            body += 1;
            end
        } else {
            slot.end
        };
        ranges.push(BodyRange { start, end });
        index += 1;
    }
    ranges
}

/// A validated all-fixed container body. Variable or erroneous widths are not cursors.
#[derive(Debug)]
pub struct FixedCursor<'a> {
    rest: &'a [u8],
}

impl<'a> FixedCursor<'a> {
    /// Borrow the next field. A width outside the validated body is `InvalidLength`.
    pub fn read(&mut self, width: usize) -> Result<&'a [u8], CodecError> {
        let Some((head, rest)) = self.rest.split_at_checked(width) else {
            return Err(CodecError::InvalidLength);
        };
        self.rest = rest;
        Ok(head)
    }
}

/// Return a no-allocation cursor when every width is fixed and sums to `input.len()`.
/// `Ok(None)` means at least one width is variable or a child metadata error.
pub fn fixed_container_cursor<'a>(
    input: &'a [u8],
    widths: &[Result<Option<usize>, CodecError>],
) -> Result<Option<FixedCursor<'a>>, CodecError> {
    if input.len() as u128 >= 4_294_967_296 {
        return Err(CodecError::SizeOverflow);
    }
    match fixed_total(widths) {
        FixedTotal::Variable => Ok(None),
        FixedTotal::Overflow => Err(CodecError::InvalidLength),
        FixedTotal::Exact(total) if total == input.len() => Ok(Some(FixedCursor { rest: input })),
        FixedTotal::Exact(_) => Err(CodecError::InvalidLength),
    }
}

enum FixedTotal {
    Variable,
    Overflow,
    Exact(usize),
}

fn fixed_total(widths: &[Result<Option<usize>, CodecError>]) -> FixedTotal {
    let mut total: usize = 0;
    for width in widths {
        let Ok(Some(width)) = width else {
            return FixedTotal::Variable;
        };
        match total.checked_add(*width) {
            Some(next) => total = next,
            None => return FixedTotal::Overflow,
        }
    }
    FixedTotal::Exact(total)
}
