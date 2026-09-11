//! Deliberate fixture refresh tool; review both languages before replacing goldens.
#[path = "../tests/common/mod.rs"]
mod common;
use binprot::BinProtWrite;

fn hex(value: &impl BinProtWrite) -> String {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    println!("request={}", hex(&common::request()));
    println!("events={}", hex(&common::events()));
}
