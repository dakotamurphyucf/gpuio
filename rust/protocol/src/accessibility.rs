//! Bounded semantic metadata; no layout, actions, or controllers live here.
use binprot::macros::BinProtWrite;

pub const MAX_TEXT_BYTES: usize = 4096;
pub const MAX_CONFIG_BYTES: usize = 16384;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct TreeItem {
    pub level: i64,
    pub index: i64,
    pub count: Option<i64>,
    pub expanded: Option<bool>,
    pub selected: bool,
    pub disabled: bool,
    pub busy: bool,
}
impl TreeItem {
    pub fn is_valid(self) -> bool {
        (1..=128).contains(&self.level)
            && (0..100_000).contains(&self.index)
            && self
                .count
                .is_none_or(|count| count > self.index && count <= 100_000)
    }
}

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
    Navigation,
    Tree(bool),
    TreeItem(TreeItem),
}
impl Role {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Heading(level) => (1..=6).contains(&level),
            Self::TreeItem(item) => item.is_valid(),
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Current {
    True,
    Page,
    Step,
    Location,
    Date,
    Time,
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
    pub current: Option<Current>,
}
impl Config {
    pub fn supports(&self, kind: crate::v1::Kind) -> bool {
        use crate::v1::Kind;
        if self.current.is_some()
            && !matches!(kind, Kind::Text | Kind::Button | Kind::CommandButton)
        {
            return false;
        }
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
                        | Kind::OtpInput
                        | Kind::Calendar
                        | Kind::ColorInput
                        | Kind::RadioGroup
                        | Kind::Select
                );
        }
        match self.role {
            Some(Role::Tree(_)) => kind == Kind::VirtualList,
            Some(Role::TreeItem(_)) => kind == Kind::Container,
            Some(Role::Navigation) => kind == Kind::Container,
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
                    | Kind::VirtualList
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
                    | Kind::OtpInput
                    | Kind::Calendar
                    | Kind::ColorInput
                    | Kind::RadioGroup
                    | Kind::Select
            ),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.role.is_none_or(Role::is_valid)
            && self.label.as_deref().is_none_or(valid_text)
            && self.description.as_deref().is_none_or(valid_text)
            && (self.current.is_none() || self.description.is_some())
            && self.field.as_ref().is_none_or(|field| {
                field.is_valid()
                    && self.current.is_none()
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
