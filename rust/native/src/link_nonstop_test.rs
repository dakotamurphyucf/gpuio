//! Native fallback traversal keeps non-stops as anchors inside a modal scope.
use super::*;

pub(super) fn exercise_trap(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    for (indices, order) in [
        ([30, -10, 20], [3, 4, 2]),
        ([0, 0, 0], [2, 3, 4]),
        ([20, 20, 20], [2, 3, 4]),
    ] {
        for (position, &anchor) in order.iter().enumerate() {
            apply(
                cx,
                handle,
                (2..=4)
                    .zip(indices)
                    .map(|(slot, index)| Op::SetLink(id(slot), config(slot, index)))
                    .collect(),
            );
            draw(cx, handle);
            focus(cx, handle, anchor);
            let retained = handle
                .update(cx, |view, _, _| view.buttons[&id(anchor)].focus.clone())
                .unwrap();
            apply(
                cx,
                handle,
                vec![Op::SetLink(
                    id(anchor),
                    Config {
                        tab_stop: false,
                        ..config(anchor, indices[(anchor - 2) as usize])
                    },
                )],
            );
            draw(cx, handle);
            focused(cx, handle, anchor);
            assert!(
                handle
                    .update(cx, |view, _, _| view.buttons[&id(anchor)].focus == retained)
                    .unwrap()
            );
            key(cx, handle, "tab");
            focused(cx, handle, order[(position + 1) % 3]);
            click(cx, handle, anchor + 5);
            focused(cx, handle, anchor);
            assert_eq!(presses(transport), vec![id(anchor)]);
            key(cx, handle, "shift-tab");
            focused(cx, handle, order[(position + 2) % 3]);
            assert!(presses(transport).is_empty());
        }
        apply(
            cx,
            handle,
            (2..=4)
                .zip(indices)
                .map(|(slot, index)| {
                    Op::SetLink(
                        id(slot),
                        Config {
                            tab_stop: false,
                            ..config(slot, index)
                        },
                    )
                })
                .collect(),
        );
        draw(cx, handle);
        for key_name in ["tab", "shift-tab"] {
            focus(cx, handle, order[1]);
            draw(cx, handle);
            key(cx, handle, key_name);
            handle
                .update(cx, |view, window, _| {
                    assert!(
                        view.focus
                            .borrow()
                            .handle(id(10))
                            .unwrap()
                            .is_focused(window)
                    );
                })
                .unwrap();
        }
        // The scope root is not in the entry list. Its forward/backward entry
        // chooses the first/last eligible stop, skipping non-stops at both ends.
        apply(
            cx,
            handle,
            vec![Op::SetLink(
                id(order[1]),
                config(order[1], indices[(order[1] - 2) as usize]),
            )],
        );
        draw(cx, handle);
        for key_name in ["tab", "shift-tab"] {
            handle
                .update(cx, |view, window, cx| {
                    window.focus(&view.focus.borrow().handle(id(10)).unwrap(), cx);
                })
                .unwrap();
            draw(cx, handle);
            key(cx, handle, key_name);
            focused(cx, handle, order[1]);
            // The only stop wraps onto itself, without activating it.
            key(cx, handle, key_name);
            focused(cx, handle, order[1]);
        }
        assert!(presses(transport).is_empty());
    }
    apply(
        cx,
        handle,
        vec![
            Op::SetLink(id(2), config(2, 30)),
            Op::SetLink(id(3), config(3, -10)),
            Op::SetLink(id(4), config(4, 20)),
        ],
    );
    draw(cx, handle);
    focus(cx, handle, 2);
    draw(cx, handle);
    eprintln!(
        "GPUIO_LINK_NONSTOP_OK: signed/zero/tied indices, pointer and retained policy changes, trapped forward/reverse wrap, all-nonstop fallback, missing anchor and single-stop cycles"
    );
}
