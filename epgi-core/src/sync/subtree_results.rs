#[derive(Debug, PartialOrd, PartialEq, Eq, Ord, Clone, Copy)]
pub enum SubtreeCommitResult {
    NoUpdate = 0,
    NewRenderObject = 1,
    Suspended = 2,
}

impl SubtreeCommitResult {
    pub fn merge(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }

    pub fn absorb(self) -> Self {
        match self {
            SubtreeCommitResult::NewRenderObject => SubtreeCommitResult::NoUpdate,
            default => default,
        }
    }
}
impl Default for SubtreeCommitResult {
    fn default() -> Self {
        SubtreeCommitResult::NoUpdate
    }
}