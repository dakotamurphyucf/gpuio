//! Native font measurement on the admitted chart worker, with a private cache.
#[cfg(feature = "native-canvas-tests")]
pub(crate) mod native_test;
use crate::{chart_geometry::LabelMetrics, chart_paint::Error};
use gpui::{Font, TextRun, TextSystem, WindowTextSystem, px};
use gpuio_protocol::{
    chart_data::{Contents, Data},
    chart_style::Style,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct LabelStyle {
    pub font: Font,
    pub padding: f32,
}
impl LabelStyle {
    pub fn new(mut font: Font, padding: f32) -> Self {
        // Some(empty) explicitly clears a later inherited fallback list when
        // painting an older ready plan while its replacement is pending.
        font.fallbacks.get_or_insert_with(Default::default);
        Self { font, padding }
    }
}
#[derive(Clone)]
pub struct Context {
    pub style: LabelStyle,
    pub system: Arc<TextSystem>,
}
impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.style == other.style && Arc::ptr_eq(&self.system, &other.system)
    }
}
impl Context {
    pub fn measure(
        &self,
        data: &Data,
        style: &Style,
        cancel: &AtomicBool,
    ) -> Result<Vec<Option<LabelMetrics>>, Error> {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        if !self.style.padding.is_finite()
            || self.style.padding < 0.
            || !style.is_valid()
            || data.validate().is_err()
        {
            return Err(Error::InvalidInput);
        }
        // Bounded source labels/rich lines bound cache input. It is discarded
        // after this request, not retained on a window or across revisions.
        let text_system = WindowTextSystem::new(self.system.clone());
        collect(data, style, cancel, |text, size| {
            let run = TextRun {
                len: text.len(),
                font: self.style.font.clone(),
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let line =
                text_system.shape_line(text.to_owned().into(), px(size as f32), &[run], None);
            f64::from(f32::from(line.width) + 2. * self.style.padding)
        })
    }
}
fn collect(
    data: &Data,
    style: &Style,
    cancel: &AtomicBool,
    mut width: impl FnMut(&str, f64) -> f64,
) -> Result<Vec<Option<LabelMetrics>>, Error> {
    if let Contents::Pie(slices) = &data.contents {
        let overrides: BTreeMap<_, _> = style.pie_labels.iter().map(|e| (e.slice, e)).collect();
        return slices
            .iter()
            .map(|slice| {
                if cancel.load(Ordering::Relaxed) {
                    return Err(Error::Cancelled);
                }
                let text = overrides
                    .get(&slice.id)
                    .and_then(|e| e.text.as_deref())
                    .unwrap_or(&slice.label);
                if text.is_empty() {
                    return Ok(None);
                }
                let measured = width(text, 11.);
                if !measured.is_finite() || measured < 0. {
                    return Err(Error::NativeFailure);
                }
                Ok(Some(LabelMetrics {
                    width: measured,
                    height: 18.,
                }))
            })
            .collect();
    }
    let Contents::Sankey(nodes, _) = &data.contents else {
        return Err(Error::InvalidInput);
    };
    let overrides: BTreeMap<_, _> = style
        .node_labels
        .iter()
        .map(|n| (n.node, &n.lines))
        .collect();
    nodes
        .iter()
        .map(|node| {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let mut metric = LabelMetrics {
                width: 0.,
                height: 0.,
            };
            let mut line = |text: &str, size: f64| -> Result<(), Error> {
                if cancel.load(Ordering::Relaxed) {
                    return Err(Error::Cancelled);
                }
                let measured = width(text, size);
                if !measured.is_finite() || measured < 0. {
                    return Err(Error::NativeFailure);
                }
                metric.width = metric.width.max(measured);
                metric.height += (size + 4.).max(18.);
                Ok(())
            };
            match overrides.get(&node.id) {
                Some(lines) => {
                    for value in *lines {
                        line(&value.text, value.font_size.unwrap_or(11.))?;
                    }
                }
                None => line(&node.label, 11.)?,
            }
            Ok((metric.height > 0.).then_some(metric))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::{
        chart_data::Node,
        chart_node_labels::{Line, Node as LabelNode},
    };
    #[test]
    fn metrics_follow_ids_hide_empty_overrides_and_stop_on_cancel_or_bad_native_width() {
        let data = Data {
            version: 1,
            contents: Contents::Sankey(
                [20, 10, 30]
                    .into_iter()
                    .map(|id| Node {
                        id,
                        label: format!("node {id}"),
                    })
                    .collect(),
                vec![],
            ),
        };
        let style = Style {
            node_labels: vec![
                LabelNode {
                    node: 10,
                    lines: vec![
                        Line {
                            text: "λ 👨‍👩‍👧‍👦".into(),
                            font_size: Some(32.),
                            color: None,
                        },
                        Line {
                            text: "detail".into(),
                            font_size: None,
                            color: Some(0xff0000ff),
                        },
                    ],
                },
                LabelNode {
                    node: 20,
                    lines: vec![],
                },
            ],
            ..Default::default()
        };
        let mut calls = vec![];
        let measured = collect(&data, &style, &AtomicBool::new(false), |text, size| {
            calls.push((text.to_owned(), size));
            77.
        })
        .unwrap();
        assert_eq!(
            measured,
            vec![
                None,
                Some(LabelMetrics {
                    width: 77.,
                    height: 54.
                }),
                Some(LabelMetrics {
                    width: 77.,
                    height: 18.
                })
            ]
        );
        assert_eq!(
            calls,
            vec![
                ("λ 👨‍👩‍👧‍👦".into(), 32.),
                ("detail".into(), 11.),
                ("node 30".into(), 11.)
            ]
        );
        assert_eq!(
            collect(&data, &style, &AtomicBool::new(true), |_, _| panic!(
                "cancelled work measured"
            )),
            Err(Error::Cancelled)
        );
        for bad in [f64::NAN, f64::INFINITY, -1.] {
            assert_eq!(
                collect(&data, &style, &AtomicBool::new(false), |_, _| bad),
                Err(Error::NativeFailure)
            );
        }
        let cancel = AtomicBool::new(false);
        let mut count = 0;
        assert_eq!(
            collect(&data, &style, &cancel, |_, _| {
                count += 1;
                cancel.store(true, Ordering::Relaxed);
                1.
            }),
            Err(Error::Cancelled)
        );
        assert_eq!(count, 1);
    }
    #[test]
    fn captured_font_style_is_complete_and_sensitive_to_padding() {
        let a = LabelStyle::new(Font::default(), 4.);
        assert_eq!(a.font.fallbacks, Some(Default::default()));
        assert_ne!(a, LabelStyle::new(Font::default().bold(), 4.));
        assert_ne!(a, LabelStyle::new(Font::default(), 5.));
    }
}
