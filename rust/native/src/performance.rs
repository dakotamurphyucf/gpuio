//! Opt-in native qualification measurements. No timers, file I/O, trace-ring
//! activation or view invalidation. Capture on the GPUI thread at workload
//! boundaries; keep OS input/presentation and OCaml scheduling evidence separate.
use std::{collections::BTreeMap, time::Instant};

/// Read-only native window observations. `visible` is unavailable on backends
/// without a qualified OS occlusion query. No timer, event or redraw is created.
#[derive(Clone, Copy, Debug)]
pub struct WindowObservation {
    pub active: bool,
    pub visible: Option<bool>,
}

impl WindowObservation {
    pub fn capture(window: &gpui::Window) -> Self {
        Self {
            active: window.is_window_active(),
            visible: window_visibility(window),
        }
    }
}

#[cfg(target_os = "macos")]
fn window_visibility(window: &gpui::Window) -> Option<bool> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    objc2::MainThreadMarker::new()?;
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: the borrowed GPUI window owns this live NSView; the main-thread
    // marker is checked above. The temporary NSWindow reference stays in scope.
    let view = unsafe { handle.ns_view.cast::<objc2_app_kit::NSView>().as_ref() };
    let native = view.window()?;
    Some(
        native
            .occlusionState()
            .contains(objc2_app_kit::NSWindowOcclusionState::Visible),
    )
}

#[cfg(not(target_os = "macos"))]
fn window_visibility(_: &gpui::Window) -> Option<bool> {
    None
}

/// Application-wide cumulative document-worker counters. Durations are elapsed
/// worker stage times, not CPU time or physical presentation. Completed jobs
/// include cancelled/stale work; retained-source fallback can report zero parse
/// time. Reading these counters does not schedule jobs or request a redraw.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocumentPreparation {
    pub queue_us: u128,
    pub configure_us: u128,
    pub parse_us: u128,
    pub highlight_us: u128,
    pub search_us: u128,
    pub source_bytes: usize,
    pub completed: usize,
    pub discarded: usize,
    pub peak_workers: usize,
    pub peak_reserved_bytes: usize,
}

impl DocumentPreparation {
    pub fn capture(cx: &gpui::App) -> Self {
        let Some((m, completed, discarded, peak_workers, peak_reserved_bytes)) =
            crate::document_host::measurements(cx)
        else {
            return Self::default();
        };
        Self {
            queue_us: m.queue_us,
            configure_us: m.configure_us,
            parse_us: m.parse_us,
            highlight_us: m.highlight_us,
            search_us: m.search_us,
            source_bytes: m.source_bytes,
            completed,
            discarded,
            peak_workers,
            peak_reserved_bytes,
        }
    }
}

/// Sparse cumulative histogram buckets. Values are the inclusive upper bounds
/// reported by GPUI's three-significant-digit HDR histograms, not exact samples.
/// Durations use nanoseconds; `inputs_per_frame` uses event counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Distribution {
    buckets: BTreeMap<u64, u64>,
}

impl Distribution {
    /// Nonempty buckets in ascending value order: (inclusive upper bound, count).
    /// Retain these to combine intervals; never average/subtract percentiles.
    pub fn buckets(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.buckets.iter().map(|(&value, &count)| (value, count))
    }

    pub fn count(&self) -> u64 {
        self.buckets.values().sum()
    }

    /// Nearest-rank percentile in 1..=100. Empty distributions have no percentile.
    pub fn percentile(&self, percentile: u8) -> Option<u64> {
        if !(1..=100).contains(&percentile) {
            return None;
        }
        let count = self.count();
        if count == 0 {
            return None;
        }
        let rank = (u128::from(count) * u128::from(percentile)).div_ceil(100);
        let mut seen = 0_u128;
        self.buckets().find_map(|(value, count)| {
            seen += u128::from(count);
            (seen >= rank).then_some(value)
        })
    }

    fn since(&self, earlier: &Self) -> Result<Self, Error> {
        let mut buckets = self.buckets.clone();
        for (&value, &count) in &earlier.buckets {
            let current = buckets.get(&value).copied().unwrap_or(0);
            let remaining = current.checked_sub(count).ok_or(Error::CountersRegressed)?;
            if remaining == 0 {
                buckets.remove(&value);
            } else {
                buckets.insert(value, remaining);
            }
        }
        Ok(Self { buckets })
    }
}

