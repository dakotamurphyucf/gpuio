//! Owned macOS panels. All operations, including Drop, stay on the GPUI main
//! thread. A weak completion block prevents a panel/block/owner reference cycle.
use block2::RcBlock;
use gpuio_protocol::{
    file_dialog::*,
    file_path::{FilePath, PathError},
};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSModalResponseCancel, NSModalResponseOK, NSOpenPanel, NSSavePanel, NSView};
use objc2_foundation::{NSString, NSURL};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::RefCell, os::unix::ffi::OsStrExt, path::Path, rc::Rc};

type Completion = Box<dyn FnOnce(FileDialogResult)>;

enum NativePanel {
    Open(Retained<NSOpenPanel>),
    Save(Retained<NSSavePanel>),
}

impl NativePanel {
    fn panel(&self) -> &NSSavePanel {
        match self {
            Self::Open(panel) => panel,
            Self::Save(panel) => panel,
        }
    }

    fn selected(&self) -> Result<Vec<FilePath>, FileDialogError> {
        fn path(url: &NSURL) -> Result<FilePath, FileDialogError> {
            if !url.isFileURL() {
                return Err(FileDialogError::NativeFailure);
            }
            let path = url.to_file_path().ok_or(FileDialogError::NativeFailure)?;
            FilePath::new(path.as_os_str().as_bytes().to_vec()).map_err(|error| match error {
                PathError::TooLong => FileDialogError::LimitExceeded,
                PathError::NotAbsolute | PathError::ContainsNul => FileDialogError::NativeFailure,
            })
        }
        match self {
            Self::Open(panel) => {
                let urls = panel.URLs();
                if urls.len() > MAX_SELECTED_PATHS {
                    return Err(FileDialogError::LimitExceeded);
                }
                let mut paths = Vec::with_capacity(urls.len());
                let mut bytes = 0;
                for url in urls.iter() {
                    let path = path(&url)?;
                    bytes += path.as_bytes().len();
                    if bytes > MAX_SELECTED_PATH_BYTES {
                        return Err(FileDialogError::LimitExceeded);
                    }
                    paths.push(path);
                }
                Ok(paths)
            }
            Self::Save(panel) => {
                let url = panel.URL().ok_or(FileDialogError::NativeFailure)?;
                path(&url).map(|path| vec![path])
            }
        }
    }
}

struct State {
    panel: NativePanel,
    config: FileDialogConfig,
    complete: RefCell<Option<Completion>>,
}

impl State {
    fn finish(&self, result: FileDialogResult) {
        // Take before calling AppKit or the callback: either can reenter.
        let complete = self.complete.borrow_mut().take();
        if let Some(complete) = complete {
            self.panel.panel().orderOut(None);
            complete(result);
        }
    }

    fn cancel(&self, result: FileDialogResult) {
        // Claim the terminal result before cancel: AppKit may immediately invoke
        // its completion with NSModalResponseCancel during the following call.
        let complete = self.complete.borrow_mut().take();
        if let Some(complete) = complete {
            // SAFETY: nil is a valid action sender, on the main thread.
            unsafe { self.panel.panel().cancel(None) };
            self.panel.panel().orderOut(None);
            complete(result);
        }
    }
}

/// One window-attached native picker. Dropping it dismisses the physical panel
/// and completes an outstanding callback with Closed. It is intentionally !Send.
pub struct Panel(Rc<State>);

impl Panel {
    /// Present a sheet on the exact GPUI native window. No file is opened or
    /// written; successful save results preserve the URL's path without rewriting.
    pub fn show(
        config: FileDialogConfig,
        window: &gpui::Window,
        complete: impl FnOnce(FileDialogResult) + 'static,
    ) -> Result<Self, FileDialogError> {
        if !config.is_valid() {
            return Err(FileDialogError::InvalidRequest);
        }
        let marker = MainThreadMarker::new().ok_or(FileDialogError::NativeFailure)?;
        let native_handle =
            HasWindowHandle::window_handle(window).map_err(|_| FileDialogError::NativeFailure)?;
        let RawWindowHandle::AppKit(handle) = native_handle.as_raw() else {
            return Err(FileDialogError::Unsupported);
        };
        // SAFETY: GPUI's borrowed AppKit handle points to a live NSView. Its
        // window is retained below before presenting, on the main thread.
        let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
        let parent = view.window().ok_or(FileDialogError::Closed)?;
        if parent.attachedSheet().is_some() {
            return Err(FileDialogError::Busy);
        }
        let native = match &config {
            FileDialogConfig::Capabilities => return Err(FileDialogError::InvalidRequest),
            FileDialogConfig::Open(config) => {
                let panel = NSOpenPanel::openPanel(marker);
                panel.setCanChooseFiles(matches!(
                    config.selection,
                    FileSelection::Files | FileSelection::FilesAndDirectories
                ));
                panel.setCanChooseDirectories(matches!(
                    config.selection,
                    FileSelection::Directories | FileSelection::FilesAndDirectories
                ));
                panel.setAllowsMultipleSelection(config.multiple);
                panel.setResolvesAliases(false);
                panel.setTitle(Some(&NSString::from_str(&config.title)));
                panel.setPrompt(Some(&NSString::from_str(&config.accept_label)));
                if let Some(directory) = &config.directory {
                    let url = directory_url(directory)?;
                    panel.setDirectoryURL(Some(&url));
                }
                NativePanel::Open(panel)
            }
            FileDialogConfig::Save(config) => {
                let panel = NSSavePanel::savePanel(marker);
                let url = directory_url(&config.directory)?;
                panel.setDirectoryURL(Some(&url));
                panel.setNameFieldStringValue(&NSString::from_str(&config.suggested_name));
                panel.setTitle(Some(&NSString::from_str(&config.title)));
                panel.setPrompt(Some(&NSString::from_str(&config.accept_label)));
                NativePanel::Save(panel)
            }
        };
        native.panel().setCanCreateDirectories(true);
        let state = Rc::new(State {
            panel: native,
            config,
            complete: RefCell::new(Some(Box::new(complete))),
        });
        let weak = Rc::downgrade(&state);
        let handler = RcBlock::new(move |response| {
            if let Some(state) = weak.upgrade() {
                if state.complete.borrow().is_none() {
                    return;
                }
                let result = if response == NSModalResponseOK {
                    match state.panel.selected() {
                        Ok(paths) if state.config.accepts_selection(&paths) => {
                            FileDialogResult::Selected(paths)
                        }
                        Ok(_) => FileDialogResult::Failed(FileDialogError::LimitExceeded),
                        Err(error) => FileDialogResult::Failed(error),
                    }
                } else if response == NSModalResponseCancel {
                    FileDialogResult::Cancelled
                } else {
                    FileDialogResult::Failed(FileDialogError::NativeFailure)
                };
                state.finish(result);
            }
        });
        state
            .panel
            .panel()
            .beginSheetModalForWindow_completionHandler(&parent, &handler);
        Ok(Self(state))
    }

    pub fn is_pending(&self) -> bool {
        self.0.complete.borrow().is_some()
    }

    pub fn cancel(&self) {
        self.0.cancel(FileDialogResult::Cancelled);
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        self.0
            .cancel(FileDialogResult::Failed(FileDialogError::Closed));
    }
}

fn directory_url(path: &FilePath) -> Result<Retained<NSURL>, FileDialogError> {
    NSURL::from_directory_path(Path::new(std::ffi::OsStr::from_bytes(path.as_bytes())))
        .ok_or(FileDialogError::InvalidRequest)
}

#[cfg(feature = "native-tests")]
#[path = "file_dialog_test.rs"]
pub(crate) mod test;
