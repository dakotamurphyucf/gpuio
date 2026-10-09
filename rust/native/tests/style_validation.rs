//! Native admission independently enforces the public numeric style boundaries.
use gpuio_native::session::Session;
use gpuio_protocol::{NodeId, WindowId, v1::*};

struct Fixture {
    session: Session,
    window: WindowId,
    node: NodeId,
}
impl Fixture {
    fn new() -> Self {
        let mut session = Session::default();
        let window = WindowId::from_parts(0, 1).unwrap();
        let node = NodeId::from_parts(0, 1).unwrap();
        session.hello(VERSION, CAPABILITIES).unwrap();
        session.open(1, window, "validation", 400., 300.).unwrap();
        session
            .apply(&Transaction {
                window,
                base: 0,
                revision: 1,
                operations: vec![
                    Op::Create(node, Kind::Text, "original".into(), None),
                    Op::SetRoot(Some(node)),
                ],
            })
            .unwrap();
        Self {
            session,
            window,
            node,
        }
    }
    fn check(&mut self, field: Field, expected: Result<(), ErrorCode>) {
        self.check_fields(vec![field], expected);
    }
    fn check_fields(&mut self, fields: Vec<Field>, expected: Result<(), ErrorCode>) {
        let base = self.session.tree(self.window).unwrap().revision();
        let retained = self.session.retained_bytes();
        let before = self
            .session
            .tree(self.window)
            .unwrap()
            .get(self.node)
            .unwrap()
            .style
            .clone();
        let result = self
            .session
            .apply(&Transaction {
                window: self.window,
                base,
                revision: base + 1,
                operations: vec![
                    Op::SetText(
                        self.node,
                        if expected.is_ok() {
                            "original"
                        } else {
                            "must not publish"
                        }
                        .into(),
                    ),
                    Op::SetStyle(self.node, vec![Style::Fields(fields.clone())]),
                ],
            })
            .map(|_| ());
        assert_eq!(result, expected, "{fields:?}");
        let tree = self.session.tree(self.window).unwrap();
        assert_eq!(tree.get(self.node).unwrap().text.as_ref(), "original");
        if expected.is_err() {
            assert_eq!(tree.revision(), base);
            assert_eq!(self.session.retained_bytes(), retained);
            assert_eq!(tree.get(self.node).unwrap().style, before);
        }
    }
    fn close(mut self) {
        self.session.close(self.window).unwrap();
        assert_eq!(self.session.retained_bytes(), 0);
    }
}

#[test]
fn numeric_style_boundaries_are_atomic() {
    let mut f = Fixture::new();
    let nonnegative: [fn(f64) -> Field; 10] = [
        Field::Grow,
        Field::Shrink,
        Field::BorderTopWidth,
        Field::BorderRightWidth,
        Field::BorderBottomWidth,
        Field::BorderLeftWidth,
        Field::TopLeftRadius,
        Field::TopRightRadius,
        Field::BottomLeftRadius,
        Field::BottomRightRadius,
    ];
    for property in nonnegative {
        for value in [0., 1., 1_000_000.] {
            f.check(property(value), Ok(()));
        }
        for value in [-1., 1_000_001., f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            f.check(property(value), Err(ErrorCode::Malformed));
        }
    }
    for value in [0.001, 1_000_000.] {
        f.check(Field::FontSize(value), Ok(()));
    }
    for value in [0., -1., 1_000_001., f64::NAN, f64::INFINITY] {
        f.check(Field::FontSize(value), Err(ErrorCode::Malformed));
    }
    for value in [0., 1.] {
        f.check(Field::Opacity(value), Ok(()));
    }
    for value in [-0.001, 1.001, f64::NAN, f64::INFINITY] {
        f.check(Field::Opacity(value), Err(ErrorCode::Malformed));
    }
    for property in [Field::GridColumns, Field::GridRows, Field::LineClamp] {
        for value in [1, 1024] {
            f.check(property(value), Ok(()));
        }
        for value in [-1, 0, 1025] {
            f.check(property(value), Err(ErrorCode::Malformed));
        }
    }
    for value in [1, 1000] {
        f.check(Field::FontWeight(value), Ok(()));
    }
    for value in [-1, 0, 1001] {
        f.check(Field::FontWeight(value), Err(ErrorCode::Malformed));
    }
    f.close();
}

