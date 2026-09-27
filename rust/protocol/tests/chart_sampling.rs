use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, chart_sampling::*, decode_chart_sampling};

#[test]
fn policy_bytes_match_ocaml_and_reject_every_truncation() {
    for (policy, hex) in [
        (Policy::default(), "0101fe00040000"),
        (
            Policy {
                line: Line::Exact,
                ..Default::default()
            },
            "01000000",
        ),
        (
            Policy {
                version: 1,
                line: Line::Envelope(1),
                bars: Bar::Sum(8),
                candles: Candlestick::Ohlc(8192),
            },
            "010101010801fe0020",
        ),
        (
            Policy {
                bars: Bar::Mean(128),
                ..Default::default()
            },
            "0101fe000402fe800000",
        ),
    ] {
        let expected: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
            .collect();
        let mut bytes = vec![];
        policy.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, expected);
        assert_eq!(decode_chart_sampling(&bytes), Ok(policy));
        for end in 0..bytes.len() {
            assert!(decode_chart_sampling(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert_eq!(decode_chart_sampling(&bytes), Err(DecodeError::Malformed));
    }
}

#[test]
fn invalid_policies_are_rejected_before_geometry() {
    for policy in [
        Policy {
            version: 2,
            ..Default::default()
        },
        Policy {
            line: Line::Envelope(0),
            ..Default::default()
        },
        Policy {
            bars: Bar::Sum(8193),
            ..Default::default()
        },
        Policy {
            bars: Bar::Mean(-1),
            ..Default::default()
        },
        Policy {
            candles: Candlestick::Ohlc(i64::MAX),
            ..Default::default()
        },
    ] {
        assert!(!policy.is_valid());
        let mut bytes = vec![];
        policy.binprot_write(&mut bytes).unwrap();
        assert_eq!(decode_chart_sampling(&bytes), Err(DecodeError::Malformed));
    }
    assert_eq!(
        decode_chart_sampling(&[0; 65]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_chart_sampling(&[1, 2, 0, 0]),
        Err(DecodeError::Malformed)
    );
}
