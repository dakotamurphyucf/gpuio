// Compile the exact platform registry on both hosts: replacement, stale tags
// and exhaustion are portable invariants. AppKit delegate behavior is separately
// exercised by native_menus on macOS.
#[path = "../../../vendor/gpui-macos/src/menu_actions.rs"]
mod menu_actions;
