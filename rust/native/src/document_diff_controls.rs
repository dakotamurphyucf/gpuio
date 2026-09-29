//! Native-managed diff values, independent of GPUI callbacks. The presenter
//! fences source/config/handler identities before invoking these operations.
use crate::{document_diff::Diff, document_diff_projection::Projection};
use gpuio_protocol::document_diff::{Collapse, Config, File, FileKey, LineLimit, Observation};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub struct Controls {
    config: Config,
    generation: i64,
    seed: BTreeSet<FileKey>,
    overrides: BTreeMap<FileKey, bool>,
    limit: Option<i64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidConfig;

fn key(file: &crate::document_diff::File) -> FileKey {
    file.path()
        .map_or(FileKey::Unnamed, |path| FileKey::Path(path.to_owned()))
}
fn initial_limit(config: &Config) -> Option<i64> {
    match config.line_limit {
        LineLimit::Managed { initial, .. } => initial,
        LineLimit::Controlled(value) => value,
    }
}
impl Controls {
    pub fn new(config: Config, generation: i64) -> Result<Self, InvalidConfig> {
        if generation <= 0 || !config.is_valid() {
            return Err(InvalidConfig);
        }
        Ok(Self {
            seed: config.collapse.keys().iter().cloned().collect(),
            overrides: BTreeMap::new(),
            limit: initial_limit(&config),
            config,
            generation,
        })
    }

    /// Atomic validation; changed managed seeds are deliberately ignored until
    /// a generation reset or controlled-to-managed ownership transition.
    pub fn configure(&mut self, config: Config, generation: i64) -> Result<bool, InvalidConfig> {
        if generation == self.generation && config == self.config {
            return Ok(false);
        }
        if generation < self.generation || !config.is_valid() {
            return Err(InvalidConfig);
        }
        let reset = generation != self.generation;
        if reset
            || !matches!(
                (&self.config.collapse, &config.collapse),
                (Collapse::Managed(_), Collapse::Managed(_))
            )
        {
            self.seed = config.collapse.keys().iter().cloned().collect();
            self.overrides.clear();
        }
        if reset
            || !matches!(
                (&self.config.line_limit, &config.line_limit),
                (LineLimit::Managed { .. }, LineLimit::Managed { .. })
            )
        {
            self.limit = initial_limit(&config);
        }
        self.config = config;
        self.generation = generation;
        Ok(true)
    }
    /// Conservative owned storage units, including seeds retained from an
    /// earlier managed config. This is admission accounting, not allocator RSS.
    pub fn retained_bytes(&self) -> usize {
        let key_bytes = |key: &FileKey| match key {
            FileKey::Unnamed => 128,
            FileKey::Path(path) => 128 + path.capacity(),
        };
        std::mem::size_of::<Self>()
            + self.config.retained_bytes()
            + self.seed.iter().map(key_bytes).sum::<usize>()
            + self.overrides.keys().map(key_bytes).sum::<usize>()
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn limit(&self) -> Option<usize> {
        self.limit.map(|n| n as usize)
    }
    pub fn is_collapsed(&self, file: &crate::document_diff::File) -> bool {
        let key = key(file);
        match &self.config.collapse {
            Collapse::Controlled(_) => self.seed.contains(&key),
            Collapse::Managed(_) => self
                .overrides
                .get(&key)
                .copied()
                .unwrap_or_else(|| self.seed.contains(&key)),
        }
    }

    /// Call when a new parsed snapshot installs. Unknown seed keys stay bounded
    /// configuration; interactive overrides for removed files are discarded.
    pub fn install(&mut self, diff: &Diff) {
        let present: BTreeSet<_> = diff.files.iter().map(key).collect();
        self.overrides.retain(|key, _| present.contains(key));
    }
    pub fn toggle_file(&mut self, diff: &Diff, index: usize) -> Option<Observation> {
        let source = diff.files.get(index)?;
        let collapsed = !self.is_collapsed(source);
        let file = File {
            index: i64::try_from(index).ok()?,
            key: key(source),
            before_path: source.before_path.as_deref().map(str::to_owned),
            after_path: source.after_path.as_deref().map(str::to_owned),
        };
        if !file.is_valid() {
            return None;
        }
        self.install(diff);
        let applied = matches!(self.config.collapse, Collapse::Managed(_));
        if applied {
            self.overrides.insert(file.key.clone(), collapsed);
        }
        Some(Observation::ToggleFile {
            file,
            collapsed,
            applied,
        })
    }
    pub fn show_more(&mut self, projection: &Projection) -> Option<Observation> {
        let visible = projection.shown_body_lines() as i64;
        let hidden = projection.hidden_body_lines() as i64;
        if hidden == 0 || self.limit != Some(visible) {
            return None;
        }
        let applied_limit = match self.config.line_limit {
            LineLimit::Managed { step, .. } => Some((visible + step).min(8192)),
            LineLimit::Controlled(_) => None,
        };
        let observation = Observation::ShowMore {
            visible,
            hidden,
            applied_limit,
        };
        if !observation.valid_for(&self.config) {
            return None;
        }
        if applied_limit.is_some() {
            self.limit = applied_limit;
        }
        Some(observation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_diff::parse;
    const SOURCE: &str = "--- a/a\n+++ b/a\n@@ -1 +1 @@\n a\n--- a/b\n+++ b/b\n@@ -1 +1 @@\n b\n";
    fn projection(source: &str, diff: &Diff, state: &Controls) -> Projection {
        Projection::new(
            source,
            diff,
            |_, file| state.is_collapsed(file),
            state.limit(),
        )
        .unwrap()
    }
    #[test]
    fn replaced_managed_config_still_charges_its_original_seed() {
        let config = Config {
            collapse: Collapse::Managed(vec![FileKey::Path("x".repeat(4096))]),
            ..Config::default()
        };
        let mut controls = Controls::new(config, 1).unwrap();
        controls.configure(Config::default(), 1).unwrap();
        assert!(controls.config().collapse.keys().is_empty());
        let retained = controls.retained_bytes();
        assert!(retained > 4096);
        controls.configure(Config::default(), 2).unwrap();
        assert!(controls.retained_bytes() + 4096 < retained);
    }
    #[test]
    fn managed_values_survive_updates_but_reset_with_ownership_or_generation() {
        let diff = parse(SOURCE, || false).unwrap();
        let config = Config {
            line_limit: LineLimit::Managed {
                initial: Some(0),
                step: 1,
            },
            ..Config::default()
        };
        let mut state = Controls::new(config.clone(), 1).unwrap();
        assert!(matches!(
            state.toggle_file(&diff, 0),
            Some(Observation::ToggleFile {
                collapsed: true,
                applied: true,
                ..
            })
        ));
        assert!(state.is_collapsed(&diff.files[0]));
        let p = projection(SOURCE, &diff, &state);
        assert_eq!(p.hidden_body_lines(), 1);
        assert!(matches!(
            state.show_more(&p),
            Some(Observation::ShowMore {
                visible: 0,
                hidden: 1,
                applied_limit: Some(1)
            })
        ));
        assert_eq!(state.limit(), Some(1));
        assert!(
            state.show_more(&p).is_none(),
            "old projection cannot increment the new state again"
        );
        let changed_seed = Config {
            collapse: Collapse::Managed(vec![FileKey::Path("b".into())]),
            line_limit: LineLimit::Managed {
                initial: Some(2),
                step: 2,
            },
            ..config.clone()
        };
        state.configure(changed_seed.clone(), 1).unwrap();
        assert_eq!(state.limit(), Some(1));
        assert!(state.is_collapsed(&diff.files[0]));
        assert!(
            !state.is_collapsed(&diff.files[1]),
            "changing a managed seed is not a command"
        );
        state.toggle_file(&diff, 0).unwrap();
        let p = projection(SOURCE, &diff, &state);
        assert!(matches!(
            state.show_more(&p),
            Some(Observation::ShowMore {
                applied_limit: Some(3),
                ..
            })
        ));
        state.configure(changed_seed, 2).unwrap();
        assert_eq!(state.limit(), Some(2));
        assert!(!state.is_collapsed(&diff.files[0]));
        assert!(state.is_collapsed(&diff.files[1]));
        assert_eq!(state.configure(config.clone(), 1), Err(InvalidConfig));
        assert_eq!(state.limit(), Some(2));
        let controlled = Config {
            collapse: Collapse::Controlled(vec![FileKey::Path("a".into())]),
            line_limit: LineLimit::Controlled(Some(1)),
            word_diff: false,
        };
        state.configure(controlled, 2).unwrap();
        assert!(state.is_collapsed(&diff.files[0]));
        state.configure(config, 2).unwrap();
        assert!(!state.is_collapsed(&diff.files[0]));
        assert_eq!(state.limit(), Some(0));
    }
    #[test]
    fn controlled_intents_do_not_mutate_until_configured() {
        let diff = parse(SOURCE, || false).unwrap();
        let config = Config {
            collapse: Collapse::Controlled(vec![]),
            line_limit: LineLimit::Controlled(Some(1)),
            word_diff: true,
        };
        let mut state = Controls::new(config.clone(), 1).unwrap();
        let event = state.toggle_file(&diff, 1).unwrap();
        assert!(event.valid_for(&config));
        assert!(matches!(
            event,
            Observation::ToggleFile {
                collapsed: true,
                applied: false,
                ..
            }
        ));
        assert!(!state.is_collapsed(&diff.files[1]));
        let p = projection(SOURCE, &diff, &state);
        assert!(matches!(
            state.show_more(&p),
            Some(Observation::ShowMore {
                visible: 1,
                hidden: 1,
                applied_limit: None
            })
        ));
        assert_eq!(state.limit(), Some(1));
        state
            .configure(
                Config {
                    collapse: Collapse::Controlled(vec![FileKey::Path("b".into())]),
                    line_limit: LineLimit::Controlled(None),
                    ..config
                },
                1,
            )
            .unwrap();
        let p = projection(SOURCE, &diff, &state);
        assert!(state.is_collapsed(&diff.files[1]));
        assert_eq!(p.hidden_body_lines(), 0);
        assert!(state.show_more(&p).is_none());
    }
    #[test]
    fn future_seeds_pruning_duplicate_labels_and_invalid_configuration() {
        let diff = parse(SOURCE, || false).unwrap();
        let config = Config {
            collapse: Collapse::Managed(vec![FileKey::Path("future".into())]),
            ..Config::default()
        };
        let mut state = Controls::new(config.clone(), 1).unwrap();
        state.toggle_file(&diff, 0).unwrap();
        let future = parse("--- a/future\n+++ b/future\n@@ -1 +1 @@\n future\n", || {
            false
        })
        .unwrap();
        state.install(&future);
        assert!(state.is_collapsed(&future.files[0]));
        assert!(state.overrides.is_empty());
        state.install(&diff);
        assert!(!state.is_collapsed(&diff.files[0]));
        let duplicate = parse(&format!("{SOURCE}{SOURCE}"), || false).unwrap();
        state.toggle_file(&duplicate, 2).unwrap();
        assert!(state.is_collapsed(&duplicate.files[0]) && state.is_collapsed(&duplicate.files[2]));
        assert!(state.toggle_file(&diff, usize::MAX).is_none());
        let invalid = Config {
            line_limit: LineLimit::Controlled(Some(-1)),
            ..config.clone()
        };
        assert_eq!(state.configure(invalid, 1), Err(InvalidConfig));
        assert_eq!(state.config(), &config);
        assert_eq!(state.configure(config, 0), Err(InvalidConfig));
    }
}
