use binprot::BinProtWrite;
use gpuio_protocol::{WindowId, decode, v1::*, window::*};
fn hex(message: &Message) -> String {
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(decode(&bytes).unwrap(), *message);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
#[test]
fn window_messages_match_ocaml_fixtures() {
    let id = WindowId::from_parts(0, 1).unwrap();
    for (command, expected) in [
        (Command::Observe, "0b07000100"),
        (Command::SetTitle("x".into()), "0b070001010178"),
        (
            Command::Resize(640., 400.),
            "0b0700010200000000000084400000000000007940",
        ),
        (Command::Activate, "0b07000103"),
        (Command::Zoom, "0b07000104"),
        (Command::ToggleFullscreen, "0b07000105"),
        (Command::SetEdited(true), "0b0700010601"),
    ] {
        assert_eq!(hex(&Message::WindowCommand(7, id, command)), expected);
    }
    assert_eq!(
        hex(&Message::OpenConfigured(
            7,
            id,
            Config {
                title: "x".into(),
                width: 640.,
                height: 400.,
                focus: false,
                chrome: Chrome::Hidden,
                resizable: false
            }
        )),
        "0c070001017800000000000084400000000000007940000100"
    );
    let mut bytes = Vec::new();
    vec![
        Event::CloseRequested(id),
        Event::QuitRequested,
        Event::ReopenRequested,
    ]
    .binprot_write(&mut bytes)
    .unwrap();
    assert_eq!(bytes, vec![3, 31, 0, 1, 32, 33]);
}
#[test]
fn native_decoder_rejects_invalid_window_data() {
    let id = WindowId::from_parts(0, 1).unwrap();
    for command in [
        Command::Resize(f64::NAN, 400.),
        Command::Resize(0., 400.),
        Command::SetTitle("bad\0title".into()),
    ] {
        let mut bytes = Vec::new();
        Message::WindowCommand(7, id, command)
            .binprot_write(&mut bytes)
            .unwrap();
        assert!(decode(&bytes).is_err());
    }
}
