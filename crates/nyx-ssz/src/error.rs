//! Errors shared by the typed SSZ codec interface.

/// An SSZ value or byte string violates its declared representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecError {
    InvalidLength,
    InvalidBoolean,
    NoSelector,
    UnknownSelector,
    InvalidIntegerWidth,
    EmptyVector,
    SizeOverflow,
    /// Exact successful reference output exceeds `usize::MAX` bytes.
    NativeOutputOverflow,
    OffsetUnordered,
    OffsetPastScope,
    ScopeTooSmall,
    ScopeWidthless,
    ScopeUndivided,
    FirstOffset,
    OffsetBelowTable,
    OffsetUnaligned,
    OverLimit,
    PaddingBits,
    EmptyEncoding,
    NoDelimiter,
    TrailingZeros,
    /// Reference `containerEmpty` (CONTAINER_EMPTY).
    EmptyContainer,
    /// Reference `badDeclaration` (BAD_DECLARATION).
    BadDeclaration,
    /// Reference `layoutWidth` (LAYOUT_WIDTH).
    LayoutWidth,
    /// Reference `layoutTrailingGap` (LAYOUT_TRAILING_GAP).
    LayoutTrailingGap,
    /// Reference `layoutTooWide` (LAYOUT_TOO_WIDE).
    LayoutTooWide,
    /// Reference `layoutFieldCount` (LAYOUT_FIELD_COUNT).
    LayoutFieldCount,
    /// Reference `unionEmpty` (UNION_EMPTY).
    EmptyUnion,
    /// Reference `unionSelectorRepeated` (UNION_SELECTOR_REPEATED).
    UnionSelectorRepeated,
    /// Reference `unionSelectorRange` (UNION_SELECTOR_RANGE).
    UnionSelectorRange,
    /// Reference `unionIncompatible` (UNION_INCOMPATIBLE).
    UnionIncompatible,
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLength => "SSZ input has an invalid byte length",
            Self::InvalidBoolean => "SSZ boolean must be encoded as zero or one",
            Self::NoSelector => "SSZ union encoding must contain a selector",
            Self::UnknownSelector => "SSZ union selector is not declared",
            Self::InvalidIntegerWidth => {
                "SSZ unsigned integer width must be 1, 2, 4, 8, 16 or 32 bytes"
            }
            Self::EmptyVector => "SSZ vector declarations must contain at least one element",
            Self::SizeOverflow => "SSZ size exceeds its supported representation",
            Self::NativeOutputOverflow => "SSZ reference output exceeds native byte capacity",
            Self::OffsetUnordered => "SSZ offsets must be nondecreasing",
            Self::OffsetPastScope => "SSZ offset exceeds its input scope",
            Self::ScopeTooSmall => "SSZ input is smaller than its header",
            Self::ScopeWidthless => "SSZ nonempty input cannot use zero-width elements",
            Self::ScopeUndivided => "SSZ input does not divide into complete elements",
            Self::FirstOffset => "SSZ first offset must follow its header",
            Self::OffsetBelowTable => "SSZ first offset is below its table width",
            Self::OffsetUnaligned => "SSZ first offset is not aligned to its table width",
            Self::OverLimit => "SSZ element count exceeds its declared capacity",
            Self::PaddingBits => "SSZ bitvector has nonzero padding bits",
            Self::EmptyEncoding => "SSZ bitlist encoding must contain a delimiter",
            Self::NoDelimiter => "SSZ bitlist encoding has no delimiter",
            Self::TrailingZeros => "SSZ bitlist encoding has bytes after its delimiter",
            Self::EmptyContainer => "SSZ container declarations must contain at least one field",
            Self::BadDeclaration => {
                "SSZ declaration has duplicate names or mismatched selector counts"
            }
            Self::LayoutWidth => "SSZ progressive layouts must contain at least one position",
            Self::LayoutTrailingGap => "SSZ progressive layouts must end on an active position",
            Self::LayoutTooWide => "SSZ progressive layouts must not exceed 256 positions",
            Self::LayoutFieldCount => {
                "SSZ field names and active positions must match declared fields"
            }
            Self::EmptyUnion => "SSZ compatible unions must declare selectors and options",
            Self::UnionSelectorRepeated => "SSZ compatible union selectors must be distinct",
            Self::UnionSelectorRange => "SSZ compatible union selectors must be between 1 and 127",
            Self::UnionIncompatible => {
                "SSZ compatible union options must have pairwise compatible layouts"
            }
        })
    }
}

impl std::error::Error for CodecError {}
