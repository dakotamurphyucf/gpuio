use super::*;
use gpui::{AppContext, Bounds, WindowBounds, WindowOptions, px, size};
use std::time::Duration;

#[derive(Clone, Copy)]
enum PickerAction {
    Select,
    ExtendSelection,
    Press,
}

fn picker_action_sync(pid: libc::pid_t, label: &str, action: PickerAction, diagnose: bool) -> bool {
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
        fn CFArrayCreate(allocator: Raw, values: *const Raw, count: isize, callbacks: Raw) -> Raw;
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
    fn visit(
        element: Raw,
        label: &str,
        action: PickerAction,
        depth: usize,
        diagnose: bool,
    ) -> bool {
        let select_file = !matches!(action, PickerAction::Press);
        if depth > 24 {
            return false;
        }
        let role = string(element, "AXRole");
        // Action buttons are outside the file table. Traversing every filename
        // here makes button lookup depend on the size of the user's directory.
        if !select_file && matches!(role.as_deref(), Some("AXTable" | "AXOutline" | "AXRow")) {
            return false;
        }
        if diagnose && matches!(role.as_deref(), Some("AXButton" | "AXWindow" | "AXSheet")) {
            eprintln!(
                "PICKER_AX depth={depth} role={role:?} title={:?} description={:?}",
                string(element, "AXTitle"),
                string(element, "AXDescription")
            );
        }
        if select_file && role.as_deref() == Some("AXRow") && has_filename(element, label, 0) {
            if matches!(action, PickerAction::ExtendSelection) {
                let Some(parent) = attribute(element, "AXParent") else {
                    return false;
                };
                let Some(selected) = attribute(parent.0, "AXSelectedRows") else {
                    return false;
                };
                unsafe {
                    if CFGetTypeID(selected.0) != CFArrayGetTypeID() {
                        return false;
                    }
                    let count = CFArrayGetCount(selected.0);
                    if count > 128 {
                        return false;
                    }
                    let mut rows: Vec<_> = (0..count)
                        .map(|index| CFArrayGetValueAtIndex(selected.0, index))
                        .collect();
                    rows.push(element);
                    // The selected array and traversal own these AX elements
                    // until SetAttributeValue returns; no retain callbacks needed.
                    let raw = CFArrayCreate(
                        std::ptr::null(),
                        rows.as_ptr(),
                        rows.len() as isize,
                        std::ptr::null(),
                    );
                    if raw.is_null() {
                        return false;
                    }
                    let rows = Value(raw);
                    let name = NSString::from_str("AXSelectedRows");
                    return AXUIElementSetAttributeValue(
                        parent.0,
                        Retained::as_ptr(&name).cast(),
                        rows.0,
                    ) == 0;
                }
            }
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
            && role.as_deref() == Some("AXButton")
            && string(element, "AXTitle").as_deref() == Some(label)
        {
            let action = NSString::from_str("AXPress");
            let result =
                unsafe { AXUIElementPerformAction(element, Retained::as_ptr(&action).cast()) };
            if diagnose {
                eprintln!("PICKER_AX_PRESS label={label:?} result={result}");
            }
            return result == 0;
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
            (0..count).rev().any(|index| {
                visit(
                    CFArrayGetValueAtIndex(children.0, index),
                    label,
                    action,
                    depth + 1,
                    diagnose,
                )
            })
        }
    }
    unsafe {
        if AXIsProcessTrusted() {
            let app = Value(AXUIElementCreateApplication(pid));
            visit(app.0, label, action, 0, diagnose)
        } else {
            eprintln!(
                "GPUIO_FILE_DIALOG_AX_UNAVAILABLE: this native acceptance test needs accessibility access"
            );
            false
        }
    }
}

async fn picker_action(label: &str, action: PickerAction, diagnose: bool) -> bool {
    // Same-process AX requests must not block the GPUI main thread while AppKit
    // needs that thread to answer them. Target only this test PID.
    let (tx, rx) = async_channel::bounded(1);
    let label = label.to_owned();
    std::thread::spawn(move || {
        let result = picker_action_sync(unsafe { libc::getpid() }, &label, action, diagnose);
        let _ = tx.send_blocking(result);
    });
    rx.recv().await.unwrap()
}

// Standalone test driver for the public Bonsai/Eio example. The caller supplies
// the child PID explicitly; it never searches/acts on unrelated applications.
pub(crate) fn drive(pid: libc::pid_t, filename: &str, accept_label: &str) {
    assert!(pid > 0);
    for (label, action) in [
        (filename, PickerAction::Select),
        (accept_label, PickerAction::Press),
    ] {
        let mut succeeded = false;
        for attempt in 0..30 {
            if picker_action_sync(pid, label, action, attempt == 29) {
                succeeded = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        assert!(succeeded, "native picker action failed: {label}");
    }
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

async fn ready_action(cx: &gpui::AsyncApp, label: &str, action: PickerAction) -> bool {
    // Remote AppKit controls are published asynchronously. Stop after the first
    // successful AX action; do not repeatedly activate a successful button.
    for attempt in 0..30 {
        if picker_action(label, action, attempt == 29).await {
            return true;
        }
        settle(cx).await;
    }
    false
}

async fn request_ownership(cx: &mut gpui::AsyncApp, handle: gpui::WindowHandle<gpui::Empty>) {
    use super::super::Dialogs;
    use crate::transport::Transport;
    use gpuio_protocol::{
        WindowId,
        v1::{Event, Message},
    };
    use std::{
        os::fd::{AsRawFd, FromRawFd, OwnedFd},
        sync::Arc,
    };
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let dialogs = Dialogs::default();
    let id = WindowId::from_parts(0, 1).unwrap();
    let second_id = WindowId::from_parts(1, 1).unwrap();
    let config = FileDialogConfig::Open(OpenFileConfig {
        selection: FileSelection::Directories,
        multiple: false,
        title: "GPUIO owned picker".into(),
        accept_label: "Choose directory".into(),
        directory: Some(FilePath::new(b"/tmp".to_vec()).unwrap()),
    });
    let show = |cx: &mut gpui::AsyncApp, handle: gpui::WindowHandle<gpui::Empty>, id, request| {
        transport
            .submit(Message::FileDialog(request, id, config.clone()), 128)
            .unwrap();
        assert!(transport.mailbox.lock().unwrap().pop().is_some());
        cx.update_window(handle.into(), |_, window, cx| {
            dialogs.show(request, id, config.clone(), window, cx, transport.clone())
        })
        .unwrap();
    };
    let responses = || transport.mailbox.lock().unwrap().drain(128);
    show(cx, handle, id, 1);
    show(cx, handle, id, 2);
    assert_eq!(
        responses(),
        [Event::FileDialogResult(
            2,
            id,
            FileDialogResult::Failed(FileDialogError::Busy)
        )]
    );
    assert_eq!(dialogs.pending.borrow().len(), 1);
    settle(cx).await;
    assert!(ready_action(cx, "Choose directory", PickerAction::Press).await);
    for _ in 0..20 {
        if dialogs.pending.borrow().is_empty() {
            break;
        }
        settle(cx).await;
    }
    assert!(
        dialogs.pending.borrow().is_empty(),
        "selection releases its native request owner"
    );
    let events = responses();
    assert!(
        matches!(&events[..], [Event::FileDialogResult(1, result_window, FileDialogResult::Selected(paths))] if *result_window == id && paths.len() == 1)
    );
    show(cx, handle, id, 3);
    dialogs
        .close(WindowId::from_parts(0, 2).unwrap())
        .wait()
        .await;
    assert_eq!(
        dialogs.pending.borrow().len(),
        1,
        "another generation cannot cancel this picker"
    );
    dialogs.close(id).wait().await;
    assert!(dialogs.pending.borrow().is_empty());
    assert_eq!(
        responses(),
        [Event::FileDialogResult(
            3,
            id,
            FileDialogResult::Failed(FileDialogError::Closed)
        )]
    );
    settle(cx).await;
    let second = cx
        .update(|cx| cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| gpui::Empty)))
        .unwrap();
    show(cx, handle, id, 4);
    show(cx, second, second_id, 5);
    assert_eq!(dialogs.pending.borrow().len(), 2);
    dialogs.clear().wait().await;
    assert!(dialogs.pending.borrow().is_empty());
    assert_eq!(
        responses(),
        [
            Event::FileDialogResult(4, id, FileDialogResult::Failed(FileDialogError::Closed)),
            Event::FileDialogResult(
                5,
                second_id,
                FileDialogResult::Failed(FileDialogError::Closed)
            ),
        ]
    );
    settle(cx).await;
    assert!(
        responses().is_empty(),
        "late native completions cannot consume another response reservation"
    );
    cx.update_window(second.into(), |_, window, _| window.remove_window())
        .unwrap();
    println!(
        "GPUIO_FILE_DIALOG_REQUESTS_OK: selection disposal, Busy, exact generation, two-window cancellation and response reservation ownership"
    );
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
        ready_action(cx, "Choose directory", PickerAction::Press).await,
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
        ready_action(cx, "LICENSE", PickerAction::Select).await,
        "select native file row"
    );
    assert!(
        ready_action(cx, "Choose file", PickerAction::Press).await,
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

    // Selecting two native rows must preserve both paths through the result
    // adapter; inspecting allowsMultipleSelection alone would not prove this.
    results.borrow_mut().clear();
    let config = FileDialogConfig::Open(OpenFileConfig {
        selection: FileSelection::Files,
        multiple: true,
        title: "GPUIO multiple-file picker test".into(),
        accept_label: "Choose files".into(),
        directory: Some(FilePath::new(directory.as_os_str().as_bytes().to_vec()).unwrap()),
    });
    let panel = show(cx, handle, config, &results);
    settle(cx).await;
    for (name, action) in [
        ("LICENSE", PickerAction::Select),
        ("README.md", PickerAction::ExtendSelection),
    ] {
        assert!(
            ready_action(cx, name, action).await,
            "select native row {name}"
        );
    }
    assert!(ready_action(cx, "Choose files", PickerAction::Press).await);
    for _ in 0..20 {
        if !panel.is_pending() {
            break;
        }
        settle(cx).await;
    }
    assert!(!panel.is_pending());
    {
        let observed = results.borrow();
        let [FileDialogResult::Selected(paths)] = &observed[..] else {
            panic!("multiple selection: {observed:?}")
        };
        let mut selected: Vec<_> = paths
            .iter()
            .map(|path| {
                std::fs::canonicalize(Path::new(std::ffi::OsStr::from_bytes(path.as_bytes())))
                    .unwrap()
            })
            .collect();
        selected.sort();
        let mut expected: Vec<_> = ["LICENSE", "README.md"]
            .into_iter()
            .map(|name| std::fs::canonicalize(directory.join(name)).unwrap())
            .collect();
        expected.sort();
        assert_eq!(selected, expected);
    }
    drop(panel);

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
        ready_action(cx, "Choose path", PickerAction::Press).await,
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
        "GPUIO_FILE_DIALOG_MACOS_OK: actual sheet presentation, mixed/multiple configuration, busy, native cancel, early owner cancel/drop, cycle disposal, file/directory/multiple selection and exact save path without file creation"
    );
    request_ownership(cx, handle).await;
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
