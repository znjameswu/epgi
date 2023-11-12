use crate::{
    foundation::Protocol,
    tree::{ArcChildRenderObject, RerenderAction},
};

// #[derive(Clone)]
pub enum SubtreeRenderObjectChange<P: Protocol> {
    /// Nothing has changed since the last commit.
    Keep {
        // You have to ensure child >= subtree
        child_render_action: RerenderAction,
        subtree_has_action: RerenderAction,
    },
    New(ArcChildRenderObject<P>),
    /// An element is suspended. However, the element that hold the immediate child render object
    /// is below the suspended element and undetached, and it reports an unchanged render object.
    Suspend,
    /// An element is suspended. However, the element that hold the immediate child render object
    /// is below the suspended element and undetached, and it created a new render object.
    SuspendAboveNew(ArcChildRenderObject<P>),
    /// The immediate child render object is detached
    Detach,
}

impl<P> SubtreeRenderObjectChange<P>
where
    P: Protocol,
{
    pub fn new_no_update() -> Self {
        Self::Keep {
            child_render_action: RerenderAction::None,
            subtree_has_action: RerenderAction::None,
        }
    }
    pub fn is_suspended(&self) -> bool {
        todo!()
        // match self {
        //     SubtreeRenderObjectChange::Suspended => true,
        //     _ => false,
        // }
    }

    pub fn is_keep_render_object(&self) -> bool {
        match self {
            SubtreeRenderObjectChange::Keep { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn as_summary(&self) -> SubtreeRenderObjectCommitResultSummary {
        match self {
            SubtreeRenderObjectChange::Keep {
                child_render_action,
                subtree_has_action,
            } => SubtreeRenderObjectCommitResultSummary::KeepRenderObject {
                child_render_action: child_render_action.clone(),
                subtree_has_action: subtree_has_action.clone(),
            },
            SubtreeRenderObjectChange::New(_) => {
                SubtreeRenderObjectCommitResultSummary::NewRenderObject
            }
            SubtreeRenderObjectChange::Suspend => SubtreeRenderObjectCommitResultSummary::Suspended,
            SubtreeRenderObjectChange::SuspendAboveNew(_) => {
                SubtreeRenderObjectCommitResultSummary::Suspended
            }
            SubtreeRenderObjectChange::Detach => SubtreeRenderObjectCommitResultSummary::Suspended,
        }
    }

    pub(crate) fn summarize<'a>(
        iter: impl IntoIterator<Item = &'a Self>,
    ) -> SubtreeRenderObjectCommitResultSummary {
        // The following code is an adaption from Rust iterator's try_fold and try_reduce.
        // It turns out that Rust can very efficiently optimize this pattern for the single-element array case (and the double-element array).
        // https://godbolt.org/z/z3KKv4xqM
        let mut iter = iter.into_iter();
        let Some(init) = iter.next() else {
            return SubtreeRenderObjectCommitResultSummary::KeepRenderObject {
                child_render_action: RerenderAction::None,
                subtree_has_action: RerenderAction::None,
            };
        };
        let mut res = init.as_summary();
        for item in iter {
            let s = item.as_summary();
            res = std::cmp::max(res, s);
            if matches!(res, SubtreeRenderObjectCommitResultSummary::Suspended) {
                break;
            }
        }
        res
    }
}
// impl<P> Default for SubtreeRenderObjectCommitResult<P>
// where
//     P: Protocol,
// {
//     fn default() -> Self {
//         SubtreeRenderObjectCommitResult::KeepRenderObject(RerenderAction::None)
//     }
// }

// struct CommitResult
#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub(crate) enum SubtreeRenderObjectCommitResultSummary {
    KeepRenderObject {
        // You have to ensure child >= subtree
        child_render_action: RerenderAction,
        subtree_has_action: RerenderAction,
    },
    NewRenderObject,
    Suspended,
}

impl SubtreeRenderObjectCommitResultSummary {
    pub(crate) fn is_suspended(&self) -> bool {
        matches!(self, SubtreeRenderObjectCommitResultSummary::Suspended)
    }
}