#[test]
fn length_sign_auto_and_magnitude_policies_are_atomic() {
    let mut f = Fixture::new();
    let dimensions: [fn(Length) -> Field; 7] = [
        Field::Width,
        Field::Height,
        Field::MinWidth,
        Field::MinHeight,
        Field::MaxWidth,
        Field::MaxHeight,
        Field::Basis,
    ];
    let definite: [fn(Length) -> Field; 7] = [
        Field::PaddingTop,
        Field::PaddingRight,
        Field::PaddingBottom,
        Field::PaddingLeft,
        Field::RowGap,
        Field::ColumnGap,
        Field::LineHeight,
    ];
    let signed: [fn(Length) -> Field; 8] = [
        Field::MarginTop,
        Field::MarginRight,
        Field::MarginBottom,
        Field::MarginLeft,
        Field::Top,
        Field::Right,
        Field::Bottom,
        Field::Left,
    ];
    for property in dimensions.into_iter().chain(definite).chain(signed) {
        for length in [Length::Px, Length::Percent] {
            for value in [0., 200., 1_000_000.] {
                f.check(property(length(value)), Ok(()));
            }
            for value in [
                -1_000_001.,
                1_000_001.,
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ] {
                f.check(property(length(value)), Err(ErrorCode::Malformed));
            }
        }
    }
    for property in dimensions.into_iter().chain(definite) {
        for length in [Length::Px, Length::Percent] {
            f.check(property(length(-1.)), Err(ErrorCode::Malformed));
        }
    }
    for property in signed {
        f.check(property(Length::Px(-1_000_000.)), Ok(()));
        f.check(property(Length::Percent(-10.)), Ok(()));
    }
    for property in dimensions.into_iter().chain(signed) {
        f.check(property(Length::Auto), Ok(()));
    }
    for property in definite {
        f.check(property(Length::Auto), Err(ErrorCode::Malformed));
    }
    f.close();
}

#[test]
fn text_byte_and_shadow_storage_limits_are_atomic() {
    let mut f = Fixture::new();
    f.check_fields(vec![Field::FontSize(16.); 128], Ok(()));
    f.check_fields(
        vec![Field::FontSize(16.); 129],
        Err(ErrorCode::LimitExceeded),
    );
    for count in [1, 256] {
        f.check(Field::FontFamily("a".repeat(count)), Ok(()));
    }
    for count in [0, 257] {
        f.check(
            Field::FontFamily("a".repeat(count)),
            Err(ErrorCode::Malformed),
        );
    }
    f.check(Field::FontFamily("é".repeat(128)), Ok(()));
    f.check(
        Field::FontFamily("é".repeat(129)),
        Err(ErrorCode::Malformed),
    );
    f.check(Field::AccessibleName("a".repeat(1024)), Ok(()));
    f.check(
        Field::AccessibleName("a".repeat(1025)),
        Err(ErrorCode::Malformed),
    );
    let shadow = Shadow {
        color: Color::Rgba(0),
        offset_x: -1_000_000.,
        offset_y: 0.,
        blur: 1_000_000.,
        spread: -1_000_000.,
        inset: true,
    };
    for count in [0, 8] {
        f.check(Field::Shadows(vec![shadow.clone(); count]), Ok(()));
    }
    f.check(
        Field::Shadows(vec![shadow.clone(); 9]),
        Err(ErrorCode::LimitExceeded),
    );
    f.check(
        Field::Shadows(vec![Shadow {
            blur: -1.,
            ..shadow.clone()
        }]),
        Err(ErrorCode::Malformed),
    );
    for value in [f64::NAN, f64::INFINITY, 1_000_001.] {
        for bad in [
            Shadow {
                offset_x: value,
                ..shadow.clone()
            },
            Shadow {
                offset_y: value,
                ..shadow.clone()
            },
            Shadow {
                blur: value,
                ..shadow.clone()
            },
            Shadow {
                spread: value,
                ..shadow.clone()
            },
        ] {
            f.check(Field::Shadows(vec![bad]), Err(ErrorCode::Malformed));
        }
    }
    f.close();
}
