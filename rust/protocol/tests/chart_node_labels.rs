use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError,
    chart_node_labels::{self as labels, Line, Node},
    decode_chart_node_labels,
};

fn bytes(nodes: &Vec<Node>) -> Vec<u8> {
    let mut out = vec![];
    nodes.binprot_write(&mut out).unwrap();
    out
}
fn line(text: &str) -> Line {
    Line {
        text: text.into(),
        color: None,
        font_size: None,
    }
}
#[test]
fn independent_label_fixture_truncation_utf8_and_trailing_bytes() {
    let nodes = vec![Node {
        node: 9,
        lines: vec![
            Line {
                text: "Hi".into(),
                color: Some(7),
                font_size: Some(16.),
            },
            line(""),
        ],
    }];
    let hex = "0109020248690107010000000000003040000000";
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes(&nodes), expected);
    assert_eq!(decode_chart_node_labels(&expected), Ok(nodes));
    for end in 0..expected.len() {
        assert!(decode_chart_node_labels(&expected[..end]).is_err());
    }
    let mut invalid = expected.clone();
    invalid[4] = 255;
    assert_eq!(
        decode_chart_node_labels(&invalid),
        Err(DecodeError::Malformed)
    );
    let mut trailing = expected;
    trailing.push(0);
    assert_eq!(
        decode_chart_node_labels(&trailing),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_chart_node_labels(&vec![0; labels::MAX_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn public_wire_bounds_and_retained_allocations_are_explicit() {
    for text in ["\0", "\n", "\r", "\t", "\x7f", &"a".repeat(257)] {
        assert!(!line(text).is_valid());
    }
    for n in [f64::NAN, f64::INFINITY, 7.99, 32.01] {
        assert!(
            !Line {
                font_size: Some(n),
                ..line("λ")
            }
            .is_valid()
        );
    }
    for n in [8., 32.] {
        assert!(
            Line {
                font_size: Some(n),
                ..line("λ")
            }
            .is_valid()
        );
    }
    for n in [-1, 0x1_0000_0000] {
        assert!(
            !Line {
                color: Some(n),
                ..line("color")
            }
            .is_valid()
        );
    }
    let mut nodes: Vec<_> = (1..=128)
        .map(|node| Node {
            node,
            lines: vec![line(&"x".repeat(256))],
        })
        .collect();
    assert!(labels::is_valid(&nodes));
    assert_eq!(decode_chart_node_labels(&bytes(&nodes)), Ok(nodes.clone()));
    assert!(labels::heap_bytes(&nodes) >= 32768);
    nodes[0].lines.push(line("x"));
    assert!(!labels::is_valid(&nodes));
    assert_eq!(
        decode_chart_node_labels(&bytes(&nodes)),
        Err(DecodeError::Malformed)
    );
    nodes[0].lines.clear();
    assert!(
        labels::is_valid(&nodes),
        "empty entry explicitly hides one node label"
    );
    nodes[0].node = 2;
    assert!(!labels::is_valid(&nodes), "duplicate stable identity");
    nodes[0].node = 0;
    assert!(!labels::is_valid(&nodes));
    let excessive = vec![Node {
        node: 1,
        lines: vec![line(""); 5],
    }];
    assert!(decode_chart_node_labels(&bytes(&excessive)).is_err());
    let excessive = (1..=129)
        .map(|node| Node {
            node,
            lines: vec![],
        })
        .collect();
    assert!(decode_chart_node_labels(&bytes(&excessive)).is_err());
}

#[test]
fn maximally_populated_metadata_fits_style_and_view_envelopes() {
    use gpuio_protocol::{
        chart_style::{Key, MAX_STYLE_BYTES, Ordinal, Style},
        chart_view::Config,
        decode_chart_style, decode_chart_view_config,
    };
    let style = Style {
        palette: vec![0xffff_ffff; 32],
        ordinal: Some(Ordinal {
            domain: (0..1024).map(|i| Key::Node(i64::MAX - i)).collect(),
            range: vec![0xffff_ffff; 32],
            unknown: Some(0xffff_ffff),
        }),
        node_labels: (0..128)
            .map(|i| Node {
                node: i64::MAX - i,
                lines: (0..4)
                    .map(|j| Line {
                        text: if j == 0 {
                            "x".repeat(256)
                        } else {
                            String::new()
                        },
                        color: Some(0xffff_ffff),
                        font_size: Some(32.),
                    })
                    .collect(),
            })
            .collect(),
        ..Default::default()
    };
    assert!(style.is_valid());
    let mut encoded = vec![];
    style.binprot_write(&mut encoded).unwrap();
    assert!(encoded.len() <= MAX_STYLE_BYTES);
    assert_eq!(decode_chart_style(&encoded), Ok(style.clone()));
    assert!(style.heap_bytes() >= labels::heap_bytes(&style.node_labels));
    let config = Config {
        version: -2,
        radar_labels: vec![],
        inspection_content: vec![],
        source: None,
        label: "x".repeat(1024),
        options: Default::default(),
        sampling: Default::default(),
        style,
        legend: true,
        disabled: false,
    };
    let mut encoded = vec![];
    config.binprot_write(&mut encoded).unwrap();
    assert_eq!(decode_chart_view_config(&encoded), Ok(config));
}
