//! Read-only device accounting for the qualification backend. No view/layer,
//! window, entity, event route or timer is retained here.
use gpuio_extension_sdk::{Error, gpui};

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use objc2::{
        MainThreadMarker, msg_send,
        rc::Retained,
        runtime::{AnyClass, AnyObject},
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::io::Write;

    pub struct Probe {
        device: Retained<AnyObject>,
        registry_id: u64,
        samples: u64,
        sampled_max: usize,
    }

    impl Probe {
        pub fn capture(window: &gpui::Window) -> Result<Self, Error> {
            MainThreadMarker::new().ok_or(Error::InvalidCommand)?;
            let handle =
                HasWindowHandle::window_handle(window).map_err(|_| Error::InvalidCommand)?;
            let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
                return Err(Error::InvalidCommand);
            };
            let class = AnyClass::get(c"CAMetalLayer").ok_or(Error::InvalidCommand)?;
            // SAFETY: GPUI owns the borrowed live NSView and this runs on the
            // main thread. Retained temporaries balance Objective-C ownership;
            // only the device survives the call. Verify the layer's class before
            // sending Metal-specific selectors. CAMetalLayer.device is MTLDevice.
            unsafe {
                let view = handle.ns_view.cast::<AnyObject>().as_ref();
                let layer: Option<Retained<AnyObject>> = msg_send![view, layer];
                let layer = layer.ok_or(Error::InvalidCommand)?;
                let valid: bool = msg_send![&*layer, isKindOfClass: class];
                if !valid {
                    return Err(Error::InvalidCommand);
                }
                let device: Option<Retained<AnyObject>> = msg_send![&*layer, device];
                let device = device.ok_or(Error::InvalidCommand)?;
                let registry_id: u64 = msg_send![&*device, registryID];
                Ok(Self {
                    device,
                    registry_id,
                    samples: 0,
                    sampled_max: 0,
                })
            }
        }

        pub fn require_same_device(&self, next: &Self) -> Result<(), Error> {
            if self.registry_id == next.registry_id && std::ptr::eq(&*self.device, &*next.device) {
                Ok(())
            } else {
                Err(Error::InvalidCommand)
            }
        }

        fn bytes(&self) -> Result<usize, Error> {
            MainThreadMarker::new().ok_or(Error::InvalidCommand)?;
            // SAFETY: capture verified the source; this retained object conforms
            // to MTLDevice. currentAllocatedSize returns NSUInteger on macOS.
            Ok(unsafe { msg_send![&*self.device, currentAllocatedSize] })
        }

        pub fn sample(&mut self) -> Result<(), Error> {
            self.sampled_max = self.sampled_max.max(self.bytes()?);
            self.samples = self.samples.checked_add(1).ok_or(Error::InvalidCommand)?;
            Ok(())
        }

        pub fn checkpoint(&self, cycle: u8) -> Result<(), Error> {
            if self.samples == 0 {
                return Err(Error::InvalidCommand);
            }
            let closed = self.bytes()?;
            let mut out = std::io::stdout().lock();
            writeln!(
                out,
                "GPUIO_METAL_AUDIT checkpoint ({cycle} {} {} {} {closed})",
                self.registry_id, self.samples, self.sampled_max
            )
            .map_err(|_| Error::Closed)?;
            out.flush().map_err(|_| Error::Closed)
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;
    pub struct Probe;
    impl Probe {
        pub fn capture(_: &gpui::Window) -> Result<Self, Error> {
            Err(Error::InvalidCommand)
        }
        pub fn require_same_device(&self, _: &Self) -> Result<(), Error> {
            Err(Error::InvalidCommand)
        }
        pub fn sample(&mut self) -> Result<(), Error> {
            Err(Error::InvalidCommand)
        }
        pub fn checkpoint(&self, _: u8) -> Result<(), Error> {
            Err(Error::InvalidCommand)
        }
    }
}

pub use platform::Probe;
