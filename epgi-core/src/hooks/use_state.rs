use std::{fmt::Debug, marker::PhantomData};

use crate::{
    foundation::Arc,
    scheduler::JobBuilder,
    tree::{AweakElementContextNode, BuildContext, Effect, Hook, HookIndex, HookState, Update},
};

impl<'a> BuildContext<'a> {
    pub fn use_state_ref_with<T: State>(&mut self, init: impl FnOnce() -> T) -> (&T, SetState<T>) {
        let (hook_state, index, element_context) = self.use_hook(StateHook { init });
        (
            &hook_state.value,
            SetState::new(Arc::downgrade(element_context), index),
        )
    }

    pub fn use_state_ref<T: State>(&mut self, init: T) -> (&T, SetState<T>) {
        self.use_state_ref_with(|| init)
    }

    pub fn use_state_ref_default<T: State + Default>(&mut self) -> (&T, SetState<T>) {
        self.use_state_ref_with(T::default)
    }

    pub fn use_state_with<T: State>(&mut self, init: impl FnOnce() -> T) -> (T, SetState<T>) {
        let (state, set_state) = self.use_state_ref_with(init);
        (state.clone(), set_state)
    }

    pub fn use_state<T: State>(&mut self, init: T) -> (T, SetState<T>) {
        let (state, set_state) = self.use_state_ref(init);
        (state.clone(), set_state)
    }

    pub fn use_state_default<T: State + Default>(&mut self) -> (T, SetState<T>) {
        let (state, set_state) = self.use_state_ref_default::<T>();
        (state.clone(), set_state)
    }
}

pub trait State: 'static + Debug + Send + Sync + Clone {}

impl<T> State for T where T: 'static + Debug + Send + Sync + Clone {}

struct StateHook<T: State, F: FnOnce() -> T> {
    init: F,
}

impl<T: State, F: FnOnce() -> T> Hook for StateHook<T, F> {
    type HookState = StateHookState<T>;

    fn create_hook_state(self) -> (Self::HookState, Option<impl Effect>) {
        (
            StateHookState {
                value: (self.init)(),
            },
            None::<()>,
        )
    }

    fn update_hook_state(self, _state: &mut Self::HookState) -> Option<impl Effect> {
        None::<()>
    }
}

#[derive(Clone)]
struct StateHookState<T: State> {
    value: T,
}

impl<T: State> HookState for StateHookState<T> {
    fn clone_box(&self) -> Box<dyn HookState> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct SetState<T> {
    node: AweakElementContextNode,
    self_index: HookIndex,
    phantom: PhantomData<T>,
}

impl<T> SetState<T>
where
    T: State,
{
    fn new(node: AweakElementContextNode, self_index: HookIndex) -> Self {
        Self {
            node,
            self_index,
            phantom: Default::default(),
        }
    }
    pub fn set(&self, value: T, job_builder: &mut JobBuilder) -> bool {
        let Some(node) = self.node.upgrade() else {
            return false;
        };
        node.push_update(
            Update::new::<StateHookState<T>>(self.self_index, move |hook| hook.value = value),
            job_builder,
        );
        return true;
    }
}
