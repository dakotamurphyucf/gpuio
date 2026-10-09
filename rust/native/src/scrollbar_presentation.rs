//! Pure state cascade for scrollbar paint. Resolution reads a supplied palette;
//! it never changes the global theme or imports upstream timers/scroll handles.
use crate::scrollbar_geometry::Style;
use gpuio_protocol::{
    scrollbar::{Appearance, Config, Thumb, Track},
    v1::{Color, Fill},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interaction {
    Rest,
    TrackHover,
    ThumbHover,
    Pressed,
}

#[derive(Clone, Debug)]
pub struct Parts {
    pub track_background: u32,
    pub track_border: u32,
    pub thumb_background: Fill,
    pub geometry: Style,
}
#[derive(Clone, Debug)]
pub struct Resolved {
    states: [Parts; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidConfig;

impl Resolved {
    /// The foreground is packed RGBA. Default thumb alpha replaces its alpha,
    /// matching the pinned base palette convention; explicit fills are exact.
    pub fn new(config: &Config, foreground: u32) -> Result<Self, InvalidConfig> {
        if !config.is_valid() {
            return Err(InvalidConfig);
        }
        let a = &config.appearance;
        let states = [
            Interaction::Rest,
            Interaction::TrackHover,
            Interaction::ThumbHover,
            Interaction::Pressed,
        ]
        .map(|interaction| parts(a, interaction, foreground));
        let envelope = states
            .iter()
            .map(|parts| parts.geometry.track_width)
            .fold(0., f64::max);
        Ok(Self {
            states: states.map(|mut parts| {
                parts.geometry.envelope_width = envelope;
                parts
            }),
        })
    }

    pub fn parts(&self, interaction: Interaction) -> &Parts {
        &self.states[match interaction {
            Interaction::Rest => 0,
            Interaction::TrackHover => 1,
            Interaction::ThumbHover => 2,
            Interaction::Pressed => 3,
        }]
    }
}

fn parts(a: &Appearance, interaction: Interaction, foreground: u32) -> Parts {
    let (track, thumb, expanded) = match interaction {
        Interaction::Rest => (&a.track, &a.thumb, false),
        Interaction::TrackHover => (&a.track_hover, &a.thumb, false),
        Interaction::ThumbHover => (&a.track_hover, &a.thumb_hover, true),
        Interaction::Pressed => (&a.track_pressed, &a.thumb_pressed, true),
    };
    let Track {
        background,
        border,
        width,
    } = track;
    let Thumb {
        background: thumb_background,
        width: thumb_width,
        inset,
        radius,
        min_length,
    } = thumb;
    let default_thumb = ((foreground & 0xffff_ff00) | if expanded { 140 } else { 89 }) as i64;
    Parts {
        track_background: background.or(a.track.background).unwrap_or(0) as u32,
        track_border: border.or(a.track.border).unwrap_or(0) as u32,
        thumb_background: thumb_background
            .as_ref()
            .or(a.thumb.background.as_ref())
            .cloned()
            .unwrap_or(Fill::Solid(Color::Rgba(default_thumb))),
        geometry: Style {
            // Set to the same maximum across every state after resolution.
            envelope_width: 0.,
            track_width: width.or(a.track.width).unwrap_or(16.),
            thumb_width: thumb_width
                .or(a.thumb.width)
                .unwrap_or(if expanded { 8. } else { 6. }),
            inset: inset.or(a.thumb.inset).unwrap_or(4.),
            radius: radius.or(a.thumb.radius).unwrap_or(0.),
            min_length: min_length.or(a.thumb.min_length).unwrap_or(48.),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use gpuio_protocol::scrollbar::{Axis, Mode, Motion};

    fn config() -> Config {
        Config {
            label: "Transcript".into(),
            axis: Axis::Both,
            mode: Mode::Scrolling,
            appearance: Appearance::default(),
            motion: Motion::default(),
        }
    }

    #[test]
    fn defaults_follow_palette_and_hover_only_expands_the_thumb() {
        for foreground in [0x112233ff, 0xeeeeee80] {
            let resolved = Resolved::new(&config(), foreground).unwrap();
            for state in [
                Interaction::Rest,
                Interaction::TrackHover,
                Interaction::ThumbHover,
                Interaction::Pressed,
            ] {
                let p = resolved.parts(state);
                let expanded = matches!(state, Interaction::ThumbHover | Interaction::Pressed);
                assert_eq!(p.track_background, 0);
                assert_eq!(p.track_border, 0);
                assert_eq!(p.geometry.envelope_width, 16.);
                assert_eq!(p.geometry.thumb_width, if expanded { 8. } else { 6. });
                assert_eq!(
                    p.thumb_background,
                    Fill::Solid(Color::Rgba(
                        ((foreground & 0xffffff00) | if expanded { 140 } else { 89 }) as i64
                    ))
                );
            }
        }
    }

    #[test]
    fn pressed_inherits_base_not_hover_and_transparent_is_an_override() {
        let mut c = config();
        c.appearance.track.background = Some(0x112233ff);
        c.appearance.track.width = Some(20.);
        c.appearance.track_hover.background = Some(0);
        c.appearance.track_hover.width = Some(40.);
        c.appearance.thumb.width = Some(5.);
        c.appearance.thumb.radius = Some(3.);
        c.appearance.thumb_hover.width = Some(10.);
        c.appearance.thumb_hover.radius = Some(8.);
        c.appearance.thumb_hover.background = Some(Fill::Solid(Color::Rgba(0)));
        let p = Resolved::new(&c, 0xffffffff).unwrap();
        let hovered = p.parts(Interaction::ThumbHover);
        assert_eq!(hovered.track_background, 0);
        assert_eq!(hovered.geometry.thumb_width, 10.);
        assert_eq!(hovered.thumb_background, Fill::Solid(Color::Rgba(0)));
        let pressed = p.parts(Interaction::Pressed);
        assert_eq!(pressed.track_background, 0x112233ff);
        assert_eq!(pressed.geometry.track_width, 20.);
        assert_eq!(pressed.geometry.thumb_width, 5.);
        assert_eq!(pressed.geometry.radius, 3.);
        assert_eq!(
            pressed.thumb_background,
            Fill::Solid(Color::Rgba(0xffffff8c))
        );
        for state in [
            Interaction::Rest,
            Interaction::TrackHover,
            Interaction::ThumbHover,
            Interaction::Pressed,
        ] {
            assert_eq!(p.parts(state).geometry.envelope_width, 40.);
        }
    }

    #[test]
    fn explicit_gradient_survives_palette_change_and_invalid_hidden_state_rejects_all() {
        let mut c = config();
        let gradient = Fill::LinearGradientIn(
            1,
            45.,
            Color::Rgba(0x112233ff),
            0.,
            Color::Rgba(0xabcdef80),
            1.,
        );
        c.appearance.thumb.background = Some(gradient.clone());
        for foreground in [0x000000ff, 0xffffffff] {
            let p = Resolved::new(&c, foreground).unwrap();
            for state in [
                Interaction::Rest,
                Interaction::TrackHover,
                Interaction::ThumbHover,
                Interaction::Pressed,
            ] {
                assert_eq!(p.parts(state).thumb_background, gradient);
            }
        }
        c.appearance.thumb_pressed.inset = Some(f64::NAN);
        assert!(Resolved::new(&c, 0).is_err());
    }
}
