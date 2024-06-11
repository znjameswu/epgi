use std::marker::PhantomData;

use crate::{
    foundation::Arc,
    scheduler::JobBuilder,
    tree::{AweakElementContextNode, BuildContext, HookIndex, Update},
};

use super::{State, StateHook, StateHookState};

impl<'a> BuildContext<'a> {
    pub fn use_reducer_ref_with<T: State>(
        &mut self,
        init: impl FnOnce() -> T,
    ) -> (&T, DispatchReducer<T>) {
        let node = Arc::downgrade(self.element_context);
        let (hook_state, index) = self.use_hook(StateHook { init });
        (&hook_state.value, DispatchReducer::new(node, index))
    }

    pub fn use_reducer_ref<T: State>(&mut self, init: T) -> (&T, DispatchReducer<T>) {
        self.use_reducer_ref_with(|| init)
    }

    pub fn use_reducer_ref_default<T: State + Default>(&mut self) -> (&T, DispatchReducer<T>) {
        self.use_reducer_ref_with(T::default)
    }

    pub fn use_reducer_with<T: State>(
        &mut self,
        init: impl FnOnce() -> T,
    ) -> (T, DispatchReducer<T>) {
        let (state, set_state) = self.use_reducer_ref_with(init);
        (state.clone(), set_state)
    }

    pub fn use_reducer<T: State>(&mut self, init: T) -> (T, DispatchReducer<T>) {
        let (state, set_state) = self.use_reducer_ref(init);
        (state.clone(), set_state)
    }

    pub fn use_reducer_default<T: State + Default>(&mut self) -> (T, DispatchReducer<T>) {
        let (state, set_state) = self.use_reducer_ref_default::<T>();
        (state.clone(), set_state)
    }
}

#[derive(Clone)]
pub struct DispatchReducer<T> {
    node: AweakElementContextNode,
    self_index: HookIndex,
    phantom: PhantomData<T>,
}

impl<T: State> DispatchReducer<T> {
    pub(super) fn new(node: AweakElementContextNode, self_index: HookIndex) -> Self {
        Self {
            node,
            self_index,
            phantom: Default::default(),
        }
    }
    pub fn dispatch(
        &self,
        reducer: impl FnOnce(&mut T) -> T + Clone + Send + Sync + 'static,
        job_builder: &mut JobBuilder,
    ) -> bool {
        let Some(node) = self.node.upgrade() else {
            return false;
        };
        node.push_update(
            Update::new::<StateHookState<T>>(self.self_index, |hook| {
                hook.value = reducer(&mut hook.value)
            }),
            job_builder,
        );
        return true;
    }
}
