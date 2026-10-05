//! Intrinsic sizing of overlapping fixed-size children in the pinned layout engine.
use taffy::prelude::*;

#[test]
fn negative_margins_do_not_erase_fixed_children_intrinsic_size() {
    // Fixed min/max sizes make the final layout independent of shrink policy.
    // Exercise both axes and intrinsic constraints, including subpixel bases.
    for direction in [FlexDirection::Row, FlexDirection::Column] {
        for size in [0.5, 16., 48., 80.] {
            for shrink in [0., 0.01, 0.25, 1., 2.] {
                for margin in [-size * 0.6, -size * 0.3, 0., size * 0.3] {
                    for available in [AvailableSpace::MinContent, AvailableSpace::MaxContent] {
                        let mut tree: TaffyTree<()> = TaffyTree::new();
                        tree.disable_rounding();
                        let mut children = Vec::new();
                        for index in 0..3 {
                            let offset = if index == 0 { 0. } else { margin };
                            children.push(
                                tree.new_leaf(Style {
                                    size: Size {
                                        width: length(size),
                                        height: length(size),
                                    },
                                    min_size: Size {
                                        width: length(size),
                                        height: length(size),
                                    },
                                    max_size: Size {
                                        width: length(size),
                                        height: length(size),
                                    },
                                    flex_grow: 0.,
                                    flex_shrink: shrink,
                                    margin: match direction {
                                        FlexDirection::Row => Rect {
                                            left: length(offset),
                                            ..Rect::zero()
                                        },
                                        _ => Rect {
                                            top: length(offset),
                                            ..Rect::zero()
                                        },
                                    },
                                    ..Default::default()
                                })
                                .unwrap(),
                            );
                        }
                        let group = tree
                            .new_with_children(
                                Style {
                                    flex_direction: direction,
                                    flex_shrink: 0.,
                                    ..Default::default()
                                },
                                &children,
                            )
                            .unwrap();
                        let root = tree
                            .new_with_children(
                                Style {
                                    flex_direction: direction,
                                    ..Default::default()
                                },
                                &[group],
                            )
                            .unwrap();
                        tree.compute_layout(
                            root,
                            Size {
                                width: available,
                                height: available,
                            },
                        )
                        .unwrap();
                        let measured = tree.layout(group).unwrap().size;
                        let main = if direction == FlexDirection::Row {
                            measured.width
                        } else {
                            measured.height
                        };
                        let expected = 3. * size + 2. * margin;
                        assert!(
                            (main - expected).abs() < 0.001,
                            "{direction:?}: size={size}, shrink={shrink}, margin={margin}, available={available:?}: got {main}, expected {expected}"
                        );
                        for (index, child) in children.iter().enumerate() {
                            let layout = tree.layout(*child).unwrap();
                            let position = if direction == FlexDirection::Row {
                                layout.location.x
                            } else {
                                layout.location.y
                            };
                            assert!((position - index as f32 * (size + margin)).abs() < 0.001);
                            assert!((layout.size.width - size).abs() < 0.001);
                            assert!((layout.size.height - size).abs() < 0.001);
                        }
                    }
                }
            }
        }
    }
}
