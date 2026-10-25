//! Allocation-free helpers for typed and generated encoders.
use crate::{
    codec::{fixed_size, validated_fixed_sequence_bytes, Ssz},
    error::CodecError,
};

const ENVELOPE: u128 = 4_294_967_296;

pub fn check_length(output: &[u8], expected: usize) -> Result<(), CodecError> {
    if output.len() != expected {
        return Err(CodecError::InvalidLength);
    }
    Ok(())
}

pub fn copy(output: &mut [u8], bytes: &[u8]) -> Result<(), CodecError> {
    check_length(output, bytes.len())?;
    output.copy_from_slice(bytes);
    Ok(())
}

fn fixed<T: Ssz>() -> Result<bool, CodecError> {
    match fixed_size::<T>() {
        Ok(None) => Ok(false),
        Ok(Some(_)) | Err(CodecError::SizeOverflow) => Ok(true),
        Err(error) => Err(error),
    }
}

/// Defer parent budget refusal until every child's protocol validation succeeds.
#[derive(Default)]
pub struct ContainerSize {
    leading: u128,
    body: u128,
    oversized: bool,
}

impl ContainerSize {
    pub fn push<T: Ssz>(&mut self, value: &T) -> Result<Option<usize>, CodecError> {
        let length = match value.encoded_len() {
            Ok(length) => length,
            Err(CodecError::NativeOutputOverflow) => {
                self.oversized = true;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if fixed::<T>()? {
            self.leading = (self.leading + length as u128).min(ENVELOPE);
        } else {
            self.leading = (self.leading + 4).min(ENVELOPE);
            self.body = (self.body + length as u128).min(ENVELOPE);
        }
        Ok(Some(length))
    }

    pub fn finish(self) -> Result<(usize, usize), CodecError> {
        let total = self.leading + self.body;
        if self.oversized || total >= ENVELOPE {
            return Err(CodecError::SizeOverflow);
        }
        Ok((self.leading as usize, total as usize))
    }
}

/// Every child receives only its own body slice, not the parent's full buffer.
pub struct ContainerWriter<'a> {
    output: &'a mut [u8],
    header: usize,
    leading: usize,
    body: usize,
}

impl<'a> ContainerWriter<'a> {
    pub fn new(output: &'a mut [u8], leading: usize) -> Result<Self, CodecError> {
        if leading > output.len() {
            return Err(CodecError::InvalidLength);
        }
        Ok(Self {
            output,
            header: 0,
            leading,
            body: leading,
        })
    }

    pub fn push<T: Ssz>(&mut self, value: &T, length: usize) -> Result<(), CodecError> {
        self.push_layout(value, length, fixed::<T>()?)
    }

    #[inline]
    fn push_layout<T: Ssz>(
        &mut self,
        value: &T,
        length: usize,
        fixed: bool,
    ) -> Result<(), CodecError> {
        let range = self.range(fixed, length)?;
        value.encode_into(&mut self.output[range])
    }

    #[inline]
    fn range(&mut self, fixed: bool, length: usize) -> Result<std::ops::Range<usize>, CodecError> {
        let header_end = self
            .header
            .checked_add(if fixed { length } else { 4 })
            .ok_or(CodecError::InvalidLength)?;
        if header_end > self.leading {
            return Err(CodecError::InvalidLength);
        }
        let range = if fixed {
            self.header..header_end
        } else {
            let end = self
                .body
                .checked_add(length)
                .ok_or(CodecError::InvalidLength)?;
            if end > self.output.len() {
                return Err(CodecError::InvalidLength);
            }
            let offset = u32::try_from(self.body).map_err(|_| CodecError::SizeOverflow)?;
            self.output[self.header..header_end].copy_from_slice(&offset.to_le_bytes());
            let start = self.body;
            self.body = end;
            start..end
        };
        self.header = header_end;
        Ok(range)
    }

    pub fn finish(self) -> Result<(), CodecError> {
        if self.header != self.leading || self.body != self.output.len() {
            return Err(CodecError::InvalidLength);
        }
        Ok(())
    }
}

pub(crate) fn sequence_len<T: Ssz>(values: &[T]) -> Result<usize, CodecError> {
    if let Some(bytes) = validated_fixed_sequence_bytes(values)? {
        if bytes.len() as u128 >= ENVELOPE {
            return Err(CodecError::SizeOverflow);
        }
        return Ok(bytes.len());
    }
    let mut body = 0_u128;
    let mut oversized = false;
    for value in values {
        match value.encoded_len() {
            // At most usize::MAX children, each at most usize::MAX bytes:
            // their sum fits u128 on supported 32/64-bit targets. Keeping the
            // exact sum lets constant-width scalar loops fold into arithmetic.
            Ok(length) => body += length as u128,
            Err(CodecError::NativeOutputOverflow) => oversized = true,
            Err(error) => return Err(error),
        }
    }
    let leading = if fixed::<T>()? {
        0
    } else {
        values.len() as u128 * 4
    };
    // Check each operand first: their sum need only be formed below 2^32.
    // All children were still validated before any parent budget refusal.
    if oversized || body >= ENVELOPE || leading >= ENVELOPE || leading + body >= ENVELOPE {
        return Err(CodecError::SizeOverflow);
    }
    Ok((leading + body) as usize)
}

pub(crate) fn write_sequence<T: Ssz>(values: &[T], output: &mut [u8]) -> Result<(), CodecError> {
    if let Some(bytes) = validated_fixed_sequence_bytes(values)? {
        return copy(output, bytes);
    }
    let fixed = fixed::<T>()?;
    let leading = if fixed {
        output.len()
    } else {
        values
            .len()
            .checked_mul(4)
            .ok_or(CodecError::SizeOverflow)?
    };
    let mut writer = ContainerWriter::new(output, leading)?;
    for value in values {
        writer.push_layout(value, value.encoded_len()?, fixed)?;
    }
    writer.finish()
}

pub fn union_len<T: Ssz>(value: &T) -> Result<usize, CodecError> {
    value
        .encoded_len()?
        .checked_add(1)
        .ok_or(CodecError::NativeOutputOverflow)
}
