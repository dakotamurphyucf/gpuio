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
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum MenuPresentation {
    Button,
    Context,
    Bar,
    PlatformBar,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct MenuConfig {
    pub presentation: MenuPresentation,
    pub menus: Vec<MenuDefinition>,
}
impl MenuDefinition {
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
    pub fn command_ids<'a>(&'a self, output: &mut Vec<&'a str>) {
        for item in &self.items {
            match item {
                MenuItem::Command(id) => output.push(id),
                MenuItem::Submenu(menu) => menu.command_ids(output),
                MenuItem::Separator => (),
            }
        }
    }
    fn permits(&self, id: &str) -> bool {
        !self.disabled
            && self.items.iter().any(|item| match item {
                MenuItem::Command(candidate) => candidate == id,
                MenuItem::Submenu(menu) => menu.permits(id),
                MenuItem::Separator => false,
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
                            MenuItem::Command(id) => id.len(),
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
    pub fn is_valid(&self) -> bool {
        let mut count = 0;
        let mut text = 0;
        let shape = match self.presentation {
            MenuPresentation::Button | MenuPresentation::Context => self.menus.len() == 1,
            MenuPresentation::Bar | MenuPresentation::PlatformBar => self.menus.len() <= 32,
        };
        shape
            && self
                .menus
                .iter()
                .all(|menu| menu.validate(1, &mut count, &mut text))
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
