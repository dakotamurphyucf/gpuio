//! Qualification-only collector ownership. No Window/Entity/native view is retained.
use gpuio_extension_sdk::{Error, gpui};

#[cfg(feature = "presentation-diagnostics")]
mod enabled {
    use super::*;
    use gpuio_native::performance::presentation::{Limits, Session, Snapshot};
    use serde_json::{Value, json};
    use std::io::Write;

    pub struct Probe(Session);
    pub struct Record(Value);
    impl Probe {
        pub fn capture(window: &gpui::Window) -> Result<Self, Error> {
            Session::start(window, Limits::default())
                .map(Self)
                .map_err(|_| Error::InvalidCommand)
        }
        pub fn stop(&self) {
            self.0.stop();
        }
        pub fn retire(self, cycle: u8) -> Result<Record, Error> {
            // Clone values only, then destroy the sole strong measurement owner
            // before exporting a checkpoint or sampling closed-window memory.
            let snapshot = self.0.snapshot();
            drop(self);
            let value = retirement_record(cycle, &snapshot)?;
            drop(snapshot);
            Ok(Record(value))
        }
    }
    impl Record {
        pub fn emit(self) -> Result<(), Error> {
            let mut out = std::io::stdout().lock();
            writeln!(out, "GPUIO_PRESENTATION_AUDIT {}", self.0).map_err(|_| Error::Closed)?;
            out.flush().map_err(|_| Error::Closed)
        }
    }

    fn retirement_record(cycle: u8, s: &Snapshot) -> Result<Value, Error> {
        let c = s.counts;
        let settled = [
            c.presented,
            c.zero,
            c.not_submitted,
            c.missing,
            c.invalid_clock,
        ]
        .into_iter()
        .try_fold(0u64, |sum, n| sum.checked_add(n))
        .ok_or(Error::InvalidCommand)?;
        if cycle == 0
            || cycle > 33
            || !s.window_closed
            || s.accepting
            || s.pending != 0
            || c.attempted == 0
            || c.attempted != c.admitted
            || c.admitted != settled
            || c.presented
                .checked_add(c.zero)
                .ok_or(Error::InvalidCommand)?
                == 0
            || c.saturated != 0
            || c.duplicate_callbacks != 0
            || c.histogram_overflow != 0
        {
            return Err(Error::InvalidCommand);
        }
        // Zero and other settled outcomes remain visible. This qualifies bounded
        // retirement only, never presentation timing or successful frame delivery.
        Ok(json!({"schema": 1, "cycle": cycle, "session": s.session,
            "window": s.window.as_u64(), "accepting": s.accepting,
            "window_closed": s.window_closed, "pending": s.pending,
            "attempted": c.attempted, "admitted": c.admitted,
            "presented": c.presented, "zero": c.zero, "not_submitted": c.not_submitted,
            "missing": c.missing, "invalid_clock": c.invalid_clock,
            "saturated": c.saturated, "duplicate_callbacks": c.duplicate_callbacks,
            "histogram_overflow": c.histogram_overflow, "trace_truncated": c.trace_truncated}))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use gpui::{Context, IntoElement, Render, TestAppContext, Window, div};
        struct Fixture;
        impl Render for Fixture {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
            }
        }

        #[test]
        fn stopped_owner_blocks_replacement_and_closed_idle_is_not_activity() {
            let mut app = TestAppContext::single();
            let (_, cx) = app.add_window_view(|_, _| Fixture);
            cx.run_until_parked();
            cx.update(|window, _| assert!(Probe::capture(window).is_err()));
            // Core Session is available on TestPlatform; the native wrapper must
            // reject that platform above rather than pretend it is Metal.
            let first =
                cx.update(|window, _| window.start_presentation(Limits::default()).unwrap());
            first.stop();
            cx.update(|window, _| assert!(window.start_presentation(Limits::default()).is_err()));
            let old_id = first.snapshot().session;
            assert!(retirement_record(1, &first.snapshot()).is_err());
            drop(first);
            let second =
                cx.update(|window, _| window.start_presentation(Limits::default()).unwrap());
            assert_ne!(second.snapshot().session, old_id);
            cx.update(|window, _| window.remove_window());
            cx.run_until_parked();
            let snapshot = second.snapshot();
            assert!(snapshot.window_closed && !snapshot.accepting);
            assert_eq!(snapshot.pending, 0);
            assert!(retirement_record(1, &snapshot).is_err());
        }

        #[test]
        fn retirement_rejects_pending_open_and_inconsistent_outcomes() {
            let mut app = TestAppContext::single();
            let (_, cx) = app.add_window_view(|_, _| Fixture);
            let session =
                cx.update(|window, _| window.start_presentation(Limits::default()).unwrap());
            cx.update(|window, _| window.remove_window());
            cx.run_until_parked();
            let mut closed = session.snapshot();
            // Explicit synthetic supported callback outcome for validator tests;
            // TestPlatform never supplies native presentation timestamps.
            closed.counts.attempted = 1;
            closed.counts.admitted = 1;
            closed.counts.zero = 1;
            let record = retirement_record(1, &closed).unwrap();
            assert_eq!(record["zero"], 1);
            assert_eq!(record["presented"], 0);
            for edit in [
                |s: &mut Snapshot| s.pending = 1,
                |s: &mut Snapshot| s.window_closed = false,
                |s: &mut Snapshot| s.accepting = true,
                |s: &mut Snapshot| s.counts.admitted = 2,
                |s: &mut Snapshot| s.counts.zero = 0,
                |s: &mut Snapshot| s.counts.saturated = 1,
                |s: &mut Snapshot| s.counts.duplicate_callbacks = 1,
            ] {
                let mut invalid = closed.clone();
                edit(&mut invalid);
                assert!(retirement_record(1, &invalid).is_err());
            }
        }
    }
}

#[cfg(not(feature = "presentation-diagnostics"))]
mod enabled {
    use super::*;
    pub struct Probe;
    pub struct Record;
    impl Probe {
        pub fn capture(_: &gpui::Window) -> Result<Self, Error> {
            Err(Error::InvalidCommand)
        }
        pub fn stop(&self) {}
        pub fn retire(self, _: u8) -> Result<Record, Error> {
            Err(Error::InvalidCommand)
        }
    }
}

#[cfg(not(feature = "presentation-diagnostics"))]
impl enabled::Record {
    pub fn emit(self) -> Result<(), Error> {
        Err(Error::InvalidCommand)
    }
}
pub use enabled::Probe;
