//! Resolve framework defaults once per app; explicit fonts stay application-owned.
use gpui::{App, Global, SharedString};
use gpui_base::{Theme, TypographyTokens};

#[cfg(target_os = "macos")]
const ALTERNATES: &[&str] = &["Monaco", "Courier New"];
#[cfg(not(target_os = "macos"))]
const ALTERNATES: &[&str] = &["Noto Sans Mono", "Liberation Mono", "Ubuntu Mono"];

struct Initialized;
impl Global for Initialized {}

fn resolve(
    current: SharedString,
    default: &str,
    alternates: &[&str],
    available: impl FnOnce() -> Vec<String>,
) -> SharedString {
    if current.as_ref() != default {
        return current;
    }
    let available = available();
    std::iter::once(default)
        .chain(alternates.iter().copied())
        .find(|candidate| available.iter().any(|name| name == candidate))
        .unwrap_or(".SystemUIFont")
        .to_owned()
        .into()
}

fn init_with(cx: &mut App, available: impl FnOnce(&App) -> Vec<String>) {
    if cx.has_global::<Initialized>() {
        return;
    }
    let default = TypographyTokens::default().mono;
    let family = resolve(monospace(cx), &default, ALTERNATES, || available(cx));
    Theme::global_mut(cx).tokens.typography.mono = family;
    cx.set_global(Initialized);
}

pub(crate) fn init(cx: &mut App) {
    init_with(cx, |cx| cx.text_system().all_font_names());
}

pub(crate) fn monospace(cx: &App) -> SharedString {
    cx.try_global::<Theme>()
        .map(|theme| theme.tokens.typography.mono.clone())
        .unwrap_or_else(|| TypographyTokens::default().mono)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_prefer_default_then_platform_order_over_enumeration_order() {
        for (default, alternatives) in [
            ("Menlo", &["Monaco", "Courier New"][..]),
            (
                "DejaVu Sans Mono",
                &["Noto Sans Mono", "Liberation Mono", "Ubuntu Mono"][..],
            ),
        ] {
            let names = || alternatives.iter().rev().map(|s| s.to_string()).collect();
            assert_eq!(
                resolve(default.to_owned().into(), default, alternatives, names),
                alternatives[0]
            );
            assert_eq!(
                resolve(default.to_owned().into(), default, alternatives, || {
                    let mut available = names();
                    available.push(default.to_owned());
                    available
                }),
                default
            );
            assert_eq!(
                resolve(default.to_owned().into(), default, alternatives, || vec![
                    alternatives.last().unwrap().to_string()
                ]),
                *alternatives.last().unwrap()
            );
        }
    }

    #[test]
    fn missing_monospace_candidates_use_system_family() {
        for available in [vec![], vec!["Arial".to_owned(), ".ZedMono".to_owned()]] {
            assert_eq!(
                resolve("Menlo".into(), "Menlo", &["Monaco"], || available),
                ".SystemUIFont"
            );
        }
    }

    #[test]
    fn explicit_family_is_preserved_without_enumeration() {
        assert_eq!(
            resolve("Embedded App Mono".into(), "Menlo", &["Monaco"], || {
                panic!("explicit family must not be probed")
            }),
            "Embedded App Mono"
        );
    }

    #[cfg(feature = "native-image-tests")]
    #[test]
    fn initialization_is_per_app_and_preserves_existing_theme_fields() {
        let app = gpui::TestAppContext::single();
        app.update(|cx| {
            let theme = Theme::global_mut(cx);
            theme.tokens.typography.sans = "Application Sans".into();
            let foreground = theme.tokens.colors.foreground;
            init_with(cx, |_| Vec::new());
            assert_eq!(monospace(cx), ".SystemUIFont");
            assert_eq!(Theme::global(cx).tokens.typography.sans, "Application Sans");
            assert_eq!(Theme::global(cx).tokens.colors.foreground, foreground);
            init_with(cx, |_| panic!("second initialization must not enumerate"));
        });
        let other = gpui::TestAppContext::single();
        other.update(|cx| {
            let default = TypographyTokens::default().mono;
            init_with(cx, |_| vec![default.to_string()]);
            assert_eq!(monospace(cx), default);
        });
        let configured = gpui::TestAppContext::single();
        configured.update(|cx| {
            Theme::global_mut(cx).tokens.typography.mono = "Embedded App Mono".into();
            init_with(cx, |_| panic!("configured theme must not enumerate"));
            assert_eq!(monospace(cx), "Embedded App Mono");
        });
    }
}
