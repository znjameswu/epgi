use crate::{foundation::Protocol, tree::ArcChildRenderObject};

#[derive(Debug, PartialOrd, PartialEq, Eq, Ord, Clone)]
pub enum SubtreeRenderObjectUpdate<P: Protocol> {
    KeepRenderObject = 0,
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

impl<P> SubtreeRenderObjectUpdate<P> where P: Protocol {
    pub fn is_suspended(&self) -> bool {
        match self {
            SubtreeRenderObjectUpdate::Suspended => true,
            _ => false,
        }
    }

    pub fn is_no_update(&self) -> bool {
        match self {
            SubtreeRenderObjectUpdate::KeepRenderObject => true,
            _ => false,
        }
    }
}
impl<P> Default for SubtreeRenderObjectUpdate<P>
where
    P: Protocol,
{
    fn default() -> Self {
        SubtreeRenderObjectUpdate::KeepRenderObject
    }
}

// struct CommitResult
