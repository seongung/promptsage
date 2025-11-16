#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAction {
    None,
    Review,
    CopyImproved,
    SaveExport,
    ShowDiff,
    Quit,
}

impl Default for UserAction {
    fn default() -> Self {
        UserAction::None
    }
}
