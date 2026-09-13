use super::*;
use gpui::{AppContext, Bounds, WindowBounds, WindowOptions, px, size};
use std::time::Duration;

async fn picker_action(label: &str, select_file: bool) -> bool {
    type Raw = *const std::ffi::c_void;
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXUIElementCreateApplication(pid: libc::pid_t) -> Raw;
        fn AXUIElementCopyAttributeValue(element: Raw, attribute: Raw, value: *mut Raw) -> i32;
        fn AXUIElementPerformAction(element: Raw, action: Raw) -> i32;
        fn AXUIElementSetAttributeValue(element: Raw, attribute: Raw, value: Raw) -> i32;
        fn AXIsProcessTrusted() -> bool;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: Raw);
        static kCFBooleanTrue: Raw;
        fn CFArrayGetCount(array: Raw) -> isize;
        fn CFArrayGetValueAtIndex(array: Raw, index: isize) -> Raw;
        fn CFGetTypeID(value: Raw) -> usize;
        fn CFArrayGetTypeID() -> usize;
        fn CFStringGetTypeID() -> usize;
    }
    struct Value(Raw);
    impl Drop for Value {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0) };
        }
    }
    fn attribute(element: Raw, name: &str) -> Option<Value> {
        let name = NSString::from_str(name);
        let mut value = std::ptr::null();
        let result = unsafe {
            AXUIElementCopyAttributeValue(element, Retained::as_ptr(&name).cast(), &mut value)
        };
        if result == 0 && !value.is_null() {
            Some(Value(value))
        } else {
            None
        }
    }
    fn string(element: Raw, name: &str) -> Option<String> {
        let value = attribute(element, name)?;
        unsafe {
            (CFGetTypeID(value.0) == CFStringGetTypeID())
                .then(|| (&*value.0.cast::<NSString>()).to_string())
        }
    }
    fn has_filename(element: Raw, label: &str, depth: usize) -> bool {
        if depth > 6 {
            return false;
        }
        if ["AXValue", "AXTitle", "AXDescription"]
            .iter()
            .any(|attribute| string(element, attribute).as_deref() == Some(label))
        {
            return true;
        }
        let Some(children) = attribute(element, "AXChildren") else {
            return false;
        };
        unsafe {
            if CFGetTypeID(children.0) != CFArrayGetTypeID() {
                return false;
            }
            let count = CFArrayGetCount(children.0);
            if count > 128 {
                return false;
            }
            (0..count).any(|index| {
                has_filename(CFArrayGetValueAtIndex(children.0, index), label, depth + 1)
            })
        }
    }
    fn visit(element: Raw, label: &str, select_file: bool, depth: usize) -> bool {
        if depth > 24 {
            return false;
        }
        if select_file
            && string(element, "AXRole").as_deref() == Some("AXRow")
            && has_filename(element, label, 0)
        {
            let attribute = NSString::from_str("AXSelected");
            return unsafe {
                AXUIElementSetAttributeValue(
                    element,
                    Retained::as_ptr(&attribute).cast(),
                    kCFBooleanTrue,
                ) == 0
            };
        }
        if !select_file
            && string(element, "AXRole").as_deref() == Some("AXButton")
            && string(element, "AXTitle").as_deref() == Some(label)
        {
            let action = NSString::from_str("AXPress");
            return unsafe {
                AXUIElementPerformAction(element, Retained::as_ptr(&action).cast()) == 0
            };
        }
        let Some(children) = attribute(element, "AXChildren") else {
            return false;
        };
        unsafe {
            if CFGetTypeID(children.0) != CFArrayGetTypeID() {
                return false;
            }
            let count = CFArrayGetCount(children.0);
            if count > 4096 {
                return false;
            }
            (0..count).any(|index| {
                visit(
                    CFArrayGetValueAtIndex(children.0, index),
                    label,
                    select_file,
                    depth + 1,
                )
            })
        }
    }
    // Cross-process accessibility requests must not block the GPUI main thread
    // while AppKit needs that thread to answer them. Target only this test PID.
    let (tx, rx) = async_channel::bounded(1);
    let label = label.to_owned();
    std::thread::spawn(move || {
        let result = unsafe {
            if AXIsProcessTrusted() {
                let app = Value(AXUIElementCreateApplication(libc::getpid()));
                visit(app.0, &label, select_file, 0)
            } else {
                eprintln!(
                    "GPUIO_FILE_DIALOG_AX_UNAVAILABLE: this native acceptance test needs accessibility access"
                );
                false
            }
        };
        let _ = tx.send_blocking(result);
    });
    rx.recv().await.unwrap()
}

