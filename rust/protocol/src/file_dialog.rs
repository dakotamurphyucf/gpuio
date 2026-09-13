//! Validated data for native open/save requests. Request routing and native panel
//! ownership belong to the runtime, not this module.
use crate::file_path::FilePath;
use binprot::macros::BinProtWrite;

pub const MAX_SELECTED_PATHS: usize = 128;
pub const MAX_SELECTED_PATH_BYTES: usize = 262_144;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum FileSelection {
    Files,
    Directories,
    FilesAndDirectories,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct OpenFileConfig {
    pub selection: FileSelection,
    pub multiple: bool,
    pub title: String,
    pub accept_label: String,
    pub directory: Option<FilePath>,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct SaveFileConfig {
    pub directory: FilePath,
    pub suggested_name: String,
    pub title: String,
    pub accept_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum FileDialogConfig {
    Open(OpenFileConfig),
    Save(SaveFileConfig),
    Capabilities,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum FileSelectionSupport {
    Unsupported,
    Single,
    Multiple,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct FileDialogCapabilities {
    pub files: FileSelectionSupport,
    pub directories: FileSelectionSupport,
    pub files_and_directories: FileSelectionSupport,
    pub save: bool,
}

fn label(value: &str) -> bool {
    crate::v1::CommandConfig::valid_text(value, 4096)
}

impl FileDialogConfig {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Capabilities => true,
            Self::Open(config) => label(&config.title) && label(&config.accept_label),
            Self::Save(config) => {
                label(&config.title)
                    && label(&config.accept_label)
                    && !config.suggested_name.is_empty()
                    && config.suggested_name.len() <= 255
                    && !config.suggested_name.contains(['/', '\0'])
                    && config.suggested_name != "."
                    && config.suggested_name != ".."
            }
        }
    }

    /// Validate the entire selection without silently truncating or dropping a
    /// path. FilePath already enforces absolute, bounded, NUL-free native bytes.
    pub fn accepts_selection(&self, paths: &[FilePath]) -> bool {
        let maximum = match self {
            Self::Capabilities => return false,
            Self::Open(config) if config.multiple => MAX_SELECTED_PATHS,
            Self::Open(_) | Self::Save(_) => 1,
        };
        !paths.is_empty()
            && paths.len() <= maximum
            && paths
                .iter()
                .map(|path| path.as_bytes().len())
                .sum::<usize>()
                <= MAX_SELECTED_PATH_BYTES
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum FileDialogError {
    InvalidRequest,
    Unsupported,
    Busy,
    Closed,
    NotReady,
    NativeFailure,
    LimitExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum FileDialogResult {
    Selected(Vec<FilePath>),
    Cancelled,
    Failed(FileDialogError),
    Capabilities(FileDialogCapabilities),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(multiple: bool) -> FileDialogConfig {
        FileDialogConfig::Open(OpenFileConfig {
            selection: FileSelection::Files,
            multiple,
            title: "Open".into(),
            accept_label: "Choose".into(),
            directory: None,
        })
    }

    #[test]
    fn selection_limits_reject_the_whole_result_instead_of_truncating() {
        let path = FilePath::new(b"/tmp/\xff.txt".to_vec()).unwrap();
        assert!(!open(true).accepts_selection(&[]));
        assert!(open(false).accepts_selection(std::slice::from_ref(&path)));
        assert!(!open(false).accepts_selection(&[path.clone(), path.clone()]));
        assert!(open(true).accepts_selection(&vec![path.clone(); MAX_SELECTED_PATHS]));
        assert!(!open(true).accepts_selection(&vec![path; MAX_SELECTED_PATHS + 1]));
        let path = FilePath::new(vec![b'/'; crate::file_path::MAX_PATH_BYTES]).unwrap();
        assert!(open(true).accepts_selection(&vec![path.clone(); 16]));
        assert!(!open(true).accepts_selection(&vec![path; 17]));
    }

    #[test]
    fn labels_and_filename_hints_are_validated_without_rewriting() {
        let directory = FilePath::new(b"/tmp".to_vec()).unwrap();
        for name in ["a.sql.s", ".hidden", "λ.txt", " "] {
            let config = FileDialogConfig::Save(SaveFileConfig {
                directory: directory.clone(),
                suggested_name: name.into(),
                title: "Save".into(),
                accept_label: "Choose".into(),
            });
            assert!(config.is_valid());
            assert!(!config.accepts_selection(&[directory.clone(), directory.clone()]));
        }
        for name in ["", ".", "..", "a/b", "a\0b", &"a".repeat(256)] {
            let config = FileDialogConfig::Save(SaveFileConfig {
                directory: directory.clone(),
                suggested_name: name.into(),
                title: "Save".into(),
                accept_label: "Choose".into(),
            });
            assert!(!config.is_valid());
        }
        for title in ["", " ", "\0", &"a".repeat(4097)] {
            let FileDialogConfig::Open(mut config) = open(false) else {
                unreachable!()
            };
            config.title = title.into();
            assert!(!FileDialogConfig::Open(config).is_valid());
        }
    }
}
