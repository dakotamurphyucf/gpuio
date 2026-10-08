//! Extra qualification output, emitted only after admission stops and callbacks
//! settle. This driver never performs I/O from a renderer callback.
use gpuio_extension_sdk::gpui;
use gpuio_native::performance::{
    Interval, WindowObservation,
    presentation::{Limits, Session, Snapshot, StartError},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Instant};

use gpui::profiler::journal::{
    ForegroundEvent, ForegroundJournalCollector, ForegroundJournalEntry,
};

pub struct Active {
    session: Result<Session, StartError>,
    pub start_capture_ns: u128,
    input_journal: Option<ForegroundJournalCollector>,
    input_summary: Option<Value>,
}
impl Active {
    pub fn start(window: &gpui::Window, app: &gpui::App) -> Self {
        let started = Instant::now();
        let session = Session::start(window, Limits::default());
        Self {
            session,
            start_capture_ns: started.elapsed().as_nanos(),
            input_journal: std::env::var_os("GPUIO_DIAGNOSE_FOREGROUND_INPUT")
                .is_some()
                .then(|| app.foreground_journal().collector()),
            input_summary: None,
        }
    }
    pub fn stop(&mut self) {
        if let Ok(session) = &self.session {
            session.stop();
        }
        // No extra sampling task: drain only at cutoff, before callback settling.
        // The journal is foreground-thread-wide, not specific to this window.
        if let Some(mut collector) = self.input_journal.take() {
            let drained = collector.collect_unseen();
            self.input_summary = Some(input_summary(
                drained.lost,
                drained.entries.iter().filter_map(|entry| match entry {
                    ForegroundJournalEntry::Event(ForegroundEvent::Input(input)) => {
                        Some((input.kind, input.caused_invalidation))
                    }
                    _ => None,
                }),
            ));
        }
    }
    pub fn pending(&self) -> usize {
        self.session.as_ref().map(Session::pending).unwrap_or(0)
    }
    pub fn report(
        &self,
        cpu: &Interval,
        observations: &[(u128, WindowObservation)],
        end: WindowObservation,
        stop_capture_ns: u128,
        settlement_ns: u128,
        idle: bool,
    ) -> Value {
        let started = Instant::now();
        let native = match &self.session {
            Ok(session) => snapshot(&session.snapshot()),
            Err(error) => json!({"supported": false, "error": format!("{error:?}")}),
        };
        json!({"schema": 1, "kind": "gpui_presentation_interval", "cpu_elapsed_ns": cpu.elapsed.as_nanos(),
            "cpu_input_samples": cpu.input_to_frame.count(), "cpu_draw_samples": cpu.draw.count(),
            "start_capture_ns": self.start_capture_ns, "stop_capture_ns": stop_capture_ns,
            "settlement_ns": settlement_ns, "snapshot_encode_ns": started.elapsed().as_nanos(),
            "idle_observation_mode": idle, "diagnostic_foreground_inputs": self.input_summary,
            "observations_truncated": observations.len() >= if idle {128} else {2048},
            "observations": observations.iter().map(|(elapsed, value)| json!({"elapsed_ns": elapsed,
                "active": value.active, "visible": value.visible})).collect::<Vec<_>>(),
            "end_observation": {"active": end.active, "visible": end.visible}, "native": native})
    }
}

fn input_summary<'a>(lost: u64, inputs: impl Iterator<Item = (&'a str, bool)>) -> Value {
    let mut kinds = BTreeMap::<&str, (u64, u64)>::new();
    let (mut total, mut invalidating, mut other) = (0_u64, 0_u64, 0_u64);
    for (kind, invalidates) in inputs {
        total += 1;
        invalidating += u64::from(invalidates);
        if !kinds.contains_key(kind) && kinds.len() >= 32 {
            other += 1;
            continue;
        }
        let counts = kinds.entry(kind).or_default();
        counts.0 += 1;
        counts.1 += u64::from(invalidates);
    }
    json!({"scope": "foreground thread; not window-specific", "lost_entries": lost,
        "input_events": total, "invalidating_inputs": invalidating,
        "other_kind_events": other,
        "kinds": kinds.into_iter().map(|(kind, (events, invalidating))|
            json!({"kind": kind, "events": events, "invalidating": invalidating})).collect::<Vec<_>>(),
        "qualification": "diagnostic only; excluded from release acceptance"})
}

