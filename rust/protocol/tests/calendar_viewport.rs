use binprot::BinProtWrite;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    calendar_viewport::{Display, Observation},
    decode,
    v1::*,
};
fn hex<T: BinProtWrite>(value: &T) -> String {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn viewport_subscription_and_observation_match_ocaml_fixtures() {
    let w = WindowId::from_parts(0, 1).unwrap();
    let n = NodeId::from_parts(1, 2).unwrap();
    let h = HandlerId::from_parts(3, 4).unwrap();
    let m = Message::Apply(Transaction {
        window: w,
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetCalendarViewportObserver(n, Some(h)),
            Op::SetCalendarViewportObserver(n, None),
        ],
    });
    assert_eq!(
        hex(&m),
        include_str!("../../../test/fixtures/calendar-viewport-operation.hex").trim()
    );
    let mut bytes = vec![];
    m.binprot_write(&mut bytes).unwrap();
    assert_eq!(decode(&bytes), Ok(m));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
    let events = [
        Display::Days {
            first_month: 24277,
            months: 2,
            first_weekday: 1,
        },
        Display::Months { year: 1 },
        Display::Years {
            first: 9981,
            last: 9999,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(sequence, display)| {
        Event::CalendarViewportChanged(
            w,
            n,
            h,
            5,
            Observation {
                sequence: sequence as i64,
                display,
            },
        )
    })
    .collect::<Vec<_>>();
    assert_eq!(
        hex(&events),
        include_str!("../../../test/fixtures/calendar-viewport-events.hex").trim()
    );
}
#[test]
fn viewport_validity_covers_civil_bounds_and_aligned_year_pages() {
    for first_month in [0, 119_976] {
        assert!(
            Display::Days {
                first_month,
                months: 12,
                first_weekday: 6
            }
            .is_valid()
        );
    }
    for display in [
        Display::Days {
            first_month: 119_987,
            months: 2,
            first_weekday: 0,
        },
        Display::Days {
            first_month: -1,
            months: 1,
            first_weekday: 0,
        },
        Display::Days {
            first_month: 0,
            months: i64::MAX,
            first_weekday: 0,
        },
        Display::Days {
            first_month: 0,
            months: 1,
            first_weekday: 7,
        },
        Display::Months { year: 0 },
        Display::Years {
            first: 2020,
            last: 2039,
        },
        Display::Years {
            first: 2021,
            last: 2039,
        },
        Display::Years {
            first: i64::MAX,
            last: i64::MAX,
        },
    ] {
        assert!(!display.is_valid());
    }
    assert!(
        !Observation {
            sequence: -1,
            display: Display::Months { year: 1 }
        }
        .is_valid()
    );
}
