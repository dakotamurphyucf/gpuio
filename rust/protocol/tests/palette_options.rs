use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, NodeId, WindowId, decode,
    palette_options::{Config, Escape, Keywords, Search},
    v1::*,
};
fn config() -> Config {
    Config {
        search: Search::Substring,
        searchable: false,
        escape: Escape::ClearQueryFirst,
        keywords: vec![Keywords {
            command: "run".into(),
            words: vec!["execute λ".into()],
        }],
    }
}
fn message(config: Option<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetPaletteOptions(
            NodeId::from_parts(1, 1).unwrap(),
            config,
        )],
    })
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn palette_options_codec_bounds_and_reset() {
    for options in [Some(config()), None] {
        let msg = message(options);
        let bytes = encode(&msg);
        assert_eq!(decode(&bytes), Ok(msg));
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
    }
    let valid = encode(&message(Some(config())));
    for (offset, tag) in [(9, 2), (10, 3), (11, 2), (12, 2)] {
        let mut malformed = valid.clone();
        malformed[offset] = tag;
        assert_eq!(decode(&malformed), Err(DecodeError::Malformed));
    }
    let mut bad = config();
    bad.keywords.push(bad.keywords[0].clone());
    assert_eq!(
        decode(&encode(&message(Some(bad)))),
        Err(DecodeError::Malformed)
    );
    for word in [" ".into(), "bad\0word".into(), "x".repeat(4097)] {
        let mut bad = config();
        bad.keywords[0].words = vec![word];
        assert!(decode(&encode(&message(Some(bad)))).is_err());
    }
    // Keep Core.String.strip semantics: non-ASCII spaces are literal text,
    // not blank metadata. This matches the existing command-ID contract.
    let mut unicode = config();
    unicode.keywords[0].words = vec!["\u{a0}\u{3000}".into()];
    let unicode = message(Some(unicode));
    assert_eq!(decode(&encode(&unicode)), Ok(unicode));
    let mut bad = config();
    bad.keywords[0].words = vec!["x".into(); 65];
    assert!(decode(&encode(&message(Some(bad)))).is_err());
    let mut bad = config();
    bad.keywords[0].words = vec!["x".repeat(4096); 64];
    assert!(decode(&encode(&message(Some(bad)))).is_err());
    // Independent bin_prot bytes: Apply/window/base/revision/count, tag125,
    // node1/generation1, Some, substring/false/clear, one ID with one UTF-8 word.
    assert_eq!(
        encode(&message(Some(config()))),
        b"\x03\x00\x01\x00\x01\x01\x7d\x01\x01\x01\x01\x00\x01\x01\x03run\x01\x0aexecute \xce\xbb"
    );
}
#[test]
fn palette_options_search_and_shared_metadata_budget() {
    let words = vec!["execute λ".into(), "launch".into()];
    assert!(Search::AllTerms.matches("run task", "action.run", &words, "λ action launch"));
    assert!(!Search::AllTerms.matches("run task", "action.run", &words, "λ missing"));
    assert!(Search::Substring.matches("run task", "action.run", &words, "execute λ"));
    assert!(Search::Substring.matches("run task", "action.run", &words, "  \u{2003}execute λ\t"));
    assert!(Search::Substring.matches("run task", "action.run", &[], " \u{2003}"));
    assert!(!Search::Substring.matches("run task", "action.run", &words, "execute  λ"));
    assert!(!Search::Substring.matches("run task", "action.run", &words, "action.run"));
    assert!(!Search::Substring.matches("run task", "action.run", &words, "λ launch"));
    assert!(Search::Unfiltered.matches("", "", &[], "anything"));
    let mut palette = PaletteConfig {
        label: "Actions".into(),
        placeholder: "".into(),
        commands: vec!["run".into()],
        dismiss_on_outside_pointer: true,
    };
    assert!(config().fits(&palette));
    palette.commands.clear();
    assert!(!config().fits(&palette));
    let mut options = Config::default();
    options.keywords.push(Keywords {
        command: "run".into(),
        words: vec!["x".repeat(4096); 63],
    });
    options.keywords[0].words.push("x".repeat(4093));
    assert_eq!(options.text_bytes(), 262144);
    assert!(options.is_valid());
    palette.commands.push("run".into());
    assert!(!options.fits(&palette));
}
