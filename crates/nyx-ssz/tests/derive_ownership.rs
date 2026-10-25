use nyx_ssz::{codec::Ssz, error::CodecError, type_info::TypeDescriptor as D};
use std::sync::atomic::{AtomicUsize, Ordering};

static DROPS: AtomicUsize = AtomicUsize::new(0);
static DECODES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Owner;
impl Drop for Owner {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}
impl Ssz for Owner {
    const TYPE: &'static D = &D::Uint(1);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        Ok(1)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        nyx_ssz::encoding::copy(output, &[0])
    }
    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        DECODES.fetch_add(1, Ordering::SeqCst);
        if bytes == [0] {
            Ok(Self)
        } else {
            Err(CodecError::InvalidBoolean)
        }
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok([0; 32])
    }
}

#[derive(nyx_ssz::Ssz)]
struct Record {
    first: Owner,
    second: nyx_ssz::collections::List<Owner, 4>,
    third: Owner,
}
#[derive(nyx_ssz::Ssz)]
enum Choice {
    #[ssz(selector = 1)]
    A(Owner),
    #[ssz(selector = 2)]
    B(Owner),
}

#[test]
fn should_drop_heterogeneous_prefixes_when_nested_decoding_fails() {
    DROPS.store(0, Ordering::SeqCst);
    DECODES.store(0, Ordering::SeqCst);
    assert_eq!(
        Record::decode(&[0, 6, 0, 0, 0, 0, 0, 1]).err().unwrap(),
        CodecError::InvalidBoolean
    );
    assert_eq!(DECODES.load(Ordering::SeqCst), 3);
    assert_eq!(DROPS.load(Ordering::SeqCst), 2);

    assert_eq!(
        Record::decode(&[0]).err().unwrap(),
        CodecError::ScopeTooSmall
    );
    assert_eq!(DECODES.load(Ordering::SeqCst), 3);
    assert_eq!(
        Choice::decode(&[2, 1]).err().unwrap(),
        CodecError::InvalidBoolean
    );
    assert_eq!(DROPS.load(Ordering::SeqCst), 2);
}

#[derive(Debug)]
struct Oversized;
impl Ssz for Oversized {
    const TYPE: &'static D = &D::Vector(&D::Uint(32), usize::MAX);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        Err(CodecError::NativeOutputOverflow)
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        Err(CodecError::NativeOutputOverflow)
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        Err(CodecError::InvalidLength)
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok([0; 32])
    }
}

struct Refusal;
impl Ssz for Refusal {
    const TYPE: &'static D = &D::Boolean;
    fn encoded_len(&self) -> Result<usize, CodecError> {
        Err(CodecError::InvalidBoolean)
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        Err(CodecError::InvalidBoolean)
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        Err(CodecError::InvalidBoolean)
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Err(CodecError::InvalidBoolean)
    }
}

#[derive(nyx_ssz::Ssz)]
struct Deferred {
    huge: Oversized,
    later: Refusal,
}
#[derive(nyx_ssz::Ssz)]
struct Budget {
    huge: Oversized,
    later: u8,
}
#[derive(nyx_ssz::Ssz)]
enum HugeUnion {
    #[ssz(selector = 1)]
    Huge(Oversized),
}
#[derive(nyx_ssz::Ssz)]
enum InvalidUnion {
    #[ssz(selector = 1)]
    A(Owner),
    #[ssz(selector = 2)]
    B(bool),
}

#[test]
fn should_preserve_protocol_error_precedence_when_a_child_has_oversized_native_success() {
    assert_eq!(
        Deferred {
            huge: Oversized,
            later: Refusal
        }
        .encode(),
        Err(CodecError::InvalidBoolean)
    );
    assert_eq!(
        Budget {
            huge: Oversized,
            later: 0
        }
        .encode(),
        Err(CodecError::SizeOverflow)
    );
    assert_eq!(
        HugeUnion::Huge(Oversized).encode(),
        Err(CodecError::NativeOutputOverflow)
    );
    assert_eq!(
        InvalidUnion::decode(&[]).err().unwrap(),
        CodecError::UnionIncompatible
    );
}

mod private {
    #[derive(nyx_ssz::Ssz)]
    struct Child {
        value: u8,
    }
    #[derive(nyx_ssz::Ssz)]
    pub struct Public {
        child: Child,
    }
    pub fn make() -> Public {
        Public {
            child: Child { value: 1 },
        }
    }
}

#[test]
fn should_allow_public_containers_when_their_field_types_are_private() {
    assert_eq!(private::make().encode().unwrap(), vec![1]);
}
