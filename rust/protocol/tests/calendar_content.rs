use binprot::BinProtWrite;
use gpuio_protocol::{
    calendar_content::{Config, Item, Slot},
    decode_calendar_content,
};

fn item(slot: Slot, description: Option<&str>) -> Item {
    Item {
        slot,
        description: description.map(str::to_owned),
    }
}
fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = Vec::new();
    config.binprot_write(&mut bytes).unwrap();
    bytes
}
fn config() -> Config {
    Config {
        items: vec![
            item(Slot::Previous, Some("Back")),
            item(Slot::Next, None),
            item(Slot::ChooseMonth, None),
            item(Slot::ChooseYear, None),
            item(Slot::Today, None),
            item(Slot::Clear, None),
            item(Slot::Day(0), Some("Holiday")),
            item(Slot::Month(12), None),
            item(Slot::Year(9999), None),
            item(Slot::MonthHeading(0), None),
            item(Slot::Weekday(119987, 6), Some("Weekend")),
        ],
    }
}

#[test]
fn all_slot_tags_match_independent_fixture_and_decode_strictly() {
    let config = config();
    let bytes = encode(&config);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/calendar-content.hex").trim()
    );
    assert_eq!(decode_calendar_content(&bytes), Ok(config));
    for end in 0..bytes.len() {
        assert!(decode_calendar_content(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode_calendar_content(&extra).is_err());
    for (at, value) in [(1, 11), (2, 2), (4, 0xff)] {
        let mut bad = bytes.clone();
        bad[at] = value;
        assert!(decode_calendar_content(&bad).is_err());
    }
}

#[test]
fn content_operation_is_append_only_and_preserves_metadata_bytes() {
    use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetCalendarContent(node, Some(config())),
            Op::SetCalendarContent(node, None),
        ],
    });
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/calendar-content-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}

#[test]
fn civil_domains_duplicates_and_allocation_budgets_are_enforced() {
    let decode = |slot, description| {
        decode_calendar_content(&encode(&Config {
            items: vec![item(slot, description)],
        }))
    };
    for slot in [
        Slot::Day(-1),
        Slot::Day(3652059),
        Slot::Month(0),
        Slot::Month(13),
        Slot::Year(0),
        Slot::Year(10000),
        Slot::MonthHeading(-1),
        Slot::MonthHeading(119988),
        Slot::Weekday(0, -1),
        Slot::Weekday(0, 7),
        Slot::Weekday(119988, 0),
        Slot::Day(i64::MAX),
    ] {
        assert!(decode(slot, None).is_err());
    }
    for slot in [
        Slot::Day(3652058),
        Slot::Month(1),
        Slot::Year(1),
        Slot::MonthHeading(119987),
        Slot::Weekday(0, 0),
    ] {
        assert!(decode(slot, Some("é🎉")).is_ok());
    }
    for text in ["", "   ", "\t", "line\nfeed", "\0", "\u{7f}"] {
        assert!(decode(Slot::Previous, Some(text)).is_err());
    }
    assert!(decode(Slot::Previous, Some(&"x".repeat(1025))).is_err());
    let duplicate = Config {
        items: vec![
            item(Slot::Day(0), None),
            item(Slot::Day(0), Some("duplicate")),
        ],
    };
    assert!(!duplicate.is_valid());
    assert!(decode_calendar_content(&encode(&duplicate)).is_err());
    let unsorted = Config {
        items: vec![item(Slot::Next, None), item(Slot::Previous, None)],
    };
    assert!(!unsorted.is_valid());
    assert!(decode_calendar_content(&encode(&unsorted)).is_err());
    let mut many = Config {
        items: (0..1024).map(|n| item(Slot::Day(n), None)).collect(),
    };
    assert!(decode_calendar_content(&encode(&many)).is_ok());
    many.items.push(item(Slot::Day(1024), None));
    assert!(decode_calendar_content(&encode(&many)).is_err());
    let description = "x".repeat(1024);
    let mut bytes = Config {
        items: (0..64)
            .map(|n| item(Slot::Day(n), Some(&description)))
            .collect(),
    };
    assert!(decode_calendar_content(&encode(&bytes)).is_ok());
    bytes.items.push(item(Slot::Day(64), Some("x")));
    assert!(!bytes.is_valid());
    assert!(decode_calendar_content(&encode(&bytes)).is_err());
    assert!(decode_calendar_content(&[0xff, 0xff, 0xff, 0xff, 0xff]).is_err());
    assert!(decode_calendar_content(&vec![0; 98305]).is_err());
}
