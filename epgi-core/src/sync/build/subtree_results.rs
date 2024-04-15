use crate::{
    foundation::Protocol,
    tree::{ArcChildRenderObject, RenderAction},
};

// #[derive(Clone)]
pub enum SubtreeRenderObjectChange<P: Protocol> {
    /// Nothing has changed since the last commit.
    Keep {
        // You have to ensure child >= subtree
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    },
    New(ArcChildRenderObject<P>),
    Suspend,
}

impl<P> SubtreeRenderObjectChange<P>
where
    P: Protocol,
{
    pub const fn new_no_update() -> Self {
        Self::Keep {
            child_render_action: RenderAction::None,
            subtree_has_action: RenderAction::None,
        }
    }
    pub fn is_suspend(&self) -> bool {
        match self {
            SubtreeRenderObjectChange::Suspend => true,
            _ => false,
        }
    }

    pub fn is_keep_render_object(&self) -> bool {
        match self {
            SubtreeRenderObjectChange::Keep { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn as_summary(&self) -> SubtreeRenderObjectChangeSummary {
        match self {
            SubtreeRenderObjectChange::Keep {
                child_render_action,
                subtree_has_action,
            } => SubtreeRenderObjectChangeSummary::KeepAll {
                child_render_action: child_render_action.clone(),
                subtree_has_action: subtree_has_action.clone(),
            },
            SubtreeRenderObjectChange::New(_) => SubtreeRenderObjectChangeSummary::HasNewNoSuspend,
            SubtreeRenderObjectChange::Suspend => SubtreeRenderObjectChangeSummary::HasSuspended,
        }
    }

    pub(crate) fn summarize<'a>(
        iter: impl IntoIterator<Item = &'a Self>,
    ) -> SubtreeRenderObjectChangeSummary {
        // The following code is an adaption from Rust iterator's try_fold and try_reduce.
        // It turns out that Rust can very efficiently optimize this pattern for the single-element array case (and the double-element array).
        // https://godbolt.org/z/z3KKv4xqM
        let mut iter = iter.into_iter();
        let Some(init) = iter.next() else {
            return SubtreeRenderObjectChangeSummary::KeepAll {
                child_render_action: RenderAction::None,
                subtree_has_action: RenderAction::None,
            };
        };
        let mut res = init.as_summary();
        for item in iter {
            let s = item.as_summary();
            res = std::cmp::max(res, s);
            if matches!(res, SubtreeRenderObjectChangeSummary::HasSuspended) {
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
pub(crate) enum SubtreeRenderObjectChangeSummary {
    KeepAll {
        // You have to ensure child >= subtree
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    },
    HasNewNoSuspend,
    HasSuspended,
}

impl SubtreeRenderObjectChangeSummary {
    pub(crate) fn is_suspended(&self) -> bool {
        matches!(self, SubtreeRenderObjectChangeSummary::HasSuspended)
    }

    pub(crate) fn is_keep_all(&self) -> bool {
        matches!(self, SubtreeRenderObjectChangeSummary::KeepAll { .. })
    }
}
