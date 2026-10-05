//! Qualification-only component. The ordinary application backend never links it.
use gpuio_extension_sdk::{self as sdk, gpui, gpui::prelude::*};
use gpuio_native::performance::{Distribution, Interval, Snapshot};
use std::{cell::RefCell, fmt::Write, rc::Rc, sync::Arc, time::Instant};

pub const FINGERPRINT: &str = "0a6b9804fd18e486172696a2e8a5e460427affd2b73892b2dc8fff24791a52fe";
pub fn factory() -> Arc<dyn sdk::Factory> {
    Arc::new(Factory)
}
struct Factory;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Begin,
    Finish,
    Buckets { metric: usize, offset: usize },
}
impl Command {
    fn decode(bytes: &[u8]) -> Result<Self, sdk::Error> {
        match bytes {
            [0] => Ok(Self::Begin),
            [1] => Ok(Self::Finish),
            [2, metric @ 0..=4, digits @ ..]
                if !digits.is_empty()
                    && digits.len() <= 10
                    && digits.iter().all(u8::is_ascii_digit)
                    && (digits.len() == 1 || digits[0] != b'0') =>
            {
                let offset = std::str::from_utf8(digits)
                    .unwrap()
                    .parse::<u32>()
                    .map_err(|_| sdk::Error::InvalidCommand)?;
                Ok(Self::Buckets {
                    metric: usize::from(*metric),
                    offset: offset as usize,
                })
            }
            _ => Err(sdk::Error::InvalidCommand),
        }
    }
}

enum State {
    Idle,
    Settling,
    Running(Snapshot),
    Finished(Interval),
}
struct Probe {
    state: Rc<RefCell<State>>,
    pending: Option<gpui::Task<()>>,
}

impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: "qualification.performance",
            version: 1,
            fingerprint: FINGERPRINT,
            sdk_version: sdk::SDK_VERSION,
            gpui_revision: sdk::GPUI_REVISION,
            max_properties: 1,
            max_command: 12,
            max_event: sdk::MAX_MESSAGE,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        if bytes == [0] {
            Ok(())
        } else {
            Err(sdk::Error::InvalidProperties)
        }
    }
    fn validate_command(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        Command::decode(bytes).map(|_| ())
    }
    fn mount(
        &self,
        bytes: &[u8],
        _: &mut sdk::Context<'_>,
    ) -> Result<Box<dyn sdk::Component>, sdk::Error> {
        self.validate_properties(bytes)?;
        Ok(Box::new(Probe {
            state: Rc::new(RefCell::new(State::Idle)),
            pending: None,
        }))
    }
}

fn distributions(interval: &Interval) -> [&Distribution; 5] {
    [
        &interval.draw,
        &interval.dirty_to_submission,
        &interval.animation_submission_interval,
        &interval.input_to_frame,
        &interval.inputs_per_frame,
    ]
}

fn buckets(interval: &Interval, metric: usize, offset: usize) -> Result<Vec<u8>, sdk::Error> {
    let distribution = distributions(interval)
        .get(metric)
        .copied()
        .ok_or(sdk::Error::InvalidCommand)?;
    let total = distribution.buckets().count();
    if offset > total {
        return Err(sdk::Error::InvalidCommand);
    }
    let mut out = format!("(Buckets (metric {metric}) (offset {offset}) (total {total}) (values (");
    for (value, count) in distribution.buckets().skip(offset).take(128) {
        write!(out, "({value} {count})").unwrap();
    }
    out.push_str(")))");
    if out.len() > sdk::MAX_MESSAGE {
        return Err(sdk::Error::LimitExceeded);
    }
    Ok(out.into_bytes())
}

impl sdk::Component for Probe {
    fn update(&mut self, bytes: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        use sdk::Factory;
        Factory.validate_properties(bytes)
    }
    fn command(&mut self, bytes: &[u8], cx: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        match Command::decode(bytes)? {
            Command::Begin => {
                if matches!(*self.state.borrow(), State::Settling | State::Running(_)) {
                    return Err(sdk::Error::InvalidCommand);
                }
                *self.state.borrow_mut() = State::Settling;
                let state = self.state.clone();
                let events = cx.events.clone();
                let window = cx.window.window_handle();
                self.pending = Some(cx.app.spawn(async move |cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(2))
                        .await;
                    let _ = window.update(cx, |_, window, _| {
                        if let Err(error) = events.check() {
                            eprintln!("PERFORMANCE_PROBE_BEGIN_FAILED {error:?}");
                            return;
                        }
                        let started = Instant::now();
                        let snapshot = Snapshot::capture(window);
                        let capture_ns = started.elapsed().as_nanos();
                        *state.borrow_mut() = State::Running(snapshot);
                        // Delivery is asynchronous and does not request native redraw.
                        let _ = events.emit(format!("(Begun {capture_ns})").into_bytes());
                    });
                }));
                Ok(())
            }
            Command::Finish => {
                if !matches!(*self.state.borrow(), State::Running(_)) {
                    return Err(sdk::Error::InvalidCommand);
                }
                self.pending.take();
                let started = Instant::now();
                let after = Snapshot::capture(cx.window);
                let capture_ns = started.elapsed().as_nanos();
                let mut state = self.state.borrow_mut();
                let State::Running(before) = &*state else {
                    return Err(sdk::Error::InvalidCommand);
                };
                let interval = after
                    .since(before)
                    .map_err(|_| sdk::Error::InvalidCommand)?;
                let counts: Vec<_> = distributions(&interval)
                    .iter()
                    .map(|d| d.count().to_string())
                    .collect();
                let event = format!(
                    "(Finished (elapsed_ns {}) (capture_ns {capture_ns}) (dropped_inputs {}) (counts ({})))",
                    interval.elapsed.as_nanos(),
                    interval.mid_draw_inputs_dropped,
                    counts.join(" ")
                );
                cx.events.emit(event.into_bytes())?;
                *state = State::Finished(interval);
                Ok(())
            }
            Command::Buckets { metric, offset } => {
                let state = self.state.borrow();
                let State::Finished(interval) = &*state else {
                    return Err(sdk::Error::InvalidCommand);
                };
                cx.events.emit(buckets(interval, metric, offset)?)
            }
        }
    }
    fn render(&mut self, _: &mut sdk::Context<'_>) -> Result<gpui::AnyElement, sdk::Error> {
        Ok(gpui::div()
            .w(gpui::px(1.))
            .h(gpui::px(1.))
            .flex_shrink_0()
            .into_any_element())
    }
    fn unmount(&mut self) {
        self.pending.take();
        *self.state.borrow_mut() = State::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_decode_is_bounded_and_rejects_trailing_or_out_of_range_data() {
        assert_eq!(Command::decode(&[0]), Ok(Command::Begin));
        assert_eq!(Command::decode(&[1]), Ok(Command::Finish));
        assert_eq!(
            Command::decode(
                b"\x02\x04"
                    .iter()
                    .copied()
                    .chain(*b"4294967295")
                    .collect::<Vec<_>>()
                    .as_slice()
            ),
            Ok(Command::Buckets {
                metric: 4,
                offset: u32::MAX as usize
            })
        );
        for bytes in [
            b"".as_slice(),
            &[0, 0],
            &[1, 0],
            &[2, 5, b'0'],
            &[2, 0],
            b"\x02\x00-1",
            b"\x02\x004294967296",
            b"\x02\x001x",
        ] {
            assert_eq!(Command::decode(bytes), Err(sdk::Error::InvalidCommand));
        }
    }
}
