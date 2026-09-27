use gpui::SharedString;

/// Native row identity allocated by the host for one membership lifetime.
/// A host must not reuse it for a different row inside one mounted table.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RowKey(pub u64);

/// Optimistic native selection. Indices are resolved only for layout; source
/// and column reorder must not move selection to another record.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Selection {
    #[default]
    Empty,
    Row(RowKey),
    Column(SharedString),
    Cell {
        row: RowKey,
        column: SharedString,
    },
}
