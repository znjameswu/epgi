use crate::{foundation::Protocol, tree::ArcChildRenderObject};

#[derive(Debug, PartialOrd, PartialEq, Eq, Ord, Clone)]
pub enum SubtreeVisitResult<P: Protocol> {
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

impl<P> SubtreeVisitResult<P> where P: Protocol {
    pub fn is_suspended(&self) -> bool {
        match self {
            SubtreeVisitResult::Suspended => true,
            _ => false,
        }
    }
}
impl<P> Default for SubtreeVisitResult<P>
where
    P: Protocol,
{
    fn default() -> Self {
        SubtreeVisitResult::NoUpdate
    }
}

// struct CommitResult
