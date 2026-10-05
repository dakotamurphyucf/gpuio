//! Extra qualification output, emitted only after admission stops and callbacks
//! settle. This driver never performs I/O from a renderer callback.
use gpuio_extension_sdk::gpui;
use gpuio_native::performance::{
    Interval, WindowObservation,
    presentation::{Limits, Session, Snapshot, StartError},
};
use serde_json::{Value, json};
use std::time::Instant;

pub struct Active {
    session: Result<Session, StartError>,
    pub start_capture_ns: u128,
}
impl Active {
    pub fn start(window: &gpui::Window) -> Self {
        let started = Instant::now();
        let session = Session::start(window, Limits::default());
        Self {
            session,
            start_capture_ns: started.elapsed().as_nanos(),
        }
    }
    pub fn stop(&self) {
        if let Ok(session) = &self.session {
            session.stop();
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
            "idle_observation_mode": idle, "observations_truncated": observations.len() >= if idle {128} else {2048},
            "observations": observations.iter().map(|(elapsed, value)| json!({"elapsed_ns": elapsed,
                "active": value.active, "visible": value.visible})).collect::<Vec<_>>(),
            "end_observation": {"active": end.active, "visible": end.visible}, "native": native})
    }
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
