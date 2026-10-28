#![doc = include_str!("../../../README.md")]
#![deny(unsafe_code)]
#![cfg_attr(feature = "verification", feature(register_tool))]
#![cfg_attr(feature = "verification", register_tool(verify))]

extern crate self as nyx_ssz;

pub use nyx_ssz_derive::Ssz;
pub use ruint::aliases::U256;

mod bit_codec;
mod bit_encoding;
mod bit_storage;
pub mod bitcollections;
pub mod bitfield;
mod bulk;
pub mod bytecollections;
pub mod bytes;
pub mod codec;
mod collection_codec;
mod collection_tree;
mod collection_wire;
pub mod collections;
#[doc(hidden)]
pub mod composite;
pub mod container;
pub mod container_wire;
#[doc(hidden)]
pub mod encoding;
pub mod error;
pub mod gindex;
pub mod merkle;
pub mod multiproof;
pub mod node;
pub mod path;
mod progressive_codec;
pub mod proof;
pub mod scalar;
pub mod sequence;
pub mod sha256;
pub mod tree_hash;
pub mod type_info;
