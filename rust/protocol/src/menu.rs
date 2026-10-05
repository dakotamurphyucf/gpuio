use crate::v1::CommandConfig;
use binprot::macros::BinProtWrite;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct MenuDefinition {
    pub label: String,
    pub disabled: bool,
    pub items: Vec<MenuItem>,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum MenuItem {
    Command(String),
    Separator,
    Submenu(MenuDefinition),
    Label(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum MenuPresentation {
    Button,
    Context,
    Bar,
    PlatformBar,
    EditorContext,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct MenuConfig {
    pub presentation: MenuPresentation,
    pub menus: Vec<MenuDefinition>,
}
impl MenuDefinition {
    fn item_count(&self) -> usize {
        self.items.len()
            + self
                .items
                .iter()
                .map(|item| match item {
                    MenuItem::Submenu(child) => child.item_count(),
                    MenuItem::Command(_) | MenuItem::Separator | MenuItem::Label(_) => 0,
                })
                .sum::<usize>()
    }
    fn validate(&self, depth: usize, count: &mut usize, text: &mut usize) -> bool {
        if depth > 8 || !CommandConfig::valid_text(&self.label, 4096) {
            return false;
        }
        *text += self.label.len();
        *count += self.items.len();
        if *count > 1024 || *text > 262144 {
            return false;
        }
        for item in &self.items {
            match item {
                MenuItem::Label(label) => {
                    if !CommandConfig::valid_text(label, 4096) {
                        return false;
                    }
                    *text += label.len();
                }
                MenuItem::Command(id) => {
                    if !CommandConfig::valid_text(id, 256) {
                        return false;
                    }
                    *text += id.len();
                }
                MenuItem::Submenu(menu) => {
                    if !menu.validate(depth + 1, count, text) {
                        return false;
                    }
                }
                MenuItem::Separator => (),
            }
        }
        *text <= 262144
    }
    fn contains_labels(&self) -> bool {
        self.items.iter().any(|item| match item {
            MenuItem::Label(_) => true,
            MenuItem::Submenu(menu) => menu.contains_labels(),
            MenuItem::Command(_) | MenuItem::Separator => false,
        })
    }
    pub fn command_ids<'a>(&'a self, output: &mut Vec<&'a str>) {
        for item in &self.items {
            match item {
                MenuItem::Command(id) => output.push(id),
                MenuItem::Submenu(menu) => menu.command_ids(output),
                MenuItem::Separator | MenuItem::Label(_) => (),
            }
        }
    }
    fn permits(&self, id: &str) -> bool {
        !self.disabled
            && self.items.iter().any(|item| match item {
                MenuItem::Command(candidate) => candidate == id,
                MenuItem::Submenu(menu) => menu.permits(id),
                MenuItem::Separator | MenuItem::Label(_) => false,
            })
    }
    fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.label.len()
            + self
                .items
                .iter()
                .map(|item| {
                    std::mem::size_of::<MenuItem>()
                        + match item {
                            MenuItem::Command(id) | MenuItem::Label(id) => id.len(),
                            MenuItem::Submenu(menu) => {
                                menu.retained_bytes() - std::mem::size_of::<Self>()
                            }
                            MenuItem::Separator => 0,
                        }
                })
                .sum::<usize>()
    }
}
impl MenuConfig {
    /// Preorder content slots for an already validated menu collection.
    pub fn items_preorder(&self) -> Vec<&MenuItem> {
        fn visit<'a>(menu: &'a MenuDefinition, output: &mut Vec<&'a MenuItem>) {
            for item in &menu.items {
                output.push(item);
                if let MenuItem::Submenu(child) = item {
                    visit(child, output);
                }
            }
        }
        let mut items = Vec::new();
        for menu in &self.menus {
            visit(menu, &mut items);
        }
        items
    }

    /// Slot indices of the direct rows in a root/submenu path. The caller has
    /// admitted the bounded collection; invalid navigation paths return None.
    pub fn row_content_indices(&self, path: &[usize]) -> Option<Vec<usize>> {
        let (&root, path) = path.split_first()?;
        let mut menu = self.menus.get(root)?;
        let mut offset = self.menus[..root]
            .iter()
            .map(MenuDefinition::item_count)
            .sum::<usize>();
        let size = |item: &MenuItem| {
            1 + match item {
                MenuItem::Submenu(child) => child.item_count(),
                MenuItem::Command(_) | MenuItem::Separator | MenuItem::Label(_) => 0,
            }
        };
        for &index in path {
            let MenuItem::Submenu(child) = menu.items.get(index)? else {
                return None;
            };
            offset += menu.items[..index].iter().map(size).sum::<usize>() + 1;
            menu = child;
        }
        Some(
            menu.items
                .iter()
                .map(|item| {
                    let index = offset;
                    offset += size(item);
                    index
                })
                .collect(),
        )
    }

    pub fn is_valid(&self) -> bool {
        let mut count = 0;
        let mut text = 0;
        let shape = match self.presentation {
            MenuPresentation::Button
            | MenuPresentation::Context
            | MenuPresentation::EditorContext => self.menus.len() == 1,
            MenuPresentation::Bar | MenuPresentation::PlatformBar => self.menus.len() <= 32,
        };
        shape
            && self
                .menus
                .iter()
                .all(|menu| menu.validate(1, &mut count, &mut text))
            && (self.presentation != MenuPresentation::PlatformBar
                || !self.menus.iter().any(MenuDefinition::contains_labels))
    }
    pub fn command_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        for menu in &self.menus {
            menu.command_ids(&mut ids);
        }
        ids
    }
    pub fn permits(&self, id: &str) -> bool {
        self.menus.iter().any(|menu| menu.permits(id))
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .menus
                .iter()
                .map(MenuDefinition::retained_bytes)
                .sum::<usize>()
    }
}

impl MenuPresentation {
    pub fn is_context(self) -> bool {
        matches!(self, Self::Context | Self::EditorContext)
    }
}
