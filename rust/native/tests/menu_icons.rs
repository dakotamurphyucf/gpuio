use gpui::{Menu, MenuIcon, MenuItem, NoAction, OwnedMenuItem};
use std::sync::Arc;

#[test]
fn template_shapes_are_checked_before_native_allocation() {
    for (width, height, bytes) in [
        (0, 1, 0),
        (1, 0, 0),
        (257, 1, 1028),
        (1, 257, 1028),
        (1, 1, 3),
        (1, 1, 5),
        (u32::MAX, u32::MAX, 0),
    ] {
        assert!(MenuIcon::from_rgba(width, height, vec![0; bytes].into()).is_none());
    }
    let icon = MenuIcon::from_rgba(256, 256, vec![0; 256 * 256 * 4].into()).unwrap();
    assert_eq!(
        (icon.width(), icon.height(), icon.rgba().len()),
        (256, 256, 256 * 256 * 4)
    );
}

#[test]
fn owned_menu_clones_keep_artwork_without_changing_command_state_and_release_it() {
    let data: Arc<[u8]> = vec![0, 0, 0, 191].into();
    let weak = Arc::downgrade(&data);
    let icon = MenuIcon::from_rgba(1, 1, data).unwrap();
    let menu = Menu::new("Root")
        .items([
            MenuItem::action("Command", NoAction)
                .checked(true)
                .disabled(true)
                .icon(Some(icon.clone())),
            MenuItem::submenu(Menu::new("Nested")).icon(Some(icon)),
        ])
        .owned();
    let clone = menu.clone();
    drop(menu);
    let OwnedMenuItem::Action {
        name,
        checked,
        disabled,
        icon,
        ..
    } = &clone.items[0]
    else {
        panic!("action")
    };
    assert_eq!(name, "Command");
    assert!(*checked && *disabled);
    assert_eq!(icon.as_ref().unwrap().rgba(), &[0, 0, 0, 191]);
    let OwnedMenuItem::Submenu(nested) = &clone.items[1] else {
        panic!("submenu")
    };
    assert_eq!(nested.icon.as_ref().unwrap().rgba(), &[0, 0, 0, 191]);
    drop(clone);
    assert!(weak.upgrade().is_none());
}
