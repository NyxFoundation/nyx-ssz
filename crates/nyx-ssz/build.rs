use sha2::{Digest, Sha256};
use std::{env, fmt::Write as _, fs, path::PathBuf};

fn main() {
    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set")).join("zero_hashes.rs");
    fs::write(output, zero_hashes_source()).expect("write zero hash table");
}

fn zero_hashes_source() -> String {
    let mut source = String::from("const ZERO_HASHES: [Node; 65] = [\n");
    let mut node = [0_u8; 32];
    for depth in 0..=64 {
        writeln!(source, "    {node:?}, // depth {depth}").expect("write to String");
        let mut hasher = Sha256::new();
        hasher.update(node);
        hasher.update(node);
        node = hasher.finalize().into();
    }
    source.push_str("];\n");
    source
}
