//! Text effects keep native generation/window ownership. Rendered elements hold
//! weak drivers, so cached paint elements never retain retired tree payloads.
use super::*;
use crate::text_shimmer_clock::{Decoration, Owner};

impl View {
    pub(super) fn sync_text_shimmers(&mut self) {
        let session = self.session.borrow();
        self.text_shimmers.retain(|id, owner| {
            let Some(node) = session.tree(self.id).and_then(|tree| tree.get(*id)) else {
                return false;
            };
            let Some(config) = &node.text_shimmer else {
                return false;
            };
            owner
                .update(node.text.clone(), **config)
                .expect("admitted text shimmer");
            // A retained but hidden branch may never construct another element.
            // Every accepted update disarms until the next eligible paint.
            owner.prepare_frame();
            true
        });
    }

    pub(super) fn text_shimmer<'a>(
        &'a mut self,
        node: &crate::tree::Node,
        window: &Window,
    ) -> Option<Decoration<'a>> {
        let config = **node.text_shimmer.as_ref()?;
        let owner = self.text_shimmers.entry(node.id).or_insert_with(|| {
            Owner::new(node.text.clone(), config, self.text_shimmer_clock.clone())
                .expect("admitted text shimmer")
        });
        let dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        let colors = if dark {
            gpui_base::theme_tokens::ColorTokens::dark()
        } else {
            gpui_base::theme_tokens::ColorTokens::light()
        };
        Some(Decoration {
            owner,
            budget: self.text_shimmer_budget.clone(),
            appearance: crate::text_shimmer_paint::Appearance {
                foreground: colors.foreground,
                background: colors.background,
                dark,
            },
        })
    }
}
