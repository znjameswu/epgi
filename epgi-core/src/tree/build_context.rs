use crate::sync::SyncBuildContext;

use super::{ArcElementContextNode, Hook, HookIndex};

// pub trait BuildContext {
//     fn use_hook<T: Hook>(
//         &mut self,
//         hook: T,
//     ) -> (&mut T::HookState, HookIndex, &ArcElementContextNode);
// }

// The reason this cannot be made into a trait and be statically dispatched:
// 1. build function would have a generic parameter
// 2. Function cannot be made into widget, since there is no generic function pointer, nor generic closure
// 3. ComponentWidget can no longer share a ComponenetElement, because ComponentWidget now has a generic build method which is not object-safe
// The loss significantly outweighs the mere performance gain of an elided boolean check.
//
// The reason async and sync cannot be made to have the same type layout: No one will be happy, especially the sync one.
//
// The reason this cannot be made into a trait object and be dynamically dispatched:
// Same runtime dispatch, harder impl (now generic use_hook is forbidden), worse codegen, pure stupidity.
pub struct BuildContext<'a>(_BuildContext<'a>);

enum _BuildContext<'a> {
    Sync(SyncBuildContext<'a>),
    // Async(AsyncBuildContext)
}

impl<'a> From<SyncBuildContext<'a>> for BuildContext<'a> {
    fn from(value: SyncBuildContext<'a>) -> Self {
        Self(_BuildContext::Sync(value))
    }
}

impl<'a> BuildContext<'a> {
    pub fn use_hook<T: Hook>(
        &mut self,
        hook: T,
    ) -> (&mut T::HookState, HookIndex, &ArcElementContextNode) {
        match &mut self.0 {
            _BuildContext::Sync(ctx) => ctx.use_hook(hook),
        }
    }
}
