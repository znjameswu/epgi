use crate::{
    foundation::Protocol,
    tree::{ArcChildRenderObject, RenderAction},
};

pub struct CommitResult<P: Protocol> {
    pub render_object: RenderObjectCommitResult<P>,
    // pub subtree_lanes: SubtreeLanesCommitResult,
}

// #[derive(Clone)]
// pub struct SubtreeLanesCommitResult {
//     /// The lanes removed due to a deactivated consumer
//     pub removed_lanes: LaneMask,
//     /// The last version of subtree lanes. Note that some bits may be outdated due to concurrent async job mark-ups.
//     ///
//     /// This is needed since up-propagation of removed lanes needs lane information from its siblings to decide whether to up-propagate.
//     /// The lifecycle of batch execution guarantees that removed lanes can never be marking up at the same time.
//     /// (Really? what if one subtree is marking up and another subtree is being deactivated?) (Unresolved)
//     /// Therefore, for removed lanes, this field will not be outdated. But other lanes could be outdated.
//     /// Since we never know which lanes could be removed in sibling subtrees, we always have to return a full lane mask including those potentially useless and outdated lanes.
//     pub current: LaneMask,
// }

// impl SubtreeLanesCommitResult {
//     pub(crate) fn new() -> Self {
//         Self {
//             remove_has_happened: false,
//             current: LaneMask::new(),
//         }
//     }

//     pub(crate) fn merge(self, other: Self) -> Self {
//         Self {
//             remove_has_happened: self.remove_has_happened | other.remove_has_happened,
//             current: self.current | other.current,
//         }
//     }
// }

impl<P: Protocol> CommitResult<P> {
    pub(crate) fn new(render_object_commit_result: RenderObjectCommitResult<P>) -> Self {
        Self {
            render_object: render_object_commit_result,
        }
    }

    // pub(crate) fn as_summary(&self) -> CommitSummary {
    //     CommitSummary {
    //         render_object: self.render_object.as_summary(),
    //     }
    // }

    // pub(crate) fn summarize<'a>(iter: impl IntoIterator<Item = &'a Self>) -> CommitSummary {
    //     // The following code is an adaption from Rust iterator's try_fold and try_reduce.
    //     // It turns out that Rust can very efficiently optimize this pattern for the single-element array case (and the double-element array).
    //     // https://godbolt.org/z/z3KKv4xqM
    //     let mut iter = iter.into_iter();
    //     let Some(init) = iter.next() else {
    //         return CommitSummary::new();
    //     };
    //     let mut res = init.as_summary();
    //     for item in iter {
    //         let s = item.as_summary();
    //         res = res.merge(s);
    //     }
    //     res
    // }
}

// pub struct CommitSummary {
//     pub render_object: RenderObjectCommitSummary,
//     // pub consumer_lanes: SubtreeLanesCommitResult,
// }

// impl CommitSummary {
//     pub(crate) fn new() -> Self {
//         Self {
//             render_object: RenderObjectCommitSummary::new(),
//         }
//     }

//     pub(crate) fn merge(self, other: Self) -> Self {
//         Self {
//             render_object: self.render_object.merge(other.render_object),
//         }
//     }
// }

// #[derive(Clone)]
pub enum RenderObjectCommitResult<P: Protocol> {
    /// Nothing has changed since the last commit.
    Keep {
        // You have to ensure child >= subtree
        propagated_render_action: RenderAction,
        /// This field pass through without being absorbed by some boundaries.
        subtree_has_action: RenderAction,
    },
    New(ArcChildRenderObject<P>),
    Suspend,
}

impl<P: Protocol> RenderObjectCommitResult<P> {
    pub const fn new_no_update() -> Self {
        Self::Keep {
            propagated_render_action: RenderAction::None,
            subtree_has_action: RenderAction::None,
        }
    }
    pub fn is_suspend(&self) -> bool {
        match self {
            RenderObjectCommitResult::Suspend => true,
            _ => false,
        }
    }

    pub fn is_keep_render_object(&self) -> bool {
        match self {
            RenderObjectCommitResult::Keep { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn as_summary(&self) -> RenderObjectCommitSummary {
        match self {
            RenderObjectCommitResult::Keep {
                propagated_render_action,
                subtree_has_action,
            } => RenderObjectCommitSummary::KeepAll {
                propagated_render_action: *propagated_render_action,
                descendant_has_action: *subtree_has_action,
            },
            RenderObjectCommitResult::New(_) => RenderObjectCommitSummary::HasNewNoSuspend,
            RenderObjectCommitResult::Suspend => RenderObjectCommitSummary::HasSuspended,
        }
    }

    pub(crate) fn summarize<'a>(
        iter: impl IntoIterator<Item = &'a Self>,
    ) -> RenderObjectCommitSummary {
        // The following code is an adaption from Rust iterator's try_fold and try_reduce.
        // It turns out that Rust can very efficiently optimize this pattern for the single-element array case (and the double-element array).
        // https://godbolt.org/z/z3KKv4xqM
        let mut iter = iter.into_iter();
        let Some(init) = iter.next() else {
            return RenderObjectCommitSummary::new();
        };
        let mut res = init.as_summary();
        for item in iter {
            let s = item.as_summary();
            res = res.merge(s);
            if matches!(res, RenderObjectCommitSummary::HasSuspended) {
                break;
            }
        }
        res
    }
}

/// An optimiaztion helper struct, which aims to cache a summary version of all commited results, to avoid repetitive queries on commited results
#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub(crate) enum RenderObjectCommitSummary {
    KeepAll {
        // You have to ensure child >= subtree
        propagated_render_action: RenderAction,
        descendant_has_action: RenderAction,
    },
    HasNewNoSuspend,
    HasSuspended,
}

impl RenderObjectCommitSummary {
    pub(crate) fn new() -> Self {
        Self::KeepAll {
            propagated_render_action: RenderAction::None,
            descendant_has_action: RenderAction::None,
        }
    }

    pub(crate) fn merge(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }
    pub(crate) fn is_suspended(&self) -> bool {
        matches!(self, RenderObjectCommitSummary::HasSuspended)
    }

    pub(crate) fn is_keep_all(&self) -> bool {
        matches!(self, RenderObjectCommitSummary::KeepAll { .. })
    }
}
