//! Actual AppKit menu replacement must retire command payloads, including when
//! an OS client still retains an old NSMenuItem. Bar and Dock owners are separate.
use gpui::{AsyncApp, Menu, MenuItem};
use objc2::{
    class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Clone, PartialEq, Eq, gpui::Action)]
#[action(namespace = gpuio_test, no_json, no_register)]
struct RetentionProbe {
    revision: usize,
    owner: Arc<()>,
}

// Retain actual OS objects across replacement to exercise late delegate calls.
fn native_item(dock: bool) -> Retained<AnyObject> {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let menu: *mut AnyObject = if dock {
            let delegate: *mut AnyObject = msg_send![app, delegate];
            msg_send![delegate, applicationDockMenu: app]
        } else {
            let main: *mut AnyObject = msg_send![app, mainMenu];
            let item: *mut AnyObject = msg_send![main, itemAtIndex: 0usize];
            msg_send![item, submenu]
        };
        assert!(!menu.is_null());
        let item: *mut AnyObject = msg_send![menu, itemAtIndex: 0usize];
        Retained::retain(item).expect("native menu item")
    }
}

fn validate_and_invoke(item: &AnyObject, expected_available: bool) {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        let available: Bool = msg_send![delegate, validateMenuItem: item];
        assert_eq!(available.as_bool(), expected_available);
        // Exercise a late action even when validation rejected the stale item.
        let _: () = msg_send![delegate, handleGPUIMenuItem: item];
    }
}

pub(super) fn exercise(cx: &mut AsyncApp) {
    let dispatched = Rc::new(RefCell::new(Vec::new()));
    cx.update(|cx| {
        let dispatched = dispatched.clone();
        cx.on_action(move |action: &RetentionProbe, _| {
            dispatched.borrow_mut().push(action.revision);
        });
    });
    let dock_owner = Arc::new(());
    let dock_weak = Arc::downgrade(&dock_owner);
    cx.update(|cx| {
        cx.set_dock_menu(vec![MenuItem::action(
            "Dock retention probe",
            RetentionProbe {
                revision: 0,
                owner: dock_owner,
            },
        )])
    });
    let dock_item = native_item(true);
    let mut stale_item: Option<Retained<AnyObject>> = None;
    let mut previous = None;
    for revision in 1..=128 {
        let owner = Arc::new(());
        let weak = Arc::downgrade(&owner);
        cx.update(|cx| {
            cx.set_menus([Menu::new("Retention probe").items([MenuItem::action(
                "Current command",
                RetentionProbe { revision, owner },
            )])])
        });
        if let Some(previous) = previous {
            assert_eq!(
                std::sync::Weak::strong_count(&previous),
                0,
                "replaced native menu retained its command payload at revision {revision}"
            );
        }
        assert!(
            dock_weak.strong_count() > 0,
            "bar replacement retired Dock owner"
        );
        previous = Some(weak);
        if let Some(stale) = &stale_item {
            validate_and_invoke(stale, false);
        }
        let item = native_item(false);
        validate_and_invoke(&item, true);
        assert_eq!(dispatched.borrow().as_slice(), &[revision]);
        dispatched.borrow_mut().clear();
        stale_item = Some(item);
    }
    validate_and_invoke(&dock_item, true);
    assert_eq!(dispatched.borrow().as_slice(), &[0]);
    dispatched.borrow_mut().clear();
    cx.update(|cx| cx.set_menus(Vec::<Menu>::new()));
    assert_eq!(
        previous.unwrap().strong_count(),
        0,
        "cleared bar retained its last action"
    );
    assert!(dock_weak.strong_count() > 0);
    validate_and_invoke(stale_item.as_ref().unwrap(), false);
    cx.update(|cx| cx.set_dock_menu(vec![]));
    assert_eq!(
        dock_weak.strong_count(),
        0,
        "cleared Dock retained its action"
    );
    validate_and_invoke(&dock_item, false);
    assert!(dispatched.borrow().is_empty());
    println!(
        "GPUIO_MENU_RETENTION_OK: 128 bar replacements, stale OS delegates, current dispatch and independent Dock teardown"
    );
}
