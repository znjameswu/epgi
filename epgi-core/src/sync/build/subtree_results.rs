use crate::{foundation::Protocol, tree::ArcChildRenderObject};

#[derive(Debug, PartialOrd, PartialEq, Eq, Ord, Clone, Copy)]
pub enum SubtreeCommitResult<P: Protocol> {
    NoUpdate = 0,
    NewRenderObject(ArcChildRenderObject<P>) = 1,
    Suspended = 2,
}

// impl SubtreeCommitResult {
//     pub fn merge(self, other: Self) -> Self {
//         std::cmp::max(self, other)
//     }

//     pub fn absorb(self) -> Self {
//         match self {
//             SubtreeCommitResult::NewRenderObject => SubtreeCommitResult::NoUpdate,
//             default => default,
//         }
//     }
// }
impl<P> Default for SubtreeCommitResult<P>
where
    P: Protocol,
{
    fn default() -> Self {
        SubtreeCommitResult::NoUpdate
    }
}

// struct CommitResult