type Results = Rc<RefCell<Vec<FileDialogResult>>>;

fn open_config() -> FileDialogConfig {
    FileDialogConfig::Open(OpenFileConfig {
        selection: FileSelection::FilesAndDirectories,
        multiple: true,
        title: "GPUIO picker test".into(),
        accept_label: "Choose".into(),
        directory: Some(FilePath::new(b"/tmp".to_vec()).unwrap()),
    })
}

fn show(
    cx: &mut gpui::AsyncApp,
    handle: gpui::WindowHandle<gpui::Empty>,
    config: FileDialogConfig,
    results: &Results,
) -> Panel {
    let results = results.clone();
    cx.update_window(handle.into(), |_, window, _| {
        Panel::show(config, window, move |result| {
            results.borrow_mut().push(result)
        })
    })
    .unwrap()
    .unwrap()
}

async fn settle(cx: &gpui::AsyncApp) {
    cx.background_executor()
        .timer(Duration::from_millis(200))
        .await;
}

async fn exercise(cx: &mut gpui::AsyncApp, handle: gpui::WindowHandle<gpui::Empty>) {
    settle(cx).await;
    let results: Results = Default::default();
    let panel = show(cx, handle, open_config(), &results);
    settle(cx).await;
    assert!(panel.is_pending());
    assert!(panel.0.panel.panel().isVisible());
    assert!(panel.0.panel.panel().sheetParent().is_some());
    let NativePanel::Open(native) = &panel.0.panel else {
        panic!()
    };
    assert!(native.canChooseFiles() && native.canChooseDirectories());
    assert!(native.allowsMultipleSelection());
    let overlapping = cx
        .update_window(handle.into(), |_, window, _| {
            Panel::show(open_config(), window, |_| {
                panic!("rejected request callback")
            })
        })
        .unwrap();
    assert!(matches!(overlapping, Err(FileDialogError::Busy)));
    // Exercise the actual native Cancel action, rather than a synthetic result.
    unsafe { panel.0.panel.panel().cancel(None) };
    settle(cx).await;
    assert!(!panel.is_pending());
    assert!(!panel.0.panel.panel().isVisible());
    assert_eq!(*results.borrow(), [FileDialogResult::Cancelled]);
    drop(panel);
    assert_eq!(results.borrow().len(), 1);
    results.borrow_mut().clear();
    let panel = show(cx, handle, open_config(), &results);
    // Owner cancellation must work even before the sheet has finished animating.
    let state = Rc::downgrade(&panel.0);
    panel.cancel();
    panel.cancel();
    assert!(!panel.is_pending());
    drop(panel);
    settle(cx).await;
    assert!(
        state.upgrade().is_none(),
        "no completion-block ownership cycle"
    );
    assert_eq!(*results.borrow(), [FileDialogResult::Cancelled]);
    results.borrow_mut().clear();
    let panel = show(cx, handle, open_config(), &results);
    let native: Retained<NSSavePanel> = match &panel.0.panel {
        NativePanel::Open(panel) => panel.clone().into_super(),
        NativePanel::Save(panel) => panel.clone(),
    };
    let state = Rc::downgrade(&panel.0);
    drop(panel);
    settle(cx).await;
    assert!(state.upgrade().is_none());
    assert!(!native.isVisible());
    assert_eq!(
        *results.borrow(),
        [FileDialogResult::Failed(FileDialogError::Closed)]
    );

    results.borrow_mut().clear();
    let config = FileDialogConfig::Open(OpenFileConfig {
        selection: FileSelection::Directories,
        multiple: false,
        title: "GPUIO directory picker test".into(),
        accept_label: "Choose directory".into(),
        directory: Some(FilePath::new(b"/tmp".to_vec()).unwrap()),
    });
    let panel = show(cx, handle, config, &results);
    settle(cx).await;
    assert!(
        picker_action("Choose directory", false).await,
        "native directory selection action"
    );
    for _ in 0..20 {
        if !panel.is_pending() {
            break;
        }
        settle(cx).await;
    }
    assert!(!panel.is_pending());
    let NativePanel::Open(native) = &panel.0.panel else {
        panic!()
    };
    let selected = native
        .URLs()
        .iter()
        .map(|url| url.to_file_path().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(
        std::fs::canonicalize(&selected[0]).unwrap(),
        std::fs::canonicalize("/tmp").unwrap()
    );
    assert_eq!(
        *results.borrow(),
        [FileDialogResult::Selected(vec![
            FilePath::new(selected[0].as_os_str().as_bytes().to_vec()).unwrap()
        ])]
    );
    drop(panel);
    assert_eq!(results.borrow().len(), 1);

    results.borrow_mut().clear();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let config = FileDialogConfig::Open(OpenFileConfig {
        selection: FileSelection::Files,
        multiple: false,
        title: "GPUIO file picker test".into(),
        accept_label: "Choose file".into(),
        directory: Some(FilePath::new(directory.as_os_str().as_bytes().to_vec()).unwrap()),
    });
    let panel = show(cx, handle, config, &results);
    settle(cx).await;
    assert!(
        picker_action("LICENSE", true).await,
        "select native file row"
    );
    assert!(
        picker_action("Choose file", false).await,
        "native file selection action"
    );
    for _ in 0..20 {
        if !panel.is_pending() {
            break;
        }
        settle(cx).await;
    }
    assert!(!panel.is_pending());
    {
        let observed = results.borrow();
        let FileDialogResult::Selected(paths) = &observed[0] else {
            panic!()
        };
        assert_eq!(paths.len(), 1);
        assert_eq!(
            std::fs::canonicalize(Path::new(std::ffi::OsStr::from_bytes(paths[0].as_bytes())))
                .unwrap(),
            std::fs::canonicalize(directory.join("LICENSE")).unwrap()
        );
    }
    drop(panel);
    assert_eq!(results.borrow().len(), 1);

    // Native Save accepts a destination URL without creating the file. Check
    // the extension pattern rewritten by the upstream GPUI workaround.
    results.borrow_mut().clear();
    let directory = std::env::temp_dir();
    let name = format!("gpuio-picker-{}.sql.s", std::process::id());
    let target = directory.join(&name);
    assert!(
        !target.exists(),
        "test must not select an existing destination"
    );
    let config = FileDialogConfig::Save(SaveFileConfig {
        directory: FilePath::new(directory.as_os_str().as_bytes().to_vec()).unwrap(),
        suggested_name: name,
        title: "GPUIO save path test (does not write a file)".into(),
        accept_label: "Choose path".into(),
    });
    let panel = show(cx, handle, config, &results);
    settle(cx).await;
    assert!(panel.0.panel.panel().isVisible());
    // NSSavePanel's public ok: action is deliberately unimplemented on this
    // AppKit version. Use the actual remote picker accessibility action instead.
    assert!(
        picker_action("Choose path", false).await,
        "native accessible Choose path button"
    );
    for _ in 0..20 {
        if !panel.is_pending() {
            break;
        }
        settle(cx).await;
    }
    if panel.is_pending() {
        let image = std::env::temp_dir().join(format!(
            "gpuio-file-dialog-{}-failure.png",
            std::process::id()
        ));
        let status = std::process::Command::new("/usr/sbin/screencapture")
            .args([
                "-x",
                "-l",
                &panel.0.panel.panel().windowNumber().to_string(),
            ])
            .arg(&image)
            .status();
        eprintln!("picker snapshot {image:?}: {status:?}");
        eprintln!(
            "name field: {:?}; first responder: {:?}",
            panel.0.panel.panel().nameFieldStringValue(),
            panel.0.panel.panel().firstResponder()
        );
    }
    assert!(!panel.is_pending(), "native save acceptance completed");
    let selected = panel.0.panel.panel().URL().unwrap().to_file_path().unwrap();
    assert_eq!(selected.file_name(), target.file_name());
    assert_eq!(
        *results.borrow(),
        [FileDialogResult::Selected(vec![
            FilePath::new(selected.as_os_str().as_bytes().to_vec()).unwrap()
        ])]
    );
    assert!(
        !target.exists(),
        "selecting a save path must not write the file"
    );
    assert!(!panel.0.panel.panel().isVisible());
    drop(panel);
    assert_eq!(results.borrow().len(), 1);
    println!(
        "GPUIO_FILE_DIALOG_MACOS_OK: actual sheet presentation, mixed/multiple configuration, busy, native cancel, early owner cancel/drop, cycle disposal, file/directory selection and exact save path without file creation"
    );
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(500.), px(350.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| gpui::Empty),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(exercise(cx, handle)).await;
            *task_failure.borrow_mut() = result.err();
            let _ = cx.update_window(handle.into(), |_, window, _| window.remove_window());
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
