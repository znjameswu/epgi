use std::{any::type_name, fmt::Debug, marker::PhantomData};

use crate::{
    foundation::{Arc, PtrEq},
    scheduler::JobBuilder,
    tree::{AweakElementContextNode, BuildContext, Effect, Hook, HookIndex, HookState, Update},
};

impl<'a> BuildContext<'a> {
    pub fn use_reducer_ref_with<T: Reduce>(
        &mut self,
        init: impl FnOnce() -> T,
    ) -> (&T, DispatchReducer<T>) {
        let node = Arc::downgrade(self.element_context);
        let (hook_state, index) = self.use_hook(ReducerHook { init });
        (&hook_state.state, DispatchReducer::new(node, index))
    }

    pub fn use_reducer_ref<T: Reduce>(&mut self, init: T) -> (&T, DispatchReducer<T>) {
        self.use_reducer_ref_with(|| init)
    }

    pub fn use_reducer_ref_default<T: Reduce + Default>(&mut self) -> (&T, DispatchReducer<T>) {
        self.use_reducer_ref_with(T::default)
    }

    pub fn use_reducer_with<T: Reduce>(
        &mut self,
        init: impl FnOnce() -> T,
    ) -> (T, DispatchReducer<T>) {
        let (state, set_state) = self.use_reducer_ref_with(init);
        (state.clone(), set_state)
    }

    pub fn use_reducer<T: Reduce>(&mut self, init: T) -> (T, DispatchReducer<T>) {
        let (state, set_state) = self.use_reducer_ref(init);
        (state.clone(), set_state)
    }

    pub fn use_reducer_default<T: Reduce + Default>(&mut self) -> (T, DispatchReducer<T>) {
        let (state, set_state) = self.use_reducer_ref_default::<T>();
        (state.clone(), set_state)
    }
}

pub trait Reduce: Clone + Debug + Send + Sync + 'static {
    type Action: Clone + Debug + Send + Sync + 'static;

    fn reduce(&mut self, action: Self::Action);
}

#[derive(Clone)]
pub struct DispatchReducer<T> {
    node: AweakElementContextNode,
    self_index: HookIndex,
    phantom: PhantomData<T>,
}

impl<T> Debug for DispatchReducer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchReducer")
            .field("Type", &type_name::<T>())
            .finish()
    }
}

impl<T> PartialEq for DispatchReducer<T> {
    fn eq(&self, other: &Self) -> bool {
        PtrEq(&self.node) == PtrEq(&other.node)
            && self.self_index == other.self_index
            && self.phantom == other.phantom
    }
}

pub(super) struct ReducerHook<T: Reduce, F: FnOnce() -> T> {
    pub(super) init: F,
}

impl<T: Reduce, F: FnOnce() -> T> Hook for ReducerHook<T, F> {
    type HookState = ReducerHookState<T>;

    fn create_hook_state(self) -> (Self::HookState, Option<impl Effect>) {
        (
            ReducerHookState {
                state: (self.init)(),
            },
            None::<()>,
        )
    }

    fn update_hook_state(self, _state: &mut Self::HookState) -> Option<impl Effect> {
        None::<()>
    }
}

#[derive(Clone)]
pub(super) struct ReducerHookState<T: Reduce> {
    pub(super) state: T,
}

impl<T: Reduce> HookState for ReducerHookState<T> {
    fn clone_box(&self) -> Box<dyn HookState> {
        Box::new(self.clone())
    }
}

impl<T: Reduce> DispatchReducer<T> {
    pub(super) fn new(node: AweakElementContextNode, self_index: HookIndex) -> Self {
        Self {
            node,
            self_index,
            phantom: Default::default(),
        }
    }
    pub fn dispatch(&self, action: T::Action, job_builder: &mut JobBuilder) -> bool {
        let Some(node) = self.node.upgrade() else {
            return false;
        };
        node.push_update(
            Update::new::<ReducerHookState<T>>(self.self_index, |hook| hook.state.reduce(action)),
            job_builder,
        );
        return true;
    }
}
