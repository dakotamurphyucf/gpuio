//! Independently consumable component: only the public extension SDK is imported.
use gpuio_extension_sdk::{
    self as sdk,
    gpui::{self, prelude::*},
};
use sdk::Factory as _;
use std::{cell::Cell, rc::Rc, sync::Arc};

// Schema: properties=(value:u8,step:u8), command=(value:u8), event=(value:u8).
// All values 0..100, step 1..10. Fixed bytes, no nested allocation.
pub const FINGERPRINT: &str = "ce6beefc974d5b1f01875857c46f2ce31cb2fcfc8e8c3ef3a21716b967f96e15";
pub fn factory() -> Arc<dyn sdk::Factory> {
    Arc::new(Factory)
}
struct Factory;
struct Counter {
    value: Rc<Cell<u8>>,
    step: u8,
}
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: "example.counter",
            version: 1,
            fingerprint: FINGERPRINT,
            sdk_version: sdk::SDK_VERSION,
            gpui_revision: sdk::GPUI_REVISION,
            max_properties: 2,
            max_command: 1,
            max_event: 1,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        match bytes {
            [0..=100, 1..=10] => Ok(()),
            _ => Err(sdk::Error::InvalidProperties),
        }
    }
    fn validate_command(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        match bytes {
            [0..=100] => Ok(()),
            _ => Err(sdk::Error::InvalidCommand),
        }
    }
    fn mount(
        &self,
        bytes: &[u8],
        _: &mut sdk::Context<'_>,
    ) -> Result<Box<dyn sdk::Component>, sdk::Error> {
        self.validate_properties(bytes)?;
        Ok(Box::new(Counter {
            value: Rc::new(Cell::new(bytes[0])),
            step: bytes[1],
        }))
    }
}
impl sdk::Component for Counter {
    fn update(&mut self, bytes: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        Factory.validate_properties(bytes)?;
        self.value.set(bytes[0]);
        self.step = bytes[1];
        Ok(())
    }
    fn command(&mut self, bytes: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        Factory.validate_command(bytes)?;
        self.value.set(bytes[0]);
        Ok(())
    }
    fn render(&mut self, cx: &mut sdk::Context<'_>) -> Result<gpui::AnyElement, sdk::Error> {
        let value = self.value.clone();
        let step = self.step;
        let events = cx.events.clone();
        let focus = cx.focus.clone();
        let activate = Rc::new(
            move |pointer: bool, window: &mut gpui::Window, app: &mut gpui::App| {
                let action = || {
                    let next = value.get().saturating_add(step).min(100);
                    // Admission precedes local mutation so overload does not hide a change.
                    events.emit(vec![next])?;
                    value.set(next);
                    window.focus(&focus, app);
                    window.refresh();
                    Ok(())
                };
                let _ = if pointer {
                    events.guard_pointer(action)
                } else {
                    events.guard(action)
                };
            },
        );
        let click = activate.clone();
        let keyboard = activate.clone();
        Ok(gpui::div()
            .id("example-counter")
            .track_focus(&cx.focus)
            .role(gpui::Role::Button)
            .aria_label(format!(
                "Increment counter, current value {}",
                self.value.get()
            ))
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px_6()
            .py_4()
            .rounded_xl()
            .bg(gpui::rgb(0x263f38))
            .text_color(gpui::rgb(0xe8f3ed))
            .cursor_pointer()
            .child(gpui::div().child("NATIVE COUNTER"))
            .child(
                gpui::div()
                    .text_xl()
                    .child(format!("{}  +", self.value.get())),
            )
            .on_click(move |_, window, app| click(true, window, app))
            .on_key_down(move |event, window, app| {
                if !event.keystroke.modifiers.modified()
                    && matches!(event.keystroke.key.as_str(), "space" | "enter")
                {
                    keyboard(false, window, app);
                    app.stop_propagation();
                }
            })
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, app| {
                activate(false, window, app)
            })
            .into_any_element())
    }
}
