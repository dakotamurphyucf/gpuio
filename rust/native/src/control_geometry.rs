//! Bounded local coordinates for scalable checkable artwork, independent of GPUI.
use gpuio_protocol::control_appearance::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Checkbox,
    Radio,
    Switch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    None,
    Check { points: [[f64; 2]; 3], stroke: f64 },
    Dash(Rect),
    Dot(Rect),
    Thumb(Rect),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub mark: Mark,
}

impl Geometry {
    /// Invalid geometry is rejected before generating coordinates.
    pub fn new(kind: Kind, config: &Config, checked: bool, mixed: bool) -> Option<Self> {
        if !config.valid_geometry() {
            return None;
        }
        let h = config.size;
        let scale = h / 18.;
        let rect = |x, y, width, height| Rect {
            x: x * scale,
            y: y * scale,
            width: width * scale,
            height: height * scale,
        };
        let width = if kind == Kind::Switch {
            config.switch_width
        } else {
            h
        };
        let mark = match kind {
            Kind::Switch => Mark::Thumb(Rect {
                x: if checked {
                    width - h / 6. - h * (2. / 3.)
                } else {
                    h / 6.
                },
                y: h / 6.,
                width: h * (2. / 3.),
                height: h * (2. / 3.),
            }),
            Kind::Radio if checked => Mark::Dot(rect(5., 5., 8., 8.)),
            Kind::Radio => Mark::None,
            Kind::Checkbox if mixed => Mark::Dash(rect(4., 8., 10., 2.)),
            Kind::Checkbox if checked => Mark::Check {
                points: [
                    [4. * scale, 9. * scale],
                    [8. * scale, 13. * scale],
                    [14. * scale, 5. * scale],
                ],
                stroke: 2. * scale,
            },
            Kind::Checkbox => Mark::None,
        };
        Some(Self {
            width,
            height: h,
            radius: if kind == Kind::Checkbox {
                h / 6.
            } else {
                h / 2.
            },
            mark,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_coordinates_preserve_existing_artwork() {
        let config = Config::default();
        assert_eq!(
            Geometry::new(Kind::Checkbox, &config, true, false)
                .unwrap()
                .mark,
            Mark::Check {
                points: [[4., 9.], [8., 13.], [14., 5.]],
                stroke: 2.
            }
        );
        assert_eq!(
            Geometry::new(Kind::Switch, &config, true, false)
                .unwrap()
                .mark,
            Mark::Thumb(Rect {
                x: 15.,
                y: 3.,
                width: 12.,
                height: 12.
            })
        );
        assert_eq!(
            Geometry::new(Kind::Radio, &config, true, false)
                .unwrap()
                .mark,
            Mark::Dot(Rect {
                x: 5.,
                y: 5.,
                width: 8.,
                height: 8.
            })
        );
    }

    #[test]
    fn all_value_states_stay_inside_minimum_default_and_maximum_geometry() {
        for size in [8., 12., 18., 32., 128.] {
            for switch_width in [size, size * (5. / 3.), 256.] {
                let config = Config {
                    size,
                    switch_width,
                    ..Config::default()
                };
                for kind in [Kind::Checkbox, Kind::Radio, Kind::Switch] {
                    for (checked, mixed) in
                        [(false, false), (true, false), (false, true), (true, true)]
                    {
                        let geometry = Geometry::new(kind, &config, checked, mixed).unwrap();
                        let inside = |x: f64, y: f64| {
                            x.is_finite()
                                && y.is_finite()
                                && x >= 0.
                                && y >= 0.
                                && x <= geometry.width
                                && y <= geometry.height
                        };
                        match geometry.mark {
                            Mark::None => (),
                            Mark::Check { points, stroke } => {
                                assert!(stroke > 0.);
                                for [x, y] in points {
                                    assert!(
                                        inside(x - stroke / 2., y - stroke / 2.)
                                            && inside(x + stroke / 2., y + stroke / 2.)
                                    );
                                }
                            }
                            Mark::Dash(r) | Mark::Dot(r) | Mark::Thumb(r) => {
                                assert!(r.width > 0. && r.height > 0.);
                                assert!(inside(r.x, r.y) && inside(r.x + r.width, r.y + r.height));
                            }
                        }
                    }
                }
            }
        }
        for size in [f64::NAN, f64::INFINITY, -1., 0., 7.99, 128.01] {
            assert!(
                Geometry::new(
                    Kind::Checkbox,
                    &Config {
                        size,
                        ..Config::default()
                    },
                    true,
                    false
                )
                .is_none()
            );
        }
    }
}
