use crate::{
    foundation::VecPushLastExt,
    tree::{ArcElementContextNode, Effect, Hook, HookIndex, HooksWithTearDowns, WorkMode},
};

pub(crate) struct SyncBuildContext<'a> {
    pub(super) hooks: &'a mut SyncHookContext,
    pub(super) element_context: &'a ArcElementContextNode,
}

pub(super) struct SyncHookContext {
    pub(crate) hooks: HooksWithTearDowns,
    pub(crate) index: usize,
    pub(crate) mode: WorkMode,
}

impl SyncHookContext {
    pub(crate) fn new_rebuild(hooks: HooksWithTearDowns) -> Self {
        Self {
            hooks,
            index: 0,
            mode: WorkMode::Rebuild,
        }
    }

    pub(crate) fn new_inflate() -> Self {
        Self {
            hooks: Default::default(),
            index: 0,
            mode: WorkMode::Inflate,
            // layout_effects: Default::default(),
        }
    }

    pub(crate) fn new_poll_inflate(hooks: HooksWithTearDowns) -> Self {
        Self {
            hooks,
            index: 0,
            mode: WorkMode::PollInflate,
        }
    }

    fn has_finished(&mut self) -> bool {
        self.index == self.hooks.array_hooks.len()
    }

    fn use_hook<T: Hook>(&mut self, hook: T) -> (&mut T::HookState, HookIndex) {
        let hooks_len = self.hooks.array_hooks.len();

        if self.index < hooks_len {
            debug_assert!(matches!(
                self.mode,
                WorkMode::Rebuild | WorkMode::PollInflate
            ));
            let (hook_state, tear_down) = self
                .hooks
                .array_hooks
                .get_mut(self.index)
                .expect("Impossible to fail");
            let hook_state = hook_state
                .as_any_mut()
                .downcast_mut::<T::HookState>()
                .expect("Hook type should match with the last hook type");

            let new_effect = hook.update_hook_state(hook_state);

            if let Some(new_effect) = new_effect {
                if let Some(tear_down) = tear_down.take() {
                    tear_down.cleanup()
                }
                *tear_down = new_effect.fire();
            }
            let index = self.index;
            self.index += 1;
            (hook_state, HookIndex { index })
        } else if self.index == hooks_len {
            debug_assert!(matches!(
                self.mode,
                WorkMode::Inflate | WorkMode::PollInflate
            ));
            let (hook_state, effect) = hook.create_hook_state();

            let tear_down = effect.and_then(|effect| effect.fire());

            let (hook_state, _tear_down) = self
                .hooks
                .array_hooks
                .push_last((Box::new(hook_state), tear_down));
            let hook_state = hook_state
                .as_any_mut()
                .downcast_mut::<T::HookState>()
                .expect("Impossible to fail");
            let index = self.index;
            self.index += 1;
            (hook_state, HookIndex { index })
        } else {
            panic!("Hook reads should not be out of bound")
        }
    }
}

impl<'a> SyncBuildContext<'a> {
    pub(crate) fn use_hook<T: Hook>(
        &mut self,
        hook: T,
    ) -> (&mut T::HookState, HookIndex, &ArcElementContextNode) {
        let (hook_state, index) = self.hooks.use_hook(hook);
        (hook_state, index, self.element_context)
    }
}
