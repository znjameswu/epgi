use crate::{
    foundation::Protocol,
    tree::{ArcChildRenderObject, RerenderAction},
};

#[derive(Clone)]
pub enum SubtreeRenderObjectCommitResult<P: Protocol> {
    KeepRenderObject {
        // You have to ensure child >= subtree
        child_render_action: RerenderAction,
        subtree_has_action: RerenderAction,
    },
    NewRenderObject(ArcChildRenderObject<P>),
    Suspended,
}

impl<P> SubtreeRenderObjectCommitResult<P>
where
    P: Protocol,
{
    pub fn is_suspended(&self) -> bool {
        match self {
            SubtreeRenderObjectCommitResult::Suspended => true,
            _ => false,
        }
    }

    pub fn is_keep_render_object(&self) -> bool {
        match self {
            SubtreeRenderObjectCommitResult::KeepRenderObject { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn as_summary(&self) -> SubtreeRenderObjectCommitResultSummary {
        match self {
            SubtreeRenderObjectCommitResult::KeepRenderObject {
                child_render_action,
                subtree_has_action,
            } => SubtreeRenderObjectCommitResultSummary::KeepRenderObject {
                child_render_action: child_render_action.clone(),
                subtree_has_action: subtree_has_action.clone(),
            },
            SubtreeRenderObjectCommitResult::NewRenderObject(_) => {
                SubtreeRenderObjectCommitResultSummary::NewRenderObject
            }
            SubtreeRenderObjectCommitResult::Suspended => {
                SubtreeRenderObjectCommitResultSummary::Suspended
            }
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