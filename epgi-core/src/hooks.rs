mod use_effect;
pub use use_effect::*;

mod use_state;
pub use use_state::*;

use crate::{foundation::Arc, tree::BuildContext};

pub trait BuildContextHookExt {
    fn use_state_ref_with<T: State>(&mut self, init: impl FnOnce() -> T) -> (&T, SetState<T>);
    fn use_state_ref<T: State>(&mut self, init: T) -> (&T, SetState<T>);
    fn use_state_ref_with_default<T: State + Default>(&mut self) -> (&T, SetState<T>);
    fn use_state_with<T: State>(&mut self, init: impl FnOnce() -> T) -> (T, SetState<T>);
    fn use_state<T: State>(&mut self, init: T) -> (T, SetState<T>);
    fn use_state_with_default<T: State + Default>(&mut self) -> (T, SetState<T>);
}

impl<'a> BuildContextHookExt for BuildContext<'a> {
    fn use_state_ref_with<T: State>(&mut self, init: impl FnOnce() -> T) -> (&T, SetState<T>) {
        let (hook_state, index, element_context) = self.use_hook(StateHook { init });
        (
            &hook_state.value,
            SetState::new(Arc::downgrade(element_context), index),
        )
    }

    fn use_state_ref<T: State>(&mut self, init: T) -> (&T, SetState<T>) {
        self.use_state_ref_with(|| init)
    }

    fn use_state_ref_with_default<T: State + Default>(&mut self) -> (&T, SetState<T>) {
        self.use_state_ref_with(T::default)
    }

    fn use_state_with<T: State>(&mut self, init: impl FnOnce() -> T) -> (T, SetState<T>) {
        let (state_ref, set_state) = self.use_state_ref_with(init);
        (state_ref.clone(), set_state)
    }

    fn use_state<T: State>(&mut self, init: T) -> (T, SetState<T>) {
        let (state_ref, set_state) = self.use_state_ref(init);
        (state_ref.clone(), set_state)
    }

    fn use_state_with_default<T: State + Default>(&mut self) -> (T, SetState<T>) {
        let (state_ref, set_state) = self.use_state_ref_with_default::<T>();
        (state_ref.clone(), set_state)
    }
}
