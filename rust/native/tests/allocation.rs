//! Isolated integration-test process: measure allocations only during one edit.
use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, WindowId, v1::*};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
struct Counting;
static ENABLED: AtomicBool = AtomicBool::new(false);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ENABLED.load(Ordering::Relaxed) {
            BYTES.fetch_add(new_size, Ordering::Relaxed);
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
#[test]
fn small_edit_allocation_does_not_scale_with_retained_history() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let id = |slot| NodeId::from_parts(slot, 1).unwrap();
    let mut tree = Tree::new(window);
    let apply = |tree: &mut Tree, operations| {
        tree.apply(&Transaction {
            window,
            base: tree.revision(),
            revision: tree.revision() + 1,
            operations,
        })
        .unwrap();
    };
    apply(
        &mut tree,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetRoot(Some(id(0))),
        ],
    );
    for batch in 0..5 {
        let start = batch * 2000 + 1;
        let nodes = (start..start + 2000).map(id).collect::<Vec<_>>();
        let mut operations = nodes
            .iter()
            .map(|id| Op::Create(*id, Kind::Text, "x".repeat(1024), None))
            .collect::<Vec<_>>();
        operations.push(Op::Splice(id(0), batch * 2000, 0, nodes));
        apply(&mut tree, operations);
    }
    let transaction = Transaction {
        window,
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations: vec![Op::SetText(id(5000), "y".repeat(1024))],
    };
    let start = std::time::Instant::now();
    ENABLED.store(true, Ordering::Relaxed);
    let result = tree.apply(&transaction).unwrap();
    ENABLED.store(false, Ordering::Relaxed);
    let elapsed = start.elapsed();
    let bytes = BYTES.load(Ordering::Relaxed);
    assert_eq!(result.touched_records, 1);
    assert_eq!(result.validated_nodes, 0);
    assert!(bytes < 65536, "small edit allocated {bytes} bytes");
    eprintln!(
        "BRIDGE_ALLOCATION nodes={} retained_payload_bytes={} edit_allocated_bytes={} edit_allocation_calls={} latency_us={}",
        tree.len(),
        tree.retained_bytes(),
        bytes,
        CALLS.load(Ordering::Relaxed),
        elapsed.as_micros()
    );
}
