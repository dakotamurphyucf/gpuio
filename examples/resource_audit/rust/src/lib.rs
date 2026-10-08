//! Separate qualification backend: no production hooks, OCaml callbacks or strong
//! window/entity ownership. App-owned audit survives component retirement solely
//! to inspect the settled closed window, then cancels its subscription at the end.
use gpuio_extension_sdk::{self as sdk, gpui, gpui::prelude::*};
mod metal;
mod presentation;

use std::{io::Write, sync::Arc, time::Duration};

pub const FINGERPRINT: &str = "245fc08e5aa950f642710e391572b120743a54eece59e0c09345cfeacda6cc83";
pub fn factory() -> Arc<dyn sdk::Factory> {
    Arc::new(Factory)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Properties {
    warmups: u8,
    measurements: u8,
    cycle: u8,
    metal_memory: bool,
    presentation: bool,
}
impl Properties {
    fn decode(bytes: &[u8]) -> Result<Self, sdk::Error> {
        match bytes {
            &[
                warmups @ 1..=3,
                measurements @ 1..=30,
                cycle,
                metal_memory @ 0..=1,
                presentation @ 0..=1,
            ] if cycle > 0 && cycle <= warmups + measurements => Ok(Self {
                warmups,
                measurements,
                cycle,
                metal_memory: metal_memory == 1,
                presentation: presentation == 1,
            }),
            _ => Err(sdk::Error::InvalidProperties),
        }
    }
    fn total(self) -> u8 {
        self.warmups + self.measurements
    }
}

struct Audit {
    config: Properties,
    window: Option<gpui::WindowId>,
    checked: u8,
    baseline: Option<gpui::LeakDetectorSnapshot>,
    subscription: Option<gpui::Subscription>,
    pending: Option<gpui::Task<()>>,
    failed: bool,
    metal: Option<metal::Probe>,
    presentation: Option<presentation::Probe>,
}
impl gpui::Global for Audit {}

fn record(value: std::fmt::Arguments<'_>) -> Result<(), sdk::Error> {
    let mut out = std::io::stdout().lock();
    writeln!(out, "GPUIO_ENTITY_AUDIT {value}").map_err(|_| sdk::Error::Closed)?;
    out.flush().map_err(|_| sdk::Error::Closed)
}

fn fail(app: &mut gpui::App, cycle: u8) {
    if app.has_global::<Audit>() {
        let state = app.global_mut::<Audit>();
        state.failed = true;
        state.presentation.take();
        state.subscription.take();
    }
    // Even if output fails, the collector rejects the missing checkpoint and
    // withholds its acknowledgement; never turn a failed audit into a pass.
    let _ = record(format_args!("failed {cycle}"));
}

fn check(app: &mut gpui::App, cycle: u8) -> Result<(), sdk::Error> {
    if !app.windows().is_empty() {
        return Err(sdk::Error::InvalidCommand);
    }
    app.update_global::<Audit, _>(|state, app| {
        sdk::contain(|| {
            if state.failed || state.window.is_some() || cycle != state.checked + 1 {
                return Err(sdk::Error::InvalidCommand);
            }
            let presentation = if state.config.presentation {
                Some(
                    state
                        .presentation
                        .take()
                        .ok_or(sdk::Error::InvalidCommand)?
                        .retire(cycle)?,
                )
            } else if state.presentation.is_some() {
                return Err(sdk::Error::InvalidCommand);
            } else {
                None
            };
            let phase = if cycle < state.config.warmups {
                "warmup"
            } else if cycle == state.config.warmups {
                state.baseline = Some(app.leak_detector_snapshot());
                "baseline"
            } else {
                let baseline = state.baseline.as_ref().ok_or(sdk::Error::InvalidCommand)?;
                app.assert_no_new_leaks(baseline);
                "checked"
            };
            record(format_args!("checkpoint ({cycle} {phase})"))?;
            if let Some(probe) = &state.metal {
                probe.checkpoint(cycle)?;
            }
            if let Some(record) = presentation {
                record.emit()?;
            }
            state.checked = cycle;
            if cycle == state.config.total() {
                state.subscription.take();
                record(format_args!("complete {cycle}"))?;
            }
            Ok(())
        })
    })
}

fn closed(app: &mut gpui::App, id: gpui::WindowId) {
    let cycle = app.global::<Audit>().checked + 1;
    let result = sdk::contain(|| {
        let state = app.global_mut::<Audit>();
        if state.failed || state.window != Some(id) {
            return Err(sdk::Error::InvalidCommand);
        }
        state.window = None;
        if let Some(probe) = &state.presentation {
            probe.stop();
        }
        // App stores only this task and IDs. AsyncApp holds a weak application
        // reference. No window, extension EventSink or Entity crosses the delay.
        let task = app.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            // Contain both the GPUI assertion and any callback bookkeeping panic.
            cx.update(|app| {
                if sdk::contain(|| check(app, cycle)).is_err() {
                    fail(app, cycle);
                }
            });
        });
        app.global_mut::<Audit>().pending = Some(task);
        Ok(())
    });
    if result.is_err() {
        fail(app, cycle);
    }
}

