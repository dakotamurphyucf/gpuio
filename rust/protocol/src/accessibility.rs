//! Bounded semantic metadata; no layout, actions, or controllers live here.
use binprot::macros::BinProtWrite;

pub const MAX_TEXT_BYTES: usize = 4096;
pub const MAX_CONFIG_BYTES: usize = 16384;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Role {
    Group,
    Label,
    Link,
    Separator,
    DescriptionList,
    Term,
    Definition,
    Status,
    Alert,
    Image,
    Heading(i64),
}
impl Role {
    pub fn is_valid(self) -> bool {
        !matches!(self, Self::Heading(level) if !(1..=6).contains(&level))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Live {
    Off,
    Polite,
    Assertive,
}

fn valid_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_TEXT_BYTES && !text.contains('\0')
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Field {
    pub label: String,
    pub help: Option<String>,
    pub error: Option<String>,
    pub required: bool,
}
impl Field {
    pub fn is_valid(&self) -> bool {
        valid_text(&self.label)
            && self.help.as_deref().is_none_or(valid_text)
            && self.error.as_deref().is_none_or(valid_text)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub role: Option<Role>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub live: Live,
    pub field: Option<Field>,
}
impl Config {
    pub fn supports(&self, kind: crate::v1::Kind) -> bool {
        use crate::v1::Kind;
        if self.field.is_some() {
            return self.role.is_none()
                && matches!(
                    kind,
                    Kind::Input
                        | Kind::Textarea
                        | Kind::Combobox
                        | Kind::Checkbox
                        | Kind::Switch
                        | Kind::Rating
                        | Kind::Slider
                        | Kind::NumberInput
                        | Kind::RadioGroup
                        | Kind::Select
                );
        }
        match self.role {
            Some(Role::Link) => matches!(kind, Kind::Button | Kind::CommandButton),
            Some(
                Role::Group
                | Role::Label
                | Role::Separator
                | Role::DescriptionList
                | Role::Term
                | Role::Definition
                | Role::Status
                | Role::Alert
                | Role::Image
                | Role::Heading(_),
            ) => matches!(kind, Kind::Container | Kind::Text),
            None => matches!(
                kind,
                Kind::Container
                    | Kind::Text
                    | Kind::Button
                    | Kind::CommandButton
                    | Kind::Input
                    | Kind::Textarea
                    | Kind::Combobox
                    | Kind::Checkbox
                    | Kind::Switch
                    | Kind::Rating
                    | Kind::Slider
                    | Kind::NumberInput
                    | Kind::RadioGroup
                    | Kind::Select
            ),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.role.is_none_or(Role::is_valid)
            && self.label.as_deref().is_none_or(valid_text)
            && self.description.as_deref().is_none_or(valid_text)
            && self.field.as_ref().is_none_or(|field| {
                field.is_valid()
                    && self.role.is_none()
                    && self.label.is_none()
                    && self.description.is_none()
                    && self.live == Live::Off
            })
    }
    pub fn retained_bytes(&self) -> usize {
        let bytes = |text: &Option<String>| text.as_ref().map_or(0, String::capacity);
        std::mem::size_of::<Self>()
            + bytes(&self.label)
            + bytes(&self.description)
            + self
                .field
                .as_ref()
                .map_or(0, |f| f.label.capacity() + bytes(&f.help) + bytes(&f.error))
    }
}
