//! Owned command definitions, independent of a GUI or language runtime.
use crate::NodeId;
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum ShortcutModifier {
    Primary,
    Control,
    Alt,
    Shift,
    Super,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ShortcutPriority {
    NativeFirst,
    Override,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum ShortcutTextInput {
    ModifiedOnly,
    Always,
    Never,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: Vec<ShortcutModifier>,
    pub priority: ShortcutPriority,
    pub text_input: ShortcutTextInput,
    pub during_composition: bool,
}
impl Shortcut {
    pub fn is_valid(&self) -> bool {
        let key = self.key.as_str();
        let named = matches!(
            key,
            "enter"
                | "escape"
                | "tab"
                | "space"
                | "backspace"
                | "delete"
                | "insert"
                | "home"
                | "end"
                | "pageup"
                | "pagedown"
                | "left"
                | "right"
                | "up"
                | "down"
        );
        let function_key = (1..=24).any(|index| key == format!("f{index}"));
        let mut chars = key.chars();
        let scalar =
            chars.next().is_some_and(|ch| !ch.is_control() && ch != ' ') && chars.next().is_none();
        (named || function_key || scalar)
            && key == key.to_ascii_lowercase()
            && self.modifiers.len() <= 5
            && self.modifiers.windows(2).all(|pair| pair[0] < pair[1])
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum NativeCommand {
    Copy,
    Cut,
    Paste,
    SelectAll,
    Undo,
    Redo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum CommandTarget {
    Callback,
    Native(NativeCommand),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct CommandConfig {
    pub id: String,
    pub generation: i64,
    pub label: String,
    pub enabled: bool,
    pub checked: Option<bool>,
    pub shortcuts: Vec<Shortcut>,
    pub target: CommandTarget,
}
impl CommandConfig {
    pub fn valid_text(value: &str, max: usize) -> bool {
        // Match Core.Char.is_whitespace, used by Command.validate_text.
        // Rust str::trim also strips Unicode whitespace, which would reject
        // identifiers that the public OCaml constructor has already accepted.
        value.bytes().any(|byte| !matches!(byte, 9..=13 | 32))
            && value.len() <= max
            && !value.contains('\0')
    }
    pub fn text_bytes(&self) -> usize {
        self.id.len()
            + self.label.len()
            + self
                .shortcuts
                .iter()
                .map(|shortcut| shortcut.key.len())
                .sum::<usize>()
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.text_bytes()
            + self
                .shortcuts
                .iter()
                .map(|shortcut| {
                    std::mem::size_of::<Shortcut>()
                        + std::mem::size_of_val(shortcut.modifiers.as_slice())
                })
                .sum::<usize>()
    }
    pub fn registry_is_valid(commands: &[Self]) -> bool {
        Self::registry_entries_are_valid(commands.iter())
    }
    /// Validate wire entries and shared native snapshots with the same policy.
    pub fn registry_entries_are_valid<'a>(
        mut commands: impl ExactSizeIterator<Item = &'a Self>,
    ) -> bool {
        let mut ids = std::collections::BTreeSet::new();
        let mut text_bytes = 0;
        commands.len() <= 1024
            && commands.all(|command| {
                Self::valid_text(&command.id, 256)
                    && Self::valid_text(&command.label, 4096)
                    && command.generation > 0
                    && ids.insert(&command.id)
                    && command.shortcuts.len() <= 4
                    && command.shortcuts.iter().all(Shortcut::is_valid)
                    && {
                        text_bytes += command.text_bytes();
                        text_bytes <= crate::v1::MAX_TEXT_BYTES
                    }
            })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum CommandSource {
    Button(NodeId),
    Shortcut,
    Menu(NodeId),
    Palette(NodeId),
}

#[cfg(test)]
mod tests {
    use super::CommandConfig;
    #[test]
    fn blank_command_text_uses_the_same_ascii_whitespace_set_as_core() {
        for text in ["", " ", "\t\n\u{b}\u{c}\r "] {
            assert!(!CommandConfig::valid_text(text, 256));
        }
        for text in ["run", "é界", "\u{a0}", "\u{2003}"] {
            assert!(CommandConfig::valid_text(text, 256));
        }
    }
}
