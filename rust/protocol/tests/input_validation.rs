use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_input_validation_source,
    input_validation::{Error, Preparation},
};

fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn input_validation_source_and_preparation_match_independent_ocaml_bytes() {
    for line in include_str!("../../../test/fixtures/input-validation-source.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        let bytes = unhex(fields[1]);
        let source = decode_input_validation_source(&bytes).unwrap();
        let mut actual = vec![];
        source.binprot_write(&mut actual).unwrap();
        assert_eq!(actual, bytes);
        for end in 0..bytes.len() {
            assert!(decode_input_validation_source(&bytes[..end]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(decode_input_validation_source(&extra).is_err());
    }
    for (result, hex) in [
        (Preparation::Checked, "00"),
        (Preparation::Failed(Error::InvalidSource), "0100"),
        (Preparation::Failed(Error::TooComplex), "0102"),
        (
            Preparation::Failed(Error::InvalidRegex("bad".into())),
            "010103626164",
        ),
    ] {
        let mut bytes = vec![];
        result.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, unhex(hex));
    }
    for hex in ["000201", "000002", "01ff0001", "01000001", "fdffffff7f"] {
        assert!(decode_input_validation_source(&unhex(hex)).is_err());
    }
    assert!(decode_input_validation_source(&vec![0; 2065]).is_err());
}

#[test]
fn editor_validation_operation_has_independent_paired_bytes() {
    use gpuio_protocol::{
        NodeId, WindowId,
        input_validation::{Matching, Rule, Source},
        v1::*,
    };
    let node = NodeId::from_parts(1, 2).unwrap();
    let rule = |pattern: &str, matching, case_sensitive, allow_empty| Rule {
        regex: Source {
            pattern: pattern.into(),
            matching,
            case_sensitive,
        },
        allow_empty,
    };
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetEditorValidation(node, None),
            Op::SetEditorValidation(node, Some(rule("[0-9]*", Matching::WholeValue, true, true))),
            Op::SetEditorValidation(
                node,
                Some(rule(r"\p{L}+", Matching::Substring, false, false)),
            ),
        ],
    });
    let expected =
        unhex(include_str!("../../../test/fixtures/input-validation-operation.hex").trim());
    let mut actual = vec![];
    message.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), message);
    for end in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..end]).is_err());
    }
    let mut malformed = expected;
    *malformed.last_mut().unwrap() = 2;
    assert!(gpuio_protocol::decode(&malformed).is_err());
}
