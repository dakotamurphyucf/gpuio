use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, decode_input_format,
    input_format::{Config, Error, MAX_TEXT_BYTES, Number},
};

fn unhex(text: &str) -> Vec<u8> {
    if text == "-" {
        return vec![];
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn editor_format_operation_has_independent_paired_bytes_and_bounded_decoding() {
    use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
    let node = NodeId::from_parts(1, 2).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetEditorFormat(node, None),
            Op::SetEditorFormat(node, Some(Config::Pattern("*–99".into()))),
            Op::SetEditorFormat(
                node,
                Some(Config::Number(Number {
                    separator: Some(",".into()),
                    fraction_digits: Some(2),
                })),
            ),
            Op::SetEditorFormat(
                node,
                Some(Config::Number(Number {
                    separator: None,
                    fraction_digits: None,
                })),
            ),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    let expected = unhex(include_str!("../../../test/fixtures/input-format-operation.hex").trim());
    assert_eq!(bytes, expected);
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert_eq!(decode(&extra), Err(DecodeError::Malformed));
    for config in [
        Config::Pattern("".into()),
        Config::Pattern("9".repeat(257)),
        Config::Number(Number {
            separator: Some("1".into()),
            fraction_digits: None,
        }),
        Config::Number(Number {
            separator: None,
            fraction_digits: Some(-1),
        }),
    ] {
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetEditorFormat(node, Some(config))],
        });
        let mut bytes = vec![];
        message.binprot_write(&mut bytes).unwrap();
        assert_eq!(decode(&bytes), Err(DecodeError::Malformed));
    }
}
#[test]
fn independent_ocaml_rust_fixtures_agree_on_wire_and_conversion() {
    for line in include_str!("../../../test/fixtures/input-format-values.tsv")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 6);
        let bytes = unhex(row[1]);
        let config = decode_input_format(&bytes).unwrap();
        assert_eq!(encode(&config), bytes, "{}", row[0]);
        let input = unhex(row[3]);
        let actual = std::str::from_utf8(&input)
            .map_err(|_| Error::InvalidText)
            .and_then(|input| match row[2] {
                "raw" => config.format_raw(input),
                "formatted" => config.raw_of_formatted(input),
                _ => panic!("unknown fixture operation"),
            });
        let expected = match row[4] {
            "ok" => Ok(String::from_utf8(unhex(row[5])).unwrap()),
            "fit" => Err(Error::DoesNotFit),
            "text" => Err(Error::InvalidText),
            _ => panic!("unknown fixture result"),
        };
        assert_eq!(actual, expected, "{}", row[0]);
        for end in 0..bytes.len() {
            assert!(decode_input_format(&bytes[..end]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert_eq!(decode_input_format(&extra), Err(DecodeError::Malformed));
    }
}
#[test]
fn malformed_formats_and_oversized_claims_are_rejected_before_use() {
    let invalid = [
        Config::Pattern("".into()),
        Config::Pattern("*".repeat(257)),
        Config::Pattern("9\0".into()),
        Config::Number(Number {
            separator: Some("١".into()),
            fraction_digits: None,
        }),
        Config::Number(Number {
            separator: Some("ab".into()),
            fraction_digits: None,
        }),
        Config::Number(Number {
            separator: Some("，".into()),
            fraction_digits: None,
        }),
        Config::Number(Number {
            separator: None,
            fraction_digits: Some(-1),
        }),
        Config::Number(Number {
            separator: None,
            fraction_digits: Some(262145),
        }),
    ];
    for config in invalid {
        assert!(!config.is_valid());
        assert_eq!(
            decode_input_format(&encode(&config)),
            Err(DecodeError::Malformed)
        );
        assert_eq!(config.format_raw(""), Err(Error::InvalidConfig));
    }
    for data in [vec![2], vec![1, 2], vec![1, 0, 2], vec![0, 1, 255]] {
        assert_eq!(decode_input_format(&data), Err(DecodeError::Malformed));
    }
    // Pattern claims 1025 bytes before supplying any; no string is allocated.
    assert_eq!(
        decode_input_format(&[0, 0xfe, 1, 4]),
        Err(DecodeError::LimitExceeded)
    );
    // Separator claims five bytes.
    assert_eq!(
        decode_input_format(&[1, 1, 5]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_input_format(&vec![0; 2049]),
        Err(DecodeError::LimitExceeded)
    );
}
#[test]
fn expansion_is_bounded_and_long_decimal_values_are_exact() {
    let config = Config::Number(Number {
        separator: Some(",".into()),
        fraction_digits: None,
    });
    let raw = format!("+{}", "1".repeat(196608));
    let formatted = config.format_raw(&raw).unwrap();
    assert_eq!(formatted.len(), MAX_TEXT_BYTES);
    assert_eq!(config.raw_of_formatted(&formatted), Ok(raw));
    assert_eq!(
        config.format_raw(&"1".repeat(196609)),
        Err(Error::LimitExceeded)
    );
    assert_eq!(
        config.format_raw(&"1".repeat(MAX_TEXT_BYTES + 1)),
        Err(Error::LimitExceeded)
    );
    let raw = "000123456789012345678901234567890.001000";
    assert_eq!(
        config.raw_of_formatted(&config.format_raw(raw).unwrap()),
        Ok(raw.to_owned())
    );
    assert_eq!(unicode_properties::UNICODE_VERSION, (17, 0, 0));
}
