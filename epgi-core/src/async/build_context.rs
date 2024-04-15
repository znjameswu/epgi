use crate::{
    foundation::VecPushLastExt,
    tree::{ArcElementContextNode, Hook, HookIndex, HooksWithEffects, WorkMode},
};

pub(crate) struct AsyncBuildContext<'a> {
    pub(crate) hooks: &'a mut AsyncHookContext,
    pub(crate) element_context: &'a ArcElementContextNode,
}

pub(super) struct AsyncHookContext {
    pub(crate) hooks: HooksWithEffects,
    pub(crate) index: usize,
    pub(crate) mode: WorkMode,
}

impl AsyncHookContext {
    pub(crate) fn new_rebuild(hooks: HooksWithEffects) -> Self {
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

    pub(crate) fn new_poll_inflate(hooks: HooksWithEffects) -> Self {
        Self {
            hooks,
            index: 0,
            mode: WorkMode::PollInflate,
        }
    }

    pub fn has_finished(&mut self) -> bool {
        self.index == self.hooks.array_hooks.len()
    }

    pub fn use_hook<T: Hook>(&mut self, hook: T) -> (&mut T::HookState, HookIndex) {
        let hooks_len = self.hooks.array_hooks.len();

        if self.index < hooks_len {
            debug_assert!(matches!(
                self.mode,
                WorkMode::Rebuild | WorkMode::PollInflate
            ));
            let (hook_state, effect) = self
                .hooks
                .array_hooks
                .get_mut(self.index)
                .expect("Impossible to fail");
            let hook_state = hook_state
                .as_any_mut()
                .downcast_mut::<T::HookState>()
                .expect("Hook type should match with the last hook type");

            let new_effect = hook.update_hook_state(hook_state);
            *effect = new_effect.map(|effect| Box::new(effect) as _);

            let index = self.index;
            self.index += 1;
            (hook_state, HookIndex { index })
        } else if self.index == hooks_len {
            debug_assert!(matches!(
                self.mode,
                WorkMode::Inflate | WorkMode::PollInflate
            ));
            let (hook_state, new_effect) = hook.create_hook_state();
            let new_effect = new_effect.map(|effect| Box::new(effect) as _);

            let (hook_state, _tear_down) = self
                .hooks
                .array_hooks
                .push_last((Box::new(hook_state), new_effect));
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

impl<'a> AsyncBuildContext<'a> {
    pub(crate) fn use_hook<T: Hook>(
        &mut self,
        hook: T,
    ) -> (&mut T::HookState, HookIndex, &ArcElementContextNode) {
        let (hook_state, index) = self.hooks.use_hook(hook);
        (hook_state, index, self.element_context)
    }
}
