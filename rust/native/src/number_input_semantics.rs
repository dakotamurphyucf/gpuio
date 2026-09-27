//! Native numeric validation supplements application field metadata. It never
//! rewrites the draft, emits a commit or interprets invalid text as zero.
use gpuio_protocol::{
    accessibility as a, number_input as n,
    numeric::{Draft, DraftError},
};
use std::sync::Arc;

fn join(parts: impl IntoIterator<Item = Option<impl AsRef<str>>>) -> Option<String> {
    let mut values = Vec::new();
    for part in parts.into_iter().flatten() {
        let value = part.as_ref();
        if !value.is_empty() && !values.iter().any(|old: &String| old == value) {
            values.push(value.to_owned());
        }
    }
    (!values.is_empty()).then(|| values.join("\n"))
}

pub(super) fn metadata(
    config: &n::Config,
    draft: &str,
    composing: bool,
    application: Option<&a::Config>,
    tooltip: Option<&str>,
) -> Arc<a::Config> {
    let field = application.and_then(|c| c.field.as_ref());
    let error = if composing {
        None
    } else {
        match Draft::parse(config.domain, draft) {
            // An unfilled field is required but not immediately marked invalid.
            // Applications can add a submit-time error through field metadata.
            Draft::Empty | Draft::Valid(_) => None,
            Draft::Incomplete => Some("Complete the number.".to_owned()),
            Draft::Invalid(DraftError::Syntax) => {
                Some("Enter a number using digits, a decimal point or an exponent.".to_owned())
            }
            Draft::Invalid(DraftError::NonFinite) => Some("Enter a finite number.".to_owned()),
            Draft::Invalid(DraftError::TooLong) => Some("The number is too long.".to_owned()),
            Draft::OutOfRange(_) => Some(format!(
                "Value will be limited to {} through {} when committed.",
                config.domain.min(),
                config.domain.max()
            )),
        }
    };
    Arc::new(a::Config {
        current: None,
        role: None,
        label: None,
        description: None,
        live: application.map_or(a::Live::Off, |c| c.live),
        field: Some(a::Field {
            label: field
                .map(|f| f.label.as_str())
                .or_else(|| application.and_then(|c| c.label.as_deref()))
                .unwrap_or(&config.label)
                .to_owned(),
            help: join([
                application.and_then(|c| c.description.as_deref()),
                field.and_then(|f| f.help.as_deref()),
                tooltip,
            ]),
            error: join([field.and_then(|f| f.error.as_deref()), error.as_deref()]),
            required: !config.allow_empty || field.is_some_and(|f| f.required),
        }),
    })
}

pub(super) fn description(metadata: &a::Config) -> Option<String> {
    metadata
        .field
        .as_ref()
        .and_then(|field| join([field.help.as_deref(), field.error.as_deref()]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> n::Config {
        n::Config {
            domain: gpuio_protocol::numeric::Domain::new(-2., 8., 0.5).unwrap(),
            label: "Temperature".into(),
            placeholder: "".into(),
            increment_label: "More".into(),
            decrement_label: "Less".into(),
            step_controls: n::StepControls::Sides,
            allow_empty: false,
            disabled: false,
            read_only: false,
            auto_focus: false,
        }
    }
    #[test]
    fn transient_numeric_feedback_preserves_application_semantics_and_composition() {
        let app = a::Config {
            role: None,
            label: None,
            description: None,
            live: a::Live::Off,
            current: None,
            field: Some(a::Field {
                label: "Sampling temperature".into(),
                help: Some("Workspace setting\nAdjust generation".into()),
                error: Some("Server unavailable".into()),
                required: false,
            }),
        };
        let composed = metadata(
            &config(),
            "1e-",
            false,
            Some(&app),
            Some("Workspace setting\nAdjust generation"),
        );
        let field = composed.field.as_ref().unwrap();
        assert_eq!(field.label, "Sampling temperature");
        assert_eq!(
            field.help.as_deref(),
            Some("Workspace setting\nAdjust generation")
        );
        assert_eq!(
            field.error.as_deref(),
            Some("Server unavailable\nComplete the number.")
        );
        assert!(field.required);
        assert!(app.is_valid());
        assert_eq!(composed.live, a::Live::Off);
        let plain = a::Config {
            role: None,
            label: Some("Fallback".into()),
            description: Some("Workspace setting".into()),
            live: a::Live::Polite,
            current: None,
            field: None,
        };
        assert!(plain.is_valid());
        let plain = metadata(
            &config(),
            "1.5",
            false,
            Some(&plain),
            Some("Adjust generation"),
        );
        assert_eq!(plain.live, a::Live::Polite);
        assert_eq!(plain.field.as_ref().unwrap().label, "Fallback");
        assert_eq!(
            description(&plain).as_deref(),
            Some("Workspace setting\nAdjust generation")
        );
        let composed = metadata(&config(), "に", true, Some(&app), None);
        assert_eq!(
            composed.field.as_ref().unwrap().error.as_deref(),
            Some("Server unavailable")
        );
        for draft in ["", "1.5"] {
            let value = metadata(&config(), draft, false, None, None);
            assert!(value.field.as_ref().unwrap().error.is_none());
            assert!(value.field.as_ref().unwrap().required);
        }
        for draft in ["1e-", "é", "1e999", "99"] {
            let value = metadata(&config(), draft, false, None, None);
            assert!(value.field.as_ref().unwrap().error.is_some());
        }
        let optional = metadata(
            &n::Config {
                allow_empty: true,
                ..config()
            },
            "",
            false,
            None,
            None,
        );
        assert!(!optional.field.as_ref().unwrap().required);
    }
}
