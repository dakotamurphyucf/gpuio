//! Paint-only highlight washes. Matching stays on workers; geometry follows the
//! already-shaped text, including wrapping, alignment and truncation. Put this
//! underlay before StyledText so native selection decoration remains on top.
use crate::{
    highlight_jobs::Ready,
    highlight_projection::{Matches, RunKey},
};
use gpui::{
    Bounds, Hsla, Pixels, TextAlign, TextLayout, TextOverflow, WrappedLineLayout, canvas, point,
    prelude::*, px, quad, rgba, size,
};
use gpuio_protocol::highlight::Config;
use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};

#[derive(Clone, Debug, PartialEq)]
pub struct Wash {
    pub bytes: Range<usize>,
    pub color: Hsla,
    pub radius: f32,
    pub active: bool,
}
/// Resolution is presentation-only. Virtualized offsets are compared by
/// subtraction, never added to an ordinal near the signed-integer maximum.
#[derive(Clone)]
pub struct Paint {
    washes: Arc<[Wash]>,
    ready: Arc<Ready>,
}
impl Paint {
    pub fn is_empty(&self) -> bool {
        self.washes.is_empty()
    }
}
pub fn resolve(ready: Arc<Ready>, key: RunKey, config: &Config) -> Paint {
    Paint {
        washes: resolve_matches(&ready.matches, key, config),
        ready,
    }
}
fn resolve_matches(matches: &Matches, key: RunKey, config: &Config) -> Arc<[Wash]> {
    matches
        .spans
        .get(&key)
        .into_iter()
        .flatten()
        .filter_map(|span| {
            let spec = config.0.get(span.spec_index)?;
            let active = spec.active_index.is_some_and(|index| {
                index >= spec.match_index_offset && index - spec.match_index_offset == span.ordinal
            });
            Some(Wash {
                bytes: span.bytes.clone(),
                color: rgba(if active {
                    spec.appearance.active_color
                } else {
                    spec.appearance.color
                } as u32)
                .into(),
                radius: spec.appearance.radius as f32,
                active,
            })
        })
        .collect::<Vec<_>>()
        .into()
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Mapping {
    source: Range<usize>,
    displayed: usize,
}
/// Never search for the query in displayed text. Map only retained source slices;
/// synthetic ellipsis text cannot acquire a match or contribute an ordinal.
fn mapping(source: &str, displayed: &str, overflow: Option<&TextOverflow>) -> Vec<Mapping> {
    if source == displayed {
        return vec![Mapping {
            source: 0..source.len(),
            displayed: 0,
        }];
    }
    match overflow {
        Some(TextOverflow::Truncate(affix)) => displayed
            .strip_suffix(affix.as_ref())
            .filter(|s| source.starts_with(s))
            .map(|s| {
                vec![Mapping {
                    source: 0..s.len(),
                    displayed: 0,
                }]
            })
            .unwrap_or_default(),
        Some(TextOverflow::TruncateStart(affix)) => displayed
            .strip_prefix(affix.as_ref())
            .filter(|s| source.ends_with(s))
            .map(|s| {
                vec![Mapping {
                    source: source.len() - s.len()..source.len(),
                    displayed: affix.len(),
                }]
            })
            .unwrap_or_default(),
        Some(TextOverflow::TruncateMiddle(affix)) if !affix.is_empty() => displayed
            .match_indices(affix.as_ref())
            .find_map(|(index, _)| {
                let prefix = &displayed[..index];
                let suffix = &displayed[index + affix.len()..];
                (prefix.len() + suffix.len() <= source.len()
                    && source.starts_with(prefix)
                    && source.ends_with(suffix))
                .then(|| {
                    vec![
                        Mapping {
                            source: 0..prefix.len(),
                            displayed: 0,
                        },
                        Mapping {
                            source: source.len() - suffix.len()..source.len(),
                            displayed: index + affix.len(),
                        },
                    ]
                })
            })
            .unwrap_or_default(),
        None | Some(TextOverflow::TruncateMiddle(_)) => {
            if source.starts_with(displayed) {
                vec![Mapping {
                    source: 0..displayed.len(),
                    displayed: 0,
                }]
            } else {
                vec![]
            }
        }
    }
}
struct Line {
    layout: Arc<WrappedLineLayout>,
    offset: usize,
    row: usize,
    // Only one entry per shaped font run, not per byte/glyph. Font layout data
    // stays in GPUI's Arc; repeated matches do not scan all preceding glyphs.
    run_ends: Vec<(usize, usize)>,
    monotonic: bool,
}
impl Line {
    fn new(layout: Arc<WrappedLineLayout>, offset: usize, row: usize) -> Self {
        let mut previous = None;
        let mut monotonic = true;
        let mut run_ends = Vec::new();
        for (index, run) in layout.unwrapped_layout.runs.iter().enumerate() {
            for glyph in &run.glyphs {
                if previous.is_some_and(|p| p > glyph.index) {
                    monotonic = false;
                }
                previous = Some(glyph.index);
            }
            if let Some(last) = run.glyphs.last() {
                run_ends.push((last.index, index));
            }
        }
        Self {
            layout,
            offset,
            row,
            run_ends,
            monotonic,
        }
    }
    fn x(&self, index: usize) -> Pixels {
        let layout = &self.layout.unwrapped_layout;
        if !self.monotonic {
            return layout.x_for_index(index);
        }
        let run = self.run_ends.partition_point(|(end, _)| *end < index);
        let Some((_, run)) = self.run_ends.get(run) else {
            return layout.width;
        };
        let glyphs = &layout.runs[*run].glyphs;
        glyphs[glyphs.partition_point(|glyph| glyph.index < index)]
            .position
            .x
    }
    fn boundary(&self, row: usize) -> (usize, Pixels) {
        if row == 0 {
            return (0, px(0.));
        }
        self.layout.wrap_boundaries.get(row - 1).map_or(
            (self.layout.len(), self.layout.unwrapped_layout.width),
            |b| {
                let glyph = &self.layout.unwrapped_layout.runs[b.run_ix].glyphs[b.glyph_ix];
                (glyph.index, glyph.position.x)
            },
        )
    }
    fn rectangles(
        &self,
        range: Range<usize>,
        bounds: Bounds<Pixels>,
        line_height: Pixels,
        align: TextAlign,
        mask: Bounds<Pixels>,
        mut emit: impl FnMut(Bounds<Pixels>),
    ) {
        let start = range.start.max(self.offset) - self.offset;
        let end = range
            .end
            .min(self.offset + self.layout.len())
            .saturating_sub(self.offset);
        if start >= end {
            return;
        }
        let first = if self.monotonic {
            self.layout.wrap_boundaries.partition_point(|b| {
                self.layout.unwrapped_layout.runs[b.run_ix].glyphs[b.glyph_ix].index <= start
            })
        } else {
            0
        };
        let visible = (((mask.top() - bounds.top()) / line_height).floor().max(0.) as usize)
            .saturating_sub(self.row);
        for row in first.max(visible)..=self.layout.wrap_boundaries.len() {
            let y = bounds.top() + line_height * (self.row + row);
            if y >= mask.bottom() {
                break;
            }
            if y + line_height <= mask.top() {
                continue;
            }
            let (from, x_from) = self.boundary(row);
            let (to, x_to) = self.boundary(row + 1);
            if from >= end {
                break;
            }
            if to <= start {
                continue;
            }
            let width = x_to - x_from;
            let inset = match align {
                TextAlign::Left => px(0.),
                TextAlign::Center => (bounds.size.width - width) / 2.,
                TextAlign::Right => bounds.size.width - width,
            };
            let a = self.x(start.max(from)) - x_from;
            let b = self.x(end.min(to)) - x_from;
            if a == b {
                continue;
            }
            emit(Bounds::new(
                point(bounds.left() + inset + a.min(b), y),
                size((b - a).abs(), line_height),
            ));
        }
    }
}
#[derive(Default)]
pub struct Cache {
    // Retained geometry/source readers keep the original worker admission charge.
    ready: Option<Arc<Ready>>,
    source: Option<Arc<str>>,
    overflow: Option<TextOverflow>,
    lines: Vec<Line>,
    mapping: Vec<Mapping>,
}
pub type SharedCache = Rc<RefCell<Cache>>;
impl Cache {
    fn update(&mut self, source: &Arc<str>, layout: &TextLayout, overflow: Option<TextOverflow>) {
        let lines = layout.line_layouts();
        if self.source.as_ref().is_some_and(|s| Arc::ptr_eq(s, source))
            && self.overflow == overflow
            && self.lines.len() == lines.len()
            && self
                .lines
                .iter()
                .zip(&lines)
                .all(|(a, b)| Arc::ptr_eq(&a.layout, b))
        {
            return;
        }
        self.mapping = mapping(source, &layout.text(), overflow.as_ref());
        self.source = Some(source.clone());
        self.overflow = overflow;
        let mut offset = 0;
        let mut row = 0;
        self.lines = lines
            .into_iter()
            .map(|layout| {
                let len = layout.len();
                let rows = layout.wrap_boundaries.len() + 1;
                let line = Line::new(layout, offset, row);
                offset += len + 1;
                row += rows;
                line
            })
            .collect();
    }
    fn rectangles(
        &self,
        wash: &Wash,
        bounds: Bounds<Pixels>,
        height: Pixels,
        align: TextAlign,
        mask: Bounds<Pixels>,
        mut emit: impl FnMut(Bounds<Pixels>),
    ) {
        for mapping in &self.mapping {
            let start = wash.bytes.start.max(mapping.source.start);
            let end = wash.bytes.end.min(mapping.source.end);
            if start >= end {
                continue;
            }
            let range = start - mapping.source.start + mapping.displayed
                ..end - mapping.source.start + mapping.displayed;
            let first = self
                .lines
                .partition_point(|line| line.offset + line.layout.len() <= range.start);
            for line in self.lines[first..]
                .iter()
                .take_while(|line| line.offset < range.end)
            {
                line.rectangles(range.clone(), bounds, height, align, mask, &mut emit);
            }
        }
    }
}
pub fn underlay(
    source: Arc<str>,
    layout: TextLayout,
    paint: Paint,
    cache: SharedCache,
) -> gpui::AnyElement {
    canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            let style = window.text_style();
            let mut cache = cache.borrow_mut();
            cache.update(&source, &layout, style.text_overflow.clone());
            cache.ready = Some(paint.ready.clone());
            for wash in paint.washes.iter() {
                cache.rectangles(
                    wash,
                    layout.bounds(),
                    layout.line_height(),
                    style.text_align,
                    window.content_mask().bounds,
                    |bounds| {
                        window.paint_quad(quad(
                            bounds,
                            px(wash.radius),
                            wash.color,
                            px(0.),
                            gpui::transparent_black(),
                            Default::default(),
                        ));
                    },
                );
            }
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight_projection::{Count, Span};
    use gpui::{FontId, GlyphId, LineLayout, ShapedGlyph, ShapedRun, WrapBoundary};
    use gpuio_protocol::{
        NodeId,
        highlight::{Appearance, Query, Spec},
    };
    use std::collections::BTreeMap;

    #[test]
    fn truncation_maps_original_utf8_slices_without_painting_the_affix() {
        let source = "α beta γ";
        assert_eq!(
            mapping(source, "α b…", Some(&TextOverflow::Truncate("…".into()))),
            vec![Mapping {
                source: 0..4,
                displayed: 0
            }]
        );
        assert_eq!(
            mapping(
                source,
                "…ta γ",
                Some(&TextOverflow::TruncateStart("…".into()))
            ),
            vec![Mapping {
                source: 5..10,
                displayed: 3
            }]
        );
        assert_eq!(
            mapping(
                source,
                "α…γ",
                Some(&TextOverflow::TruncateMiddle("…".into()))
            ),
            vec![
                Mapping {
                    source: 0..2,
                    displayed: 0
                },
                Mapping {
                    source: 8..10,
                    displayed: 5
                }
            ]
        );
        assert_eq!(
            mapping("ab…cd", "ab…cd", Some(&TextOverflow::Truncate("…".into()))),
            vec![Mapping {
                source: 0..7,
                displayed: 0
            }]
        );
        assert!(mapping(source, "not a retained slice", None).is_empty());
        assert_eq!(
            mapping(source, "…", Some(&TextOverflow::Truncate("…".into()))),
            vec![Mapping {
                source: 0..0,
                displayed: 0
            }]
        );
        // Equal byte lengths do not prove the text was not truncated.
        assert_eq!(
            mapping("café", "ca…", Some(&TextOverflow::Truncate("…".into()))),
            vec![Mapping {
                source: 0..2,
                displayed: 0
            }]
        );
    }
    fn line(indices: &[usize], wraps: &[usize], len: usize, offset: usize, row: usize) -> Line {
        let glyphs = indices
            .iter()
            .enumerate()
            .map(|(i, index)| ShapedGlyph {
                id: GlyphId(0),
                position: point(px(i as f32 * 10.), px(0.)),
                index: *index,
                is_emoji: false,
            })
            .collect();
        Line::new(
            Arc::new(WrappedLineLayout {
                unwrapped_layout: Arc::new(LineLayout {
                    font_size: px(12.),
                    width: px(indices.len() as f32 * 10.),
                    ascent: px(10.),
                    descent: px(2.),
                    runs: vec![ShapedRun {
                        font_id: FontId(0),
                        glyphs,
                    }],
                    len,
                }),
                wrap_boundaries: wraps
                    .iter()
                    .map(|glyph_ix| WrapBoundary {
                        run_ix: 0,
                        glyph_ix: *glyph_ix,
                    })
                    .collect(),
                wrap_width: Some(px(30.)),
            }),
            offset,
            row,
        )
    }
    fn rects(
        line: &Line,
        range: Range<usize>,
        align: TextAlign,
        mask: Bounds<Pixels>,
    ) -> Vec<(f32, f32, f32, f32)> {
        let mut out = Vec::new();
        line.rectangles(
            range,
            Bounds::new(point(px(10.), px(20.)), size(px(60.), px(100.))),
            px(15.),
            align,
            mask,
            |b| {
                out.push((
                    b.origin.x.as_f32(),
                    b.origin.y.as_f32(),
                    b.size.width.as_f32(),
                    b.size.height.as_f32(),
                ))
            },
        );
        out
    }
    fn mask() -> Bounds<Pixels> {
        Bounds::new(point(px(0.), px(0.)), size(px(100.), px(200.)))
    }
    #[test]
    fn wraps_have_downstream_start_affinity_and_alignment_per_visual_row() {
        let l = line(&[0, 1, 2, 3, 4, 5], &[3], 6, 0, 0);
        assert_eq!(
            rects(&l, 1..5, TextAlign::Left, mask()),
            vec![(20., 20., 20., 15.), (10., 35., 20., 15.)]
        );
        assert_eq!(
            rects(&l, 3..5, TextAlign::Center, mask()),
            vec![(25., 35., 20., 15.)]
        );
        assert_eq!(
            rects(&l, 0..3, TextAlign::Right, mask()),
            vec![(40., 20., 30., 15.)]
        );
        let clipped = Bounds::new(point(px(0.), px(35.)), size(px(100.), px(15.)));
        assert_eq!(
            rects(&l, 0..6, TextAlign::Left, clipped),
            vec![(10., 35., 30., 15.)]
        );
    }
    #[test]
    fn unicode_cluster_indices_and_hard_line_offsets_use_displayed_bytes() {
        // Three scalars with byte lengths 2, 1, 4; no byte stepping.
        let l = line(&[0, 2, 3], &[2], 7, 9, 2);
        assert_eq!(
            rects(&l, 11..16, TextAlign::Left, mask()),
            vec![(20., 50., 10., 15.), (10., 65., 10., 15.)]
        );
        assert_eq!(l.x(7), px(30.));
    }
    #[test]
    fn resolving_offsets_near_i64_max_does_not_overflow_or_shift_other_specs() {
        let key = RunKey {
            node: NodeId::from_parts(0, 1).unwrap(),
            fragment: 0,
        };
        let matches = Matches {
            counts: vec![Count {
                total: 2,
                stored: 2,
            }],
            spans: BTreeMap::from([(
                key,
                vec![
                    Span {
                        spec_index: 0,
                        ordinal: 0,
                        bytes: 0..1,
                    },
                    Span {
                        spec_index: 0,
                        ordinal: 1,
                        bytes: 2..3,
                    },
                ],
            )]),
        };
        let mut config = Config(vec![Spec {
            query: Some(Query {
                text: "a".into(),
                case_sensitive: true,
                whole_word: false,
            }),
            ranges: vec![],
            appearance: Appearance {
                color: 0x11223344,
                active_color: 0x55667788,
                radius: 3.,
            },
            active_index: Some(i64::MAX),
            match_index_offset: i64::MAX,
        }]);
        let washes = resolve_matches(&matches, key, &config);
        assert!(washes[0].active);
        assert!(!washes[1].active);
        assert_eq!(washes[0].radius, 3.);
        assert_eq!(washes[0].color, rgba(0x55667788).into());
        config.0[0].active_index = Some(i64::MAX - 1);
        assert!(
            resolve_matches(&matches, key, &config)
                .iter()
                .all(|w| !w.active)
        );
    }

    #[test]
    fn retired_paint_and_geometry_readers_keep_worker_reservations() {
        use crate::{
            highlight_jobs as jobs,
            highlight_projection::{Group, Kind, Projection, Run, Source},
        };
        let key = RunKey {
            node: NodeId::from_parts(0, 1).unwrap(),
            fragment: 0,
        };
        let config = Arc::new(Config(vec![Spec {
            query: Some(Query {
                text: "a".into(),
                case_sensitive: true,
                whole_word: false,
            }),
            ranges: vec![],
            appearance: Appearance {
                color: 1,
                active_color: 2,
                radius: 2.,
            },
            active_index: None,
            match_index_offset: 0,
        }]));
        let source = Arc::new(
            Projection::new(vec![Group {
                kind: Kind::Ordinary,
                runs: vec![Run {
                    key,
                    source: Source::Text("a".into()),
                }],
            }])
            .unwrap(),
        );
        let mut pool = jobs::Pool::default();
        let handle = pool.request(source, config.clone()).unwrap();
        let work = pool.next_work().unwrap();
        assert!(pool.complete(work.run()));
        let jobs::Status::Ready(ready) = handle.status() else {
            panic!("ready")
        };
        let paint = resolve(ready, key, &config);
        let cache = Cache {
            ready: Some(paint.ready.clone()),
            ..Default::default()
        };
        handle.close();
        assert!(pool.reserved_bytes() > 0);
        drop(paint);
        assert!(
            pool.reserved_bytes() > 0,
            "last geometry reader still owns source"
        );
        drop(cache);
        assert_eq!(pool.reserved_bytes(), 0);
    }
}

#[cfg(feature = "native-image-tests")]
#[path = "highlight_paint_test.rs"]
pub(crate) mod native_test;
