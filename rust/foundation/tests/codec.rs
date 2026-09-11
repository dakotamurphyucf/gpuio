use binprot::macros::{BinProtRead, BinProtWrite};
use binprot::{BinProtRead, BinProtWrite};
use std::io::Cursor;

#[derive(Debug, PartialEq, BinProtRead, BinProtWrite)]
enum Op {
    Clear,
    SetText(i64, String),
    SetChildren(i64, Vec<i64>),
    Listen(i64, bool, Option<i64>),
    Scale(f64),
}

#[derive(Debug, PartialEq, BinProtRead, BinProtWrite)]
struct Batch {
    version: i64,
    revision: i64,
    ops: Vec<Op>,
}

#[test]
fn independent_ocaml_and_rust_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let expected = Batch {
        version: 1,
        revision: 65536,
        ops: vec![
            Op::Clear,
            Op::SetText(127, "Hello λ 🦀\0".into()),
            Op::SetChildren(
                128,
                vec![
                    -129,
                    -128,
                    -1,
                    0,
                    255,
                    256,
                    32767,
                    32768,
                    i64::MIN,
                    i64::MAX,
                ],
            ),
            Op::Listen(2147483648, true, Some(42)),
            Op::Listen(0, false, None),
            Op::Scale(1.25),
        ],
    };
    let hex = include_str!("../../../test/fixtures/codec-v1.hex").trim();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut input = Cursor::new(&bytes);
    let actual = Batch::binprot_read(&mut input)?;
    assert_eq!(actual, expected);
    assert_eq!(input.position() as usize, bytes.len());
    let mut output = vec![];
    expected.binprot_write(&mut output)?;
    assert_eq!(bytes, output);
    println!(
        "Rust decoded OCaml fixture and produced identical bytes ({} bytes)",
        bytes.len()
    );
    Ok(())
}
