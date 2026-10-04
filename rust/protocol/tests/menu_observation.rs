use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::Event};

#[test]
fn menu_visibility_edges_match_independent_ocaml_fixture() {
    let events: Vec<_> = [true, false]
        .into_iter()
        .map(|open| {
            Event::MenuOpenChanged(
                WindowId::from_parts(0, 1).unwrap(),
                NodeId::from_parts(1, 2).unwrap(),
                HandlerId::from_parts(2, 3).unwrap(),
                7,
                open,
            )
        })
        .collect();
    let mut bytes = vec![];
    events.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/menu-observation.hex").trim()
    );
}