#[derive(Clone, Debug)]
struct Histograms {
    draw: Distribution,
    dirty_to_submission: Distribution,
    animation_submission_interval: Distribution,
    input_to_frame: Distribution,
    inputs_per_frame: Distribution,
    mid_draw_inputs_dropped: u64,
}

/// Owned, cumulative snapshot for one native window. Does not retain the window.
/// Captures clone upstream histograms; time that overhead separately from frames.
#[derive(Clone, Debug)]
pub struct Snapshot {
    window: gpui::WindowId,
    captured: Instant,
    histograms: Histograms,
}

/// Invalid interval boundaries must fail, never saturate to apparent zero work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    DifferentWindow,
    ReversedTime,
    CountersRegressed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DifferentWindow => "performance snapshots belong to different windows",
            Self::ReversedTime => "performance snapshots are in reversed time order",
            Self::CountersRegressed => "performance counters regressed between snapshots",
        })
    }
}

impl std::error::Error for Error {}

/// Counts/durations for the half-open interval after the earlier snapshot.
/// Native submission is not GPU completion or physical presentation. Input
/// coalescing/drop counts must accompany any latency claim.
#[derive(Clone, Debug)]
pub struct Interval {
    pub elapsed: std::time::Duration,
    pub draw: Distribution,
    pub dirty_to_submission: Distribution,
    pub animation_submission_interval: Distribution,
    pub input_to_frame: Distribution,
    pub inputs_per_frame: Distribution,
    pub mid_draw_inputs_dropped: u64,
}

impl Snapshot {
    pub fn capture(window: &gpui::Window) -> Self {
        let frame = window.frame_duration_snapshot();
        let input = window.input_latency_snapshot();
        // All snapshots come from the same pinned HDR representation. Keeping
        // sparse bucket bounds avoids exposing that dependency in this API.
        macro_rules! distribution {
            ($histogram:expr) => {
                Distribution {
                    buckets: $histogram
                        .iter_recorded()
                        .map(|bucket| {
                            (
                                bucket.value_iterated_to(),
                                bucket.count_since_last_iteration(),
                            )
                        })
                        .collect(),
                }
            };
        }
        Self {
            window: window.window_handle().window_id(),
            captured: Instant::now(),
            histograms: Histograms {
                draw: distribution!(frame.draw_duration_histogram),
                dirty_to_submission: distribution!(frame.dirty_to_present_histogram),
                animation_submission_interval: distribution!(frame.present_interval_histogram),
                input_to_frame: distribution!(input.latency_histogram),
                inputs_per_frame: distribution!(input.events_per_frame_histogram),
                mid_draw_inputs_dropped: input.mid_draw_events_dropped,
            },
        }
    }