fn snapshot(s: &Snapshot) -> Value {
    let c = s.counts;
    let histogram = |h: &gpui::presentation::Snapshot, metric| {
        let h = match metric {
            0 => &h.submission_latency,
            1 => &h.input_latency,
            _ => &h.animation_interval,
        };
        json!({"count": h.len(), "buckets": h.iter_recorded().map(|b|
            [b.value_iterated_to(), b.count_since_last_iteration()]).collect::<Vec<_>>()})
    };
    json!({"supported": true, "session": s.session, "window": format!("{:?}", s.window),
        "accepting": s.accepting, "window_closed": s.window_closed, "pending": s.pending,
        "counts": {"attempted": c.attempted, "admitted": c.admitted, "saturated": c.saturated,
            "presented": c.presented, "not_submitted": c.not_submitted, "missing": c.missing,
            "zero": c.zero, "invalid_clock": c.invalid_clock, "duplicate_callbacks": c.duplicate_callbacks,
            "trace_truncated": c.trace_truncated, "histogram_overflow": c.histogram_overflow},
        "histograms": {"submission_to_presentation": histogram(s, 0), "input_to_presentation": histogram(s, 1),
            "scheduled_presentation_interval": histogram(s, 2)},
        "trace": s.trace.iter().map(record).collect::<Vec<_>>()})
}

fn record(r: &gpuio_native::performance::presentation::Record) -> Value {
    json!({"sequence": r.sequence, "drawable": r.drawable, "new_scene": r.new_scene,
        "active": r.active, "animating": r.animating, "inputs": r.inputs, "outcome": format!("{:?}", r.outcome),
        "submit_host_s": r.submit_host_s, "presented_host_s": r.presented_host_s, "callback_host_s": r.callback_host_s,
        "submission_lower_ns": r.submission_latency.map(|v| v.lower_ns),
        "submission_upper_ns": r.submission_latency.map(|v| v.upper_ns),
        "input_lower_ns": r.input_latency.map(|v| v.lower_ns),
        "input_upper_ns": r.input_latency.map(|v| v.upper_ns)})
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_native::performance::presentation::{LatencyBounds, Outcome, Record};
    #[test]
    fn input_summary_preserves_loss_and_counts_without_input_payloads() {
        let value = input_summary(
            7,
            [("MouseMove", true), ("MouseMove", false), ("KeyDown", true)].into_iter(),
        );
        assert_eq!(value["lost_entries"], 7);
        assert_eq!(value["input_events"], 3);
        assert_eq!(value["invalidating_inputs"], 2);
        assert_eq!(
            value["kinds"],
            json!([
                {"kind": "KeyDown", "events": 1, "invalidating": 1},
                {"kind": "MouseMove", "events": 2, "invalidating": 1}
            ])
        );
        let empty = input_summary(9, std::iter::empty());
        assert_eq!(empty["lost_entries"], 9);
        assert_eq!(empty["input_events"], 0);
    }
    #[test]
    fn input_kind_summary_bounds_distinct_names_without_losing_totals() {
        let names: Vec<_> = (0..33).map(|n| n.to_string()).collect();
        let value = input_summary(
            0,
            names
                .iter()
                .map(|name| (name.as_str(), true))
                .chain(std::iter::once(("0", true))),
        );
        assert_eq!(value["kinds"].as_array().unwrap().len(), 32);
        assert_eq!(value["other_kind_events"], 1);
        assert_eq!(value["input_events"], 34);
        assert_eq!(value["invalidating_inputs"], 34);
        assert_eq!(value["kinds"][0]["events"], 2);
    }
    #[test]
    fn records_preserve_absence_zero_and_clock_bounds() {
        let mut r = Record {
            sequence: 0,
            drawable: Some(0),
            new_scene: true,
            active: true,
            animating: false,
            inputs: 2,
            outcome: Outcome::Presented,
            submit_host_s: Some(10.),
            presented_host_s: Some(10.01),
            callback_host_s: Some(10.02),
            submission_latency: Some(LatencyBounds {
                lower_ns: 9_999_999,
                upper_ns: 10_000_001,
            }),
            input_latency: Some(LatencyBounds {
                lower_ns: 19_999_999,
                upper_ns: 20_000_002,
            }),
        };
        let data = record(&r);
        assert_eq!(data["drawable"], 0);
        assert_eq!(data["input_lower_ns"], 19_999_999);
        assert_eq!(data["input_upper_ns"], 20_000_002);
        r.outcome = Outcome::Zero;
        r.presented_host_s = Some(0.);
        r.input_latency = None;
        let data = record(&r);
        assert_eq!(data["outcome"], "Zero");
        assert_eq!(data["presented_host_s"], 0.);
        assert!(data["input_lower_ns"].is_null());
    }
}
