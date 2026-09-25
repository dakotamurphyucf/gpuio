use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, ResourceId, WindowId, avatar::Config, decode, v1::*};
fn request(configs: Vec<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: configs
            .into_iter()
            .enumerate()
            .flat_map(|(i, c)| {
                let id = NodeId::from_parts(i as i64, 1).unwrap();
                [
                    Op::Create(id, Kind::Avatar, "".into(), None),
                    Op::SetAvatar(id, c),
                ]
            })
            .collect(),
    })
}
fn encode(message: &Message) -> Vec<u8> {
    let mut b = vec![];
    message.binprot_write(&mut b).unwrap();
    b
}
fn config(source: Option<ImageSource>) -> Config {
    Config {
        source,
        fit: ImageFit::Cover,
        label: Some("Dakota".into()),
        fallback: "DM".into(),
    }
}
#[test]
fn avatar_fixture_and_bounded_decoder() {
    let message = request(vec![
        config(None),
        config(Some(ImageSource::Unavailable(ImageError::WrongApplication))),
        config(Some(ImageSource::Reference(
            ResourceId::from_parts(2, 1).unwrap(),
        ))),
    ]);
    let bytes = encode(&message);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/avatar-request.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for n in 0..bytes.len() {
        assert!(decode(&bytes[..n]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    let mut invalid = bytes;
    let at = invalid.windows(2).position(|b| b == b"DM").unwrap();
    invalid[at] = 255;
    assert!(decode(&invalid).is_err());
    for fallback in [
        "".into(),
        " ".into(),
        "A\nB".into(),
        "A\tB".into(),
        "\u{7f}".into(),
        "A".repeat(129),
    ] {
        assert!(
            decode(&encode(&request(vec![Config {
                fallback,
                ..config(None)
            }])))
            .is_err()
        );
    }
    let maximum = Config {
        fallback: "A".repeat(128),
        label: Some("D".repeat(4096)),
        ..config(None)
    };
    let message = request(vec![maximum]);
    assert_eq!(decode(&encode(&message)), Ok(message));
    let derived = config(Some(ImageSource::Unavailable(ImageError::Released)));
    assert!(derived.matches_image(derived.image().as_ref()));
    assert!(!derived.matches_image(None));
    assert!(config(None).matches_image(None));
}