    /// Subtract bucket counts, preserving zero samples and input-loss evidence.
    /// Windows/clock order and every previous bucket must agree. Capture before
    /// and after each phase; startup/idle/occlusion are distinct workloads.
    pub fn since(&self, earlier: &Self) -> Result<Interval, Error> {
        if self.window != earlier.window {
            return Err(Error::DifferentWindow);
        }
        let elapsed = self
            .captured
            .checked_duration_since(earlier.captured)
            .ok_or(Error::ReversedTime)?;
        let now = &self.histograms;
        let old = &earlier.histograms;
        Ok(Interval {
            elapsed,
            draw: now.draw.since(&old.draw)?,
            dirty_to_submission: now.dirty_to_submission.since(&old.dirty_to_submission)?,
            animation_submission_interval: now
                .animation_submission_interval
                .since(&old.animation_submission_interval)?,
            input_to_frame: now.input_to_frame.since(&old.input_to_frame)?,
            inputs_per_frame: now.inputs_per_frame.since(&old.inputs_per_frame)?,
            mid_draw_inputs_dropped: now
                .mid_draw_inputs_dropped
                .checked_sub(old.mid_draw_inputs_dropped)
                .ok_or(Error::CountersRegressed)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn distribution(buckets: &[(u64, u64)]) -> Distribution {
        Distribution {
            buckets: buckets.iter().copied().collect(),
        }
    }

    fn snapshot(draw: Distribution) -> Snapshot {
        Snapshot {
            window: gpui::WindowId::from(1),
            captured: Instant::now(),
            histograms: Histograms {
                draw,
                dirty_to_submission: distribution(&[]),
                animation_submission_interval: distribution(&[]),
                input_to_frame: distribution(&[]),
                inputs_per_frame: distribution(&[]),
                mid_draw_inputs_dropped: 0,
            },
        }
    }

    #[test]
    fn interval_subtracts_counts_not_percentiles_and_preserves_idle_zero() {
        let old = snapshot(distribution(&[(10, 95), (100, 5)]));
        let mut now = old.clone();
        now.captured += Duration::from_secs(2);
        now.histograms.draw = distribution(&[(10, 95), (100, 10)]);
        let interval = now.since(&old).unwrap();
        assert_eq!(interval.elapsed, Duration::from_secs(2));
        assert_eq!(interval.draw.buckets().collect::<Vec<_>>(), [(100, 5)]);
        assert_eq!(interval.draw.percentile(95), Some(100));
        let idle = now.since(&now).unwrap();
        assert_eq!(idle.draw.count(), 0);
        assert_eq!(idle.draw.percentile(95), None);
    }

    #[test]
    fn rejects_other_windows_time_reversal_and_hidden_bucket_loss() {
        let old = snapshot(distribution(&[(10, 95), (100, 5)]));
        let mut now = old.clone();
        now.window = gpui::WindowId::from(2);
        assert_eq!(now.since(&old).unwrap_err(), Error::DifferentWindow);
        now = old.clone();
        now.captured -= Duration::from_secs(1);
        assert_eq!(now.since(&old).unwrap_err(), Error::ReversedTime);
        now = old.clone();
        // A higher total cannot hide a reset/missing bucket.
        now.histograms.draw = distribution(&[(10, 200)]);
        assert_eq!(now.since(&old).unwrap_err(), Error::CountersRegressed);
    }

    #[test]
    fn preserves_coalescing_and_dropped_input_counts() {
        let mut old = snapshot(distribution(&[]));
        old.histograms.mid_draw_inputs_dropped = 3;
        let mut now = old.clone();
        now.histograms.mid_draw_inputs_dropped = 7;
        now.histograms.inputs_per_frame = distribution(&[(2, 10), (5, 1)]);
        now.histograms.input_to_frame = distribution(&[(50_000_000, 11)]);
        let interval = now.since(&old).unwrap();
        assert_eq!(interval.mid_draw_inputs_dropped, 4);
        assert_eq!(interval.inputs_per_frame.count(), 11);
        assert_eq!(interval.input_to_frame.percentile(99), Some(50_000_000));
        now.histograms.mid_draw_inputs_dropped = 2;
        assert_eq!(now.since(&old).unwrap_err(), Error::CountersRegressed);
    }

    #[test]
    fn percentile_uses_nearest_rank_and_checks_domain() {
        let samples = distribution(&[(1, 1), (2, 1), (3, 1)]);
        assert_eq!(samples.percentile(1), Some(1));
        assert_eq!(samples.percentile(50), Some(2));
        assert_eq!(samples.percentile(100), Some(3));
        assert_eq!(samples.percentile(0), None);
        assert_eq!(samples.percentile(101), None);
    }

    #[cfg(feature = "native-image-tests")]
    #[test]
    fn actual_window_capture_is_read_only_and_records_a_draw_without_global_tracing() {
        use gpui::{Context, IntoElement, Render, TestAppContext, Window, div};
        struct Fixture;
        impl Render for Fixture {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
            }
        }
        let mut app = TestAppContext::single();
        let (_, cx) = app.add_window_view(|_, _| Fixture);
        cx.run_until_parked();
        cx.update(|window, cx| {
            let before = Snapshot::capture(window);
            let observation = WindowObservation::capture(window);
            assert_eq!(observation.active, window.is_window_active());
            // A TestPlatform window has no OS occlusion query. Unknown must not
            // masquerade as a visible native window in qualification reports.
            assert_eq!(observation.visible, None);
            let unchanged = Snapshot::capture(window);
            assert_eq!(unchanged.since(&before).unwrap().draw.count(), 0);
            window.draw(cx).clear(cx);
            let after = Snapshot::capture(window);
            let measured = after.since(&before).unwrap();
            assert_eq!(measured.draw.count(), 1);
            assert!(measured.draw.percentile(95).is_some());
            assert_eq!(measured.input_to_frame.count(), 0);
            assert_eq!(measured.mid_draw_inputs_dropped, 0);
            // Owned snapshots contain measurements, not native window handles.
            window.remove_window();
            assert_eq!(after.since(&before).unwrap().draw.count(), 1);
        });
        cx.run_until_parked();
        assert!(app.read(|app| app.windows().is_empty()));
    }
}
