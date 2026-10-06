use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, ResourceId, chart_view::*, decode_chart_view_config};
fn config() -> Config {
    Config {
        source: Some(ResourceId::from_parts(7, 2).unwrap()),
        label: "Chart 🦀".into(),
        legend: true,
        disabled: false,
        options: Default::default(),
        sampling: Default::default(),
        style: gpuio_protocol::chart_style::Style {
            version: -2,
            palette: vec![1, 2],
            axis_color: 3,
            grid_color: 4,
            label_color: 5,
            selection_color: 6,
            gradient_end: Some(7),
            stroke_width: 2.,
            point_radius: 3.,
            bar_radius: 4.,
            area_opacity: 0.5,
            ordinal: None,
            inspection: Default::default(),
            node_labels: vec![],
        },
    }
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn bounded_chart_view_and_transaction_match_independent_fixture() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, v1::*};
    let config = config();
    let legacy = include_str!("../../../test/fixtures/chart-v4-node-labels-view.hex")
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        decode_chart_view_config(&legacy),
        Err(DecodeError::Malformed)
    );
    let bytes = encode(&config);
    let fixture = include_str!("../../../test/fixtures/chart-v5-link-colors-view.hex").trim();
    assert_eq!(hex(&bytes), fixture);
    assert_eq!(decode_chart_view_config(&bytes), Ok(config.clone()));
    for end in 0..bytes.len() {
        assert!(decode_chart_view_config(&bytes[..end]).is_err());
    }
    let mut hidden_legend = config.clone();
    hidden_legend.legend = false;
    assert_eq!(
        decode_chart_view_config(&encode(&hidden_legend)),
        Ok(hidden_legend)
    );
    let mut disabled = config.clone();
    disabled.disabled = true;
    assert_eq!(decode_chart_view_config(&encode(&disabled)), Ok(disabled));
    // The final two fields are independently validated booleans.
    for offset in [1, 2] {
        let mut malformed = bytes.clone();
        let index = malformed.len() - offset;
        malformed[index] = 2;
        assert_eq!(
            decode_chart_view_config(&malformed),
            Err(DecodeError::Malformed)
        );
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        decode_chart_view_config(&trailing),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_chart_view_config(&[0; gpuio_protocol::chart_style::MAX_STYLE_BYTES + 2 * 1024 + 1]),
        Err(DecodeError::LimitExceeded)
    );
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::ChartView, "".into(), Some(handler)),
            Op::SetChart(node, config),
        ],
    });
    let bytes = encode(&message);
    assert_eq!(
        hex(&bytes),
        format!("0300010001020000013000010001370001{fixture}")
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let event = Event::ChartEvent(
        window,
        node,
        handler,
        1,
        Some(ResourceId::from_parts(7, 2).unwrap()),
        2,
        3,
        Observation::Ready(Metrics {
            source_values: 10,
            retained_values: 5,
            mesh_vertices: 12,
            quads: 2,
            bytes: 100,
        }),
    );
    assert_eq!(
        hex(&encode(&vec![event])),
        "013e000100010001010107020203000a050c0264"
    );
}
#[test]
fn invalid_nested_configuration_and_label_cannot_reach_native_tree() {
    for label in [
        "".into(),
        " \t\u{b}\u{c}".into(),
        "\0".into(),
        "chart\n".into(),
        "x".repeat(1025),
    ] {
        let value = Config { label, ..config() };
        assert!(!value.is_valid());
        assert!(decode_chart_view_config(&encode(&value)).is_err());
    }
    let mut value = config();
    value.options.version = 1;
    assert!(decode_chart_view_config(&encode(&value)).is_err());
    let mut value = config();
    value.sampling.line = gpuio_protocol::chart_sampling::Line::Envelope(0);
    assert!(decode_chart_view_config(&encode(&value)).is_err());
    let mut value = config();
    value.style.palette.clear();
    assert!(decode_chart_view_config(&encode(&value)).is_err());
}
