use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

fn transport() -> (Arc<Transport>, OwnedFd) {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    (
        Arc::new(Transport::new(writer.as_raw_fd()).unwrap()),
        reader,
    )
}
fn reserve(transport: &Transport, id: WindowId, request: i64) {
    transport
        .submit(
            Message::FileDialog(
                request,
                id,
                FileDialogConfig::Open(OpenFileConfig {
                    selection: FileSelection::Files,
                    multiple: false,
                    title: "Open".into(),
                    accept_label: "Choose".into(),
                    directory: None,
                }),
            ),
            64,
        )
        .unwrap();
    assert!(transport.mailbox.lock().unwrap().pop().is_some());
}
#[test]
fn cancellation_broadcasts_to_all_windows_and_cleanup_waiters() {
    let dialogs = Dialogs::default();
    let (transport, _read) = transport();
    let id = WindowId::from_parts(0, 1).unwrap();
    let other = WindowId::from_parts(1, 1).unwrap();
    reserve(&transport, id, 1);
    reserve(&transport, other, 2);
    let (cancel, first) = dialogs.begin(1, id, transport.clone()).unwrap();
    let (other_cancel, second) = dialogs.begin(2, other, transport.clone()).unwrap();
    let cleanup = dialogs.clear();
    let repeated = dialogs.clear();
    assert!(cancel.try_recv().is_ok());
    assert!(other_cancel.try_recv().is_ok());
    assert_eq!(
        transport.mailbox.lock().unwrap().drain(128),
        [
            Event::FileDialogResult(1, id, FileDialogResult::Failed(FileDialogError::Closed)),
            Event::FileDialogResult(2, other, FileDialogResult::Failed(FileDialogError::Closed)),
        ]
    );
    for completion in &cleanup.0 {
        assert!(!completion.done.is_closed());
    }
    // Late selection cannot escape owner cancellation; both cleanup observers
    // see completion through channel closure, without stealing a single value.
    first.finish(FileDialogResult::Selected(vec![
        gpuio_protocol::file_path::FilePath::new(b"/late".to_vec()).unwrap(),
    ]));
    second.finish(FileDialogResult::Failed(FileDialogError::Closed));
    for completion in cleanup.0.iter().chain(&repeated.0) {
        assert!(completion.done.is_closed());
        assert!(!completion.failed.load(Ordering::Acquire));
    }
    assert!(dialogs.owner.jobs.lock().unwrap().is_empty());
    assert!(transport.mailbox.lock().unwrap().drain(128).is_empty());
}
#[test]
fn overlap_generation_and_abandoned_worker_have_explicit_outcomes() {
    let dialogs = Dialogs::default();
    let (transport, _read) = transport();
    let id = WindowId::from_parts(0, 1).unwrap();
    reserve(&transport, id, 1);
    let (cancel, worker) = dialogs.begin(1, id, transport.clone()).unwrap();
    assert!(matches!(
        dialogs.begin(2, id, transport.clone()),
        Err(FileDialogError::Busy)
    ));
    let wrong = dialogs.close(WindowId::from_parts(0, 2).unwrap());
    assert!(wrong.0.is_empty());
    assert!(cancel.try_recv().is_err());
    drop(worker);
    assert!(dialogs.owner.jobs.lock().unwrap().is_empty());
    assert_eq!(
        transport.mailbox.lock().unwrap().drain(128),
        [Event::FileDialogResult(
            1,
            id,
            FileDialogResult::Failed(FileDialogError::NativeFailure)
        )]
    );
}
#[test]
fn natural_completion_and_cleanup_failure_retire_requests_once() {
    let dialogs = Dialogs::default();
    let (transport, _read) = transport();
    let id = WindowId::from_parts(0, 1).unwrap();
    reserve(&transport, id, 1);
    let (_cancel, worker) = dialogs.begin(1, id, transport.clone()).unwrap();
    worker.finish(FileDialogResult::Cancelled);
    assert_eq!(
        transport.mailbox.lock().unwrap().drain(128),
        [Event::FileDialogResult(1, id, FileDialogResult::Cancelled)]
    );
    reserve(&transport, id, 2);
    let (_cancel, worker) = dialogs.begin(2, id, transport.clone()).unwrap();
    let cleanup = dialogs.close(id);
    worker.finish(FileDialogResult::Failed(FileDialogError::NativeFailure));
    assert!(cleanup.0[0].done.is_closed());
    assert!(cleanup.0[0].failed.load(Ordering::Acquire));
    assert_eq!(
        transport.mailbox.lock().unwrap().drain(128),
        [Event::FileDialogResult(
            2,
            id,
            FileDialogResult::Failed(FileDialogError::Closed)
        )]
    );
}
#[test]
fn x11_parent_is_the_exact_native_id_and_wayland_is_not_faked() {
    assert_eq!(
        parent(RawWindowHandle::Xlib(
            raw_window_handle::XlibWindowHandle::new(0x1234)
        ))
        .unwrap(),
        "x11:1234"
    );
    assert_eq!(
        parent(RawWindowHandle::Xcb(
            raw_window_handle::XcbWindowHandle::new(std::num::NonZeroU32::new(0xab).unwrap())
        ))
        .unwrap(),
        "x11:ab"
    );
    assert!(
        parent(RawWindowHandle::Xlib(
            raw_window_handle::XlibWindowHandle::new(0)
        ))
        .is_err()
    );
    assert!(matches!(
        parent(RawWindowHandle::Wayland(
            raw_window_handle::WaylandWindowHandle::new(std::ptr::NonNull::dangling())
        )),
        Err(FileDialogError::Unsupported)
    ));
}

