//! Separate-process test driver. Accessibility lookup and mouse events are
//! restricted to the explicitly supplied child PID. System-wide AX hit-testing
//! verifies the owner before posting; the caller authorizes foreground testing.
use objc2::rc::Retained;
use objc2_foundation::NSString;
use std::{
    ffi::c_void,
    time::{Duration, Instant},
};
type Raw = *const c_void;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Size {
    width: f64,
    height: f64,
}
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXUIElementCreateSystemWide() -> Raw;
    fn AXUIElementCopyElementAtPosition(app: Raw, x: f32, y: f32, element: *mut Raw) -> i32;
    fn AXUIElementGetPid(element: Raw, pid: *mut libc::pid_t) -> i32;
    fn AXUIElementCreateApplication(pid: libc::pid_t) -> Raw;
    fn AXUIElementCopyAttributeValue(element: Raw, attribute: Raw, value: *mut Raw) -> i32;
    fn AXUIElementPerformAction(element: Raw, action: Raw) -> i32;
    fn AXUIElementSetAttributeValue(element: Raw, attribute: Raw, value: Raw) -> i32;
    fn AXValueGetType(value: Raw) -> u32;
    fn AXValueGetValue(value: Raw, kind: u32, result: *mut c_void) -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: Raw);
    static kCFBooleanTrue: Raw;
    fn CFGetTypeID(value: Raw) -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFArrayGetCount(array: Raw) -> isize;
    fn CFArrayGetValueAtIndex(array: Raw, index: isize) -> Raw;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateMouseEvent(source: Raw, kind: u32, position: Point, button: u32) -> Raw;
    fn CGEventPost(location: u32, event: Raw);
    fn CGEventSetIntegerValueField(event: Raw, field: u32, value: i64);
}
struct Owned(Raw);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}
fn attribute(element: Raw, name: &str) -> Option<Owned> {
    let name = NSString::from_str(name);
    let mut value = std::ptr::null();
    let result = unsafe {
        AXUIElementCopyAttributeValue(element, Retained::as_ptr(&name).cast(), &mut value)
    };
    if result == 0 && !value.is_null() {
        Some(Owned(value))
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
fn center(element: Raw) -> Option<Point> {
    let position = attribute(element, "AXPosition")?;
    let size = attribute(element, "AXSize")?;
    let mut point = Point::default();
    let mut dimensions = Size::default();
    let valid = unsafe {
        AXValueGetType(position.0) == 1
            && AXValueGetType(size.0) == 2
            && AXValueGetValue(position.0, 1, (&mut point as *mut Point).cast())
            && AXValueGetValue(size.0, 2, (&mut dimensions as *mut Size).cast())
    };
    (valid
        && dimensions.width > 0.
        && dimensions.height > 0.
        && [point.x, point.y, dimensions.width, dimensions.height]
            .iter()
            .all(|x| x.is_finite()))
    .then_some(Point {
        x: point.x + dimensions.width / 2.,
        y: point.y + dimensions.height / 2.,
    })
}
fn find(element: Raw, label: &str, depth: usize, budget: &mut usize) -> Option<Point> {
    if depth > 32 || *budget == 0 {
        return None;
    }
    *budget -= 1;
    if string(element, "AXRole").as_deref() == Some("AXWindow") {
        let action = NSString::from_str("AXRaise");
        unsafe {
            AXUIElementPerformAction(element, Retained::as_ptr(&action).cast());
        }
    }
    if string(element, "AXRole").as_deref() == Some("AXGroup")
        && ["AXTitle", "AXDescription", "AXValue"]
            .iter()
            .any(|attribute| string(element, attribute).as_deref() == Some(label))
        && let Some(point) = center(element)
    {
        return Some(point);
    }
    let children = attribute(element, "AXChildren")?;
    unsafe {
        if CFGetTypeID(children.0) != CFArrayGetTypeID() {
            return None;
        }
        let count = CFArrayGetCount(children.0);
        if !(0..=4096).contains(&count) {
            return None;
        }
        for index in 0..count {
            if let Some(point) = find(
                CFArrayGetValueAtIndex(children.0, index),
                label,
                depth + 1,
                budget,
            ) {
                return Some(point);
            }
        }
    }
    None
}
fn locate(pid: libc::pid_t, label: &str) -> Point {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        let raw = unsafe { AXUIElementCreateApplication(pid) };
        assert!(!raw.is_null(), "cannot create child AX application");
        let app = Owned(raw);
        if let Some(point) = find(app.0, label, 0, &mut 4096) {
            return point;
        }
        assert!(
            Instant::now() < deadline,
            "child AX group not found: {label}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn owner_at(point: Point) -> Option<libc::pid_t> {
    let raw = unsafe { AXUIElementCreateSystemWide() };
    if raw.is_null() {
        return None;
    }
    let system = Owned(raw);
    let mut raw = std::ptr::null();
    let status = unsafe {
        AXUIElementCopyElementAtPosition(system.0, point.x as f32, point.y as f32, &mut raw)
    };
    if status != 0 || raw.is_null() {
        return None;
    }
    let element = Owned(raw);
    let mut pid = 0;
    (unsafe { AXUIElementGetPid(element.0, &mut pid) } == 0).then_some(pid)
}
fn post(pid: libc::pid_t, kind: u32, point: Point) {
    // Always release a pressed button during unwinding; other posts must still
    // target the child's actual topmost on-screen window.
    if kind != 2 {
        assert_eq!(
            owner_at(point),
            Some(pid),
            "test window is obscured or moved"
        );
    }
    let raw = unsafe { CGEventCreateMouseEvent(std::ptr::null(), kind, point, 0) };
    assert!(!raw.is_null(), "CGEventCreateMouseEvent failed");
    let event = Owned(raw);
    unsafe {
        CGEventSetIntegerValueField(event.0, 1, 1);
        CGEventPost(0, event.0);
    }
}
struct Release {
    pid: libc::pid_t,
    point: Point,
}
impl Drop for Release {
    fn drop(&mut self) {
        post(self.pid, 2, self.point);
    }
}
pub(crate) fn drive(pid: libc::pid_t) {
    assert!(pid > 0);
    assert!(
        unsafe { AXIsProcessTrusted() },
        "GPUIO_DRAG_AX_UNAVAILABLE: test driver needs accessibility access"
    );
    // Wait for the rendered AX tree before asking AppKit to activate the child.
    // Immediately after spawn its application messaging server may not exist.
    locate(pid, "Greeting");
    let raw = unsafe { AXUIElementCreateApplication(pid) };
    assert!(!raw.is_null(), "cannot create child AX application");
    let app = Owned(raw);
    let front = NSString::from_str("AXFrontmost");
    let status = unsafe {
        AXUIElementSetAttributeValue(app.0, Retained::as_ptr(&front).cast(), kCFBooleanTrue)
    };
    assert_eq!(status, 0, "cannot activate the child application");
    std::thread::sleep(Duration::from_millis(200));
    let source = locate(pid, "Greeting");
    let target = locate(pid, "Drop text or files");
    eprintln!("GPUIO_DRAG_AX_TARGETS: pid={pid} source={source:?} target={target:?}");
    post(pid, 5, source);
    std::thread::sleep(Duration::from_millis(100));
    post(pid, 1, source);
    let mut release = Release { pid, point: source };
    for step in 1..=24 {
        let fraction = step as f64 / 24.;
        let point = Point {
            x: source.x + (target.x - source.x) * fraction,
            y: source.y + (target.y - source.y) * fraction,
        };
        release.point = point;
        post(pid, 6, point);
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(release);
    println!("GPUIO_DRAG_APPKIT_SENT: one system drag posted within verified child window");
}
