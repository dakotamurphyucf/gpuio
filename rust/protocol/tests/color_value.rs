use gpuio_protocol::color_value::{DraftError, HexDraft, Hsla, Rgba, Value};

#[test]
fn independent_color_vectors() {
    let fixture = include_str!("../../../test/fixtures/color-values.tsv");
    assert_eq!(fixture.lines().count(), 12);
    for line in fixture.lines() {
        let values: Vec<f64> = line.split('\t').map(|v| v.parse().unwrap()).collect();
        assert_eq!(values.len(), 8);
        let rgba = Rgba::new(
            values[0] as u8,
            values[1] as u8,
            values[2] as u8,
            values[3] as u8,
        );
        let expected = Hsla::new(values[4], values[5], values[6], values[7]).unwrap();
        let actual = rgba.to_hsla();
        for (a, b) in [
            (actual.hue_degrees(), expected.hue_degrees()),
            (actual.saturation(), expected.saturation()),
            (actual.lightness(), expected.lightness()),
            (actual.alpha(), expected.alpha()),
        ] {
            assert!((a - b).abs() <= 1e-10, "{line}");
        }
        assert_eq!(Rgba::of_hsla(expected), rgba);
    }
}

#[test]
fn byte_roundtrips_preserve_transparent_colors() {
    let mut count = 0;
    for red in (0..=255).step_by(17) {
        for green in (0..=255).step_by(17) {
            for blue in (0..=255).step_by(17) {
                for alpha in [0, 1, 64, 127, 128, 255] {
                    let rgba = Rgba::new(red, green, blue, alpha);
                    assert_eq!(Rgba::of_hsla(rgba.to_hsla()), rgba);
                    assert_eq!(Rgba::of_hex(&rgba.to_hex()), Some(rgba));
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 24576);
    assert_ne!(Value::Empty, Value::Color(Rgba::new(0, 0, 0, 0)));
}

#[test]
fn validation_hue_wrap_and_quantization() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 360.1] {
        assert!(Hsla::new(value, 1., 0.5, 1.).is_none());
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.1] {
        assert!(Hsla::new(0., value, 0.5, 1.).is_none());
        assert!(Hsla::new(0., 1., value, 1.).is_none());
        assert!(Hsla::new(0., 1., 0.5, value).is_none());
    }
    assert_eq!(Hsla::new(360., 1., 0.5, 1.), Hsla::new(0., 1., 0.5, 1.));
    let zero = Hsla::new(-0., -0., -0., -0.).unwrap();
    assert_eq!(
        [
            zero.hue_degrees(),
            zero.saturation(),
            zero.lightness(),
            zero.alpha()
        ]
        .map(f64::to_bits),
        [0; 4]
    );
    for i in 0..=10000 {
        let value = f64::from(i) / 10000.;
        let color = Rgba::of_hsla(Hsla::new(123., 0., value, value).unwrap());
        assert!((f64::from(color.red) / 255. - value).abs() <= 0.5 / 255. + 1e-15);
        assert_eq!(
            [color.red; 4],
            [color.red, color.green, color.blue, color.alpha]
        );
    }
    for (hue, expected) in [
        (0., "#FF000080"),
        (60., "#FFFF0080"),
        (120., "#00FF0080"),
        (180., "#00FFFF80"),
        (240., "#0000FF80"),
        (300., "#FF00FF80"),
        (360., "#FF000080"),
    ] {
        assert_eq!(
            Rgba::of_hsla(Hsla::new(hue, 1., 0.5, 0.5).unwrap()).to_hex(),
            expected
        );
    }
}

#[test]
fn hex_drafts_are_bounded_and_do_not_trim_or_accept_other_syntax() {
    assert_eq!(HexDraft::parse(""), HexDraft::Empty);
    assert_eq!(Rgba::of_hex(""), None);
    for text in ["#", "f", "12", "12345", "1234567"] {
        assert_eq!(HexDraft::parse(text), HexDraft::Incomplete);
        assert!(Rgba::of_hex(text).is_none());
    }
    for (text, canonical) in [
        ("#abc", "#AABBCC"),
        ("AbC0", "#AABBCC00"),
        ("#123456", "#123456"),
        ("11223344", "#11223344"),
        ("#FFFFFFFF", "#FFFFFF"),
    ] {
        let rgba = Rgba::of_hex(text).unwrap();
        assert_eq!(rgba.to_hex(), canonical);
        assert_eq!(HexDraft::parse(text), HexDraft::Valid(rgba));
    }
    for text in [" #123", "#123 ", "+123", "0x12", "##123", "ｆｆ", "\0abc"] {
        assert_eq!(HexDraft::parse(text), HexDraft::Invalid(DraftError::Syntax));
        assert!(Rgba::of_hex(text).is_none());
    }
    for text in [
        "123456789".to_owned(),
        "#123456789".to_owned(),
        "a".repeat(100000),
    ] {
        assert_eq!(
            HexDraft::parse(&text),
            HexDraft::Invalid(DraftError::TooLong)
        );
        assert!(Rgba::of_hex(&text).is_none());
    }
}