#[test]
fn closing_waits_for_a_response_that_is_already_being_delivered() {
    let dialogs = Dialogs::default();
    let (transport, _read) = transport();
    let id = WindowId::from_parts(0, 1).unwrap();
    reserve(&transport, id, 1);
    let (_cancel, worker) = dialogs.begin(1, id, transport.clone()).unwrap();
    let mailbox = transport.mailbox.lock().unwrap();
    let thread = std::thread::spawn(move || worker.finish(FileDialogResult::Cancelled));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if dialogs
            .owner
            .jobs
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|job| job.phase == Phase::Delivering)
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "worker did not claim its response"
        );
        std::thread::yield_now();
    }
    let cleanup = dialogs.close(id);
    assert!(
        !cleanup.0[0].done.is_closed(),
        "cleanup must not finish while the transport send is blocked"
    );
    drop(mailbox);
    let completion = cleanup.0[0].clone();
    cleanup.wait_before_quit();
    assert!(completion.done.is_closed());
    thread.join().unwrap();
    assert_eq!(
        transport.mailbox.lock().unwrap().drain(128),
        [Event::FileDialogResult(1, id, FileDialogResult::Cancelled)]
    );
}

#[test]
fn dropping_a_manager_clone_does_not_cancel_the_live_owner() {
    let dialogs = Dialogs::default();
    let (transport, _read) = transport();
    let id = WindowId::from_parts(0, 1).unwrap();
    reserve(&transport, id, 1);
    let (cancel, worker) = dialogs.begin(1, id, transport.clone()).unwrap();
    drop(dialogs.clone());
    assert!(cancel.try_recv().is_err());
    drop(dialogs);
    assert!(cancel.try_recv().is_ok());
    worker.finish(FileDialogResult::Failed(FileDialogError::Closed));
    assert_eq!(
        transport.mailbox.lock().unwrap().drain(128),
        [Event::FileDialogResult(
            1,
            id,
            FileDialogResult::Failed(FileDialogError::Closed)
        )]
    );
}

#[test]
fn worker_panic_is_contained_as_native_failure() {
    let mut request = std::pin::pin!(protect(async { panic!("test worker failure") }));
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    assert_eq!(
        request.as_mut().poll(&mut cx),
        std::task::Poll::Ready(FileDialogResult::Failed(FileDialogError::NativeFailure))
    );
}
