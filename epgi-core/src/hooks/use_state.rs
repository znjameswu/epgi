use std::{any::type_name, fmt::Debug};

use crate::{scheduler::JobBuilder, tree::BuildContext};

use super::{use_reducer::DispatchReducer, Reduce};

impl<'a> BuildContext<'a> {
    pub fn use_state_ref_with<T: State>(&mut self, init: impl FnOnce() -> T) -> (&T, SetState<T>) {
        let (state, dispatch) = self.use_reducer_ref_with(|| UseStateReducer { value: init() });
        (&state.value, SetState { dispatch })
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

pub trait State: Clone + Debug + Send + Sync + 'static {}

impl<T> State for T where T: Clone + Debug + Send + Sync + 'static {}

#[derive(Clone, Debug)]
struct UseStateReducer<T> {
    value: T,
}

impl<T> Reduce for UseStateReducer<T>
where
    T: State,
{
    type Action = T;

    fn reduce(&mut self, action: Self::Action) {
        self.value = action
    }
}

#[derive(PartialEq, Clone)]
pub struct SetState<T> {
    dispatch: DispatchReducer<UseStateReducer<T>>,
}

impl<T> Debug for SetState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetState")
            .field("Type", &type_name::<T>())
            .finish()
    }
}

impl<T> SetState<T>
where
    T: State,
{
    pub fn set(&self, value: T, job_builder: &mut JobBuilder) -> bool {
        self.dispatch.dispatch(value, job_builder)
    }
}
