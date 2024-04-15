use crate::tree::{Effect, EffectCleanup, Hook, HookState};

pub(super) struct EffectHook<E, D> {
    pub(super) dependencies: D,
    pub(super) effect: E,
}

impl<
        E: FnOnce(D) -> C + Send + Sync + 'static,
        C: EffectCleanup,
        D: PartialEq + Clone + Send + Sync + 'static,
    > Hook for EffectHook<E, D>
{
    type HookState = EffectHookState<D>;

    fn create_hook_state(self) -> (Self::HookState, Option<impl Effect>) {
        (
            EffectHookState {
                dependencies: self.dependencies.clone(),
            },
            Some(|| {
                let cleanup = (self.effect)(self.dependencies);
                if cleanup.is_noop() {
                    None
                } else {
                    Some(Box::new(cleanup) as _)
                }
            }),
        )
    }

    fn update_hook_state(self, state: &mut Self::HookState) -> Option<impl Effect> {
        (state.dependencies != self.dependencies).then_some(|| {
            let cleanup = (self.effect)(self.dependencies);
            if cleanup.is_noop() {
                None
            } else {
                Some(Box::new(cleanup) as _)
            }
        })
    }
}

#[derive(Clone)]
pub(super) struct EffectHookState<D> {
    dependencies: D,
}

impl<D: Clone + Send + Sync + 'static> HookState for EffectHookState<D> {
    fn clone_box(&self) -> Box<dyn HookState> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub(super) struct NoDependency;

impl PartialEq for NoDependency {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
