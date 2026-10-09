use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    palette_layout::{Config, Entry},
    palette_options,
    v1::*,
};
fn message(layout: Option<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetPaletteLayout(
            NodeId::from_parts(1, 1).unwrap(),
            layout,
        )],
    })
}
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn layout() -> Config {
    Config(vec![
        Entry::Group("g".into(), Some("Group".into()), vec![0]),
        Entry::Separator,
        Entry::Command(1),
    ])
}
#[test]
fn bounded_layout_codec_and_independent_fixture() {
    for config in [Some(layout()), None] {
        let message = message(config);
        let bytes = bytes(&message);
        assert_eq!(decode(&bytes), Ok(message));
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
    }
    assert_eq!(
        bytes(&message(Some(layout()))),
        b"\x03\x00\x01\x00\x01\x01\x7e\x01\x01\x01\x03\x01\x01g\x01\x05Group\x01\x00\x02\x00\x01"
    );
    let invalid = vec![
        Config(vec![Entry::Command(-1)]),
        Config(vec![Entry::Command(1)]),
        Config(vec![Entry::Command(0), Entry::Command(0)]),
        Config(vec![Entry::Separator; 1025]),
        Config(vec![Entry::Group("same".into(), None, vec![]); 2]),
        Config(vec![Entry::Group(" ".into(), None, vec![])]),
        Config(vec![Entry::Group("g".into(), Some("\0".into()), vec![])]),
        Config(vec![Entry::Group("g".into(), None, (0..1025).collect())]),
    ];
    for config in invalid {
        assert!(!config.is_valid());
        assert!(decode(&bytes(&message(Some(config)))).is_err());
    }
    // Aggregate allocation bound applies across groups, before allocating each list.
    let config = Config(vec![
        Entry::Group("a".into(), None, (0..1024).collect()),
        Entry::Group("b".into(), None, vec![1024]),
    ]);
    assert!(decode(&bytes(&message(Some(config)))).is_err());
}
#[test]
fn layout_must_cover_flat_commands_and_share_search_budget() {
    let mut palette = PaletteConfig {
        label: "Actions".into(),
        placeholder: "".into(),
        commands: vec!["run".into(), "copy".into()],
        dismiss_on_outside_pointer: true,
    };
    assert!(layout().fits(&palette, None));
    palette.commands.pop();
    assert!(!layout().fits(&palette, None));
    let mut large = Config(
        (0..64)
            .map(|i| {
                Entry::Group(
                    format!("g{i}"),
                    Some("x".repeat(4000)),
                    if i == 0 { vec![0] } else { vec![] },
                )
            })
            .collect(),
    );
    assert!(large.fits(&palette, None));
    let options = palette_options::Config {
        keywords: vec![palette_options::Keywords {
            command: "run".into(),
            words: vec!["x".repeat(4096); 2],
        }],
        ..Default::default()
    };
    assert!(options.fits(&palette));
    assert!(!large.fits(&palette, Some(&options)));
    large
        .0
        .push(Entry::Group("extra".into(), Some("x".repeat(4097)), vec![]));
    assert!(!large.is_valid());
}
