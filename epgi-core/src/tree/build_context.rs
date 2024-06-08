use crate::{foundation::VecPushLastExt, scheduler::LanePos};

use super::{
    ArcElementContextNode, Effect, Hook, HookContextMode, HookIndex, HooksWithEffects,
    HooksWithCleanups,
};

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
pub struct BuildContext<'a> {
    pub(crate) lane_pos: LanePos,
    pub(crate) element_context: &'a ArcElementContextNode,
    pub(crate) hook_context: HookContext<'a>,
}

pub(crate) struct HookContext<'a> {
    pub(crate) hooks: Hooks<'a>,
    pub(crate) index: usize,
    pub(crate) mode: HookContextMode,
}

pub(crate) enum Hooks<'a> {
    Sync(&'a mut HooksWithCleanups),
    Async(&'a mut HooksWithEffects),
}

impl<'a> HookContext<'a> {
    pub(crate) fn new_sync(hooks: &'a mut HooksWithCleanups, mode: HookContextMode) -> Self {
        Self {
            hooks: Hooks::Sync(hooks),
            index: 0,
            mode,
        }
    }

    pub(crate) fn new_async(hooks: &'a mut HooksWithEffects, mode: HookContextMode) -> Self {
        Self {
            hooks: Hooks::Async(hooks),
            index: 0,
            mode,
        }
    }

    pub(crate) fn has_finished(&mut self) -> bool {
        self.index
            == match &self.hooks {
                Hooks::Sync(hooks) => hooks.array_hooks.len(),
                Hooks::Async(hooks) => hooks.array_hooks.len(),
            }
    }
}

impl<'a> BuildContext<'a> {
    pub fn use_hook<T: Hook>(&mut self, hook: T) -> (&mut T::HookState, HookIndex) {
        match &mut self.hook_context.hooks {
            Hooks::Sync(hooks) => {
                let hooks_len = hooks.array_hooks.len();
                let index = self.hook_context.index;
                let hook_state = if self.hook_context.index < hooks_len {
                    debug_assert!(matches!(
                        self.hook_context.mode,
                        HookContextMode::Rebuild | HookContextMode::PollInflate
                    ));
                    hooks.reconcile_array_hook(hook, index)
                } else if self.hook_context.index == hooks_len {
                    debug_assert!(matches!(
                        self.hook_context.mode,
                        HookContextMode::Inflate | HookContextMode::PollInflate
                    ));
                    hooks.push_array_hook(hook)
                } else {
                    panic!("Hook reads should not be out of bound")
                };
                self.hook_context.index += 1;
                (hook_state, HookIndex { index })
            }
            Hooks::Async(hooks) => {
                let hooks_len = hooks.array_hooks.len();
                let index = self.hook_context.index;
                let hook_state = if self.hook_context.index < hooks_len {
                    debug_assert!(matches!(
                        self.hook_context.mode,
                        HookContextMode::Rebuild | HookContextMode::PollInflate
                    ));
                    hooks.reconcile_array_hook(hook, index)
                } else if self.hook_context.index == hooks_len {
                    debug_assert!(matches!(
                        self.hook_context.mode,
                        HookContextMode::Inflate | HookContextMode::PollInflate
                    ));
                    hooks.push_array_hook(hook)
                } else {
                    panic!("Hook reads should not be out of bound")
                };
                self.hook_context.index += 1;
                (hook_state, HookIndex { index })
            }
        }
    }
}

impl HooksWithCleanups {
    fn reconcile_array_hook<T: Hook>(&mut self, hook: T, index: usize) -> &mut T::HookState {
        let (hook_state, tear_down) = self.array_hooks.get_mut(index).expect("Impossible to fail");
        let hook_state = hook_state
            .as_any_mut()
            .downcast_mut::<T::HookState>()
            .expect("Hook type should match with the last hook type");

        let new_effect = hook.update_hook_state(hook_state);

        if let Some(new_effect) = new_effect {
            tear_down.take().map(|tear_down| tear_down.cleanup());
            *tear_down = new_effect.fire();
        }
        hook_state
    }

    fn push_array_hook<T: Hook>(&mut self, hook: T) -> &mut T::HookState {
        let (hook_state, effect) = hook.create_hook_state();

        let tear_down = effect.and_then(Effect::fire);

        let (hook_state, _tear_down) = self
            .array_hooks
            .push_last((Box::new(hook_state), tear_down));
        let hook_state = hook_state
            .as_any_mut()
            .downcast_mut::<T::HookState>()
            .expect("Impossible to fail");
        hook_state
    }
}

impl HooksWithEffects {
    fn reconcile_array_hook<T: Hook>(&mut self, hook: T, index: usize) -> &mut T::HookState {
        let (hook_state, effect) = self.array_hooks.get_mut(index).expect("Impossible to fail");
        let hook_state = hook_state
            .as_any_mut()
            .downcast_mut::<T::HookState>()
            .expect("Hook type should match with the last hook type");

        let new_effect = hook.update_hook_state(hook_state);
        *effect = new_effect.map(|effect| Box::new(effect) as _);
        hook_state
    }

    fn push_array_hook<T: Hook>(&mut self, hook: T) -> &mut T::HookState {
        let (hook_state, new_effect) = hook.create_hook_state();
        let new_effect = new_effect.map(|effect| Box::new(effect) as _);

        let (hook_state, _tear_down) = self
            .array_hooks
            .push_last((Box::new(hook_state), new_effect));
        let hook_state = hook_state
            .as_any_mut()
            .downcast_mut::<T::HookState>()
            .expect("Impossible to fail");
        hook_state
    }
}