fn register(config: Properties, cx: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
    let device = config
        .metal_memory
        .then(|| metal::Probe::capture(cx.window))
        .transpose()?;
    if !cx.app.has_global::<Audit>() {
        if config.cycle != 1 {
            return Err(sdk::Error::InvalidProperties);
        }
        let subscription = cx.app.on_window_closed(closed);
        cx.app.set_global(Audit {
            config,
            window: None,
            checked: 0,
            baseline: None,
            subscription: Some(subscription),
            pending: None,
            failed: false,
            metal: None,
            presentation: None,
        });
    }
    let state = cx.app.global_mut::<Audit>();
    if state.failed
        || state.window.is_some()
        || state.config.warmups != config.warmups
        || state.config.measurements != config.measurements
        || state.config.metal_memory != config.metal_memory
        || state.config.presentation != config.presentation
        || state.presentation.is_some()
        || config.cycle != state.checked + 1
        || state.checked >= config.total()
    {
        return Err(sdk::Error::InvalidProperties);
    }
    if let Some(device) = device {
        if let Some(previous) = &state.metal {
            previous.require_same_device(&device)?;
        }
        state.metal = Some(device);
    }
    state.presentation = config
        .presentation
        .then(|| presentation::Probe::capture(cx.window))
        .transpose()?;
    state.pending.take(); // prior completed audit; never retain historical tasks
    state.window = Some(cx.window.window_handle().window_id());
    Ok(())
}

struct Factory;
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: "qualification.resource_audit",
            version: 3,
            fingerprint: FINGERPRINT,
            sdk_version: sdk::SDK_VERSION,
            gpui_revision: sdk::GPUI_REVISION,
            max_properties: 5,
            max_command: 1,
            max_event: 1,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        Properties::decode(bytes).map(|_| ())
    }
    fn validate_command(&self, _: &[u8]) -> Result<(), sdk::Error> {
        Err(sdk::Error::InvalidCommand)
    }
    fn mount(
        &self,
        bytes: &[u8],
        cx: &mut sdk::Context<'_>,
    ) -> Result<Box<dyn sdk::Component>, sdk::Error> {
        let properties = Properties::decode(bytes)?;
        register(properties, cx)?;
        Ok(Box::new(Component(properties)))
    }
}
struct Component(Properties);
impl sdk::Component for Component {
    fn update(&mut self, bytes: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        if Properties::decode(bytes)? == self.0 {
            Ok(())
        } else {
            Err(sdk::Error::InvalidProperties)
        }
    }
    fn command(&mut self, _: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        Err(sdk::Error::InvalidCommand)
    }
    fn render(&mut self, cx: &mut sdk::Context<'_>) -> Result<gpui::AnyElement, sdk::Error> {
        if let Some(probe) = &mut cx.app.global_mut::<Audit>().metal {
            probe.sample()?;
        }
        Ok(gpui::div().into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_or_trailing_configuration() {
        for bytes in [
            &[][..],
            &[0, 30, 1, 0, 0],
            &[3, 31, 1, 0, 0],
            &[3, 30, 0, 0, 0],
            &[3, 30, 34, 0, 0],
            &[3, 30, 1, 0],
            &[3, 30, 1, 2, 0],
            &[3, 30, 1, 0, 2],
            &[3, 30, 1, 0, 0, 0],
        ] {
            assert_eq!(
                Properties::decode(bytes),
                Err(sdk::Error::InvalidProperties)
            );
        }
        let p = Properties::decode(&[3, 30, 33, 1, 1]).unwrap();
        assert_eq!(p.total(), 33);
        assert!(p.metal_memory && p.presentation);
    }
    #[test]
    fn retained_entity_fails_and_release_passes_with_contained_panic() {
        let app = gpui::TestAppContext::single();
        app.update(|cx| {
            let baseline = cx.leak_detector_snapshot();
            cx.set_global(Audit {
                config: Properties {
                    warmups: 1,
                    measurements: 1,
                    cycle: 1,
                    metal_memory: false,
                    presentation: false,
                },
                window: None,
                checked: 1,
                baseline: Some(baseline),
                subscription: None,
                pending: None,
                failed: false,
                metal: None,
                presentation: None,
            });
        });
        let entity = app.update(|cx| cx.new(|_| 42usize));
        assert_eq!(app.update(|cx| check(cx, 2)), Err(sdk::Error::Panicked));
        // Containment must run inside the global lease, so the failed assertion
        // neither drops our baseline nor leaves App's global storage borrowed.
        app.update(|cx| assert_eq!(cx.global::<Audit>().checked, 1));
        drop(entity);
        app.run_until_parked();
        assert_eq!(app.update(|cx| check(cx, 2)), Ok(()));
        app.update(|cx| assert_eq!(cx.global::<Audit>().checked, 2));
    }
}
