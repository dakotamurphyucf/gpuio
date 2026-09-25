use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
#[test]
fn submit_command_matches_ocaml_fixture() {
    let message = Message::EditorCommand(
        7,
        WindowId::from_parts(0, 1).unwrap(),
        NodeId::from_parts(0, 1).unwrap(),
        EditorCommand::Submit,
    );
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(bytes, [6, 7, 0, 1, 0, 1, 5]);
    assert_eq!(decode(&bytes).unwrap(), message);
}
