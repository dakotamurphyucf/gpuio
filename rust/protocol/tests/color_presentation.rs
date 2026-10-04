use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId,
    color_presentation::{Panel, Panels, Presentation, Section},
    decode,
    v1::*,
};
fn config() -> Presentation {
    Presentation {
        sections: vec![
            Section {
                featured: true,
                label: "Favorites".into(),
                count: 2,
            },
            Section {
                featured: false,
                label: "Blues".into(),
                count: 1,
            },
        ],
        swatch_size: 32.,
        featured_size: 48.,
        swatch_gap: 8.,
        section_gap: 12.,
        swatch_radius: 6.,
        channel_height: 40.,
        control_gap: 12.,
        padding: 12.,
        selected_border: Some(0x11223344),
        panels: Panels::Tabs {
            palette_label: "Swatches".into(),
            channels_label: "HSLA".into(),
            initial: Panel::Channels,
        },
        ..Presentation::default()
    }
}
fn message(p: Presentation) -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetColorPresentation(node, Some(p)),
            Op::SetColorPresentation(node, None),
        ],
    })
}
fn encode(p: Presentation) -> Vec<u8> {
    let mut bytes = vec![];
    message(p).binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn independent_fixture_and_strict_section_geometry_bounds() {
    let bytes = encode(config());
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/color-presentation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message(config())));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    for count in [i64::MIN, 0, 257, i64::MAX] {
        let mut p = config();
        p.sections[0].count = count;
        assert!(decode(&encode(p)).is_err());
    }
    for label in [
        String::new(),
        " \t".into(),
        "bad\0label".into(),
        "a".repeat(257),
    ] {
        let mut p = config();
        p.sections[0].label = label;
        assert!(decode(&encode(p)).is_err());
    }
    for p in [
        Presentation {
            sections: vec![config().sections[1].clone(); 33],
            ..config()
        },
        Presentation {
            sections: vec![config().sections[0].clone(); 2],
            ..config()
        },
        Presentation {
            sections: vec![
                Section {
                    count: 256,
                    ..config().sections[0].clone()
                },
                config().sections[1].clone(),
            ],
            ..config()
        },
        Presentation {
            outline_width: 17.,
            ..config()
        },
        Presentation {
            swatch_radius: 17.,
            ..config()
        },
        Presentation {
            swatch_size: 15.,
            ..config()
        },
        Presentation {
            selected_border: Some(-1),
            ..config()
        },
        Presentation {
            hover_border: Some(0x1_0000_0000),
            ..config()
        },
    ] {
        assert!(decode(&encode(p)).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, -1., 129.] {
        for p in [
            Presentation {
                swatch_size: bad,
                ..config()
            },
            Presentation {
                featured_size: bad,
                ..config()
            },
            Presentation {
                swatch_gap: bad,
                ..config()
            },
            Presentation {
                section_gap: bad,
                ..config()
            },
            Presentation {
                swatch_radius: bad,
                ..config()
            },
            Presentation {
                outline_width: bad,
                ..config()
            },
            Presentation {
                channel_height: bad,
                ..config()
            },
            Presentation {
                control_gap: bad,
                ..config()
            },
            Presentation {
                padding: bad,
                ..config()
            },
        ] {
            assert!(decode(&encode(p)).is_err());
        }
    }
    assert!(config().fits(3));
    assert!(!config().fits(2));
    assert!(Presentation::default().fits(0));
    assert!(Presentation::default().fits(256));
}

#[test]
fn strict_panel_tags_and_labels() {
    let bytes = encode(config());
    let mut bad = bytes.clone();
    bad[bytes.len() - 4 - 16] = 2;
    assert!(decode(&bad).is_err());
    let mut bad = bytes.clone();
    bad[bytes.len() - 5] = 2;
    assert!(decode(&bad).is_err());
    let mut bad = bytes.clone();
    bad[bytes.len() - 4 - 14] = 255;
    assert!(decode(&bad).is_err());
    for label in [
        String::new(),
        " \t".into(),
        "bad\0label".into(),
        "a".repeat(257),
    ] {
        for panels in [
            Panels::Tabs {
                palette_label: label.clone(),
                channels_label: "HSLA".into(),
                initial: Panel::Palette,
            },
            Panels::Tabs {
                palette_label: "Palette".into(),
                channels_label: label,
                initial: Panel::Channels,
            },
        ] {
            assert!(decode(&encode(Presentation { panels, ..config() })).is_err());
        }
    }
    let all = Presentation {
        panels: Panels::All,
        ..config()
    };
    assert_eq!(decode(&encode(all.clone())), Ok(message(all)));
}
