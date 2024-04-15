use crate::{
    foundation::DependencyKey,
    tree::{BuildContext, Effect, Hook, HookState},
};

use super::State;

struct MemoHook<D, F> {
    dependencies: D,
    compute: F,
}

impl<D: DependencyKey, F, T> Hook for MemoHook<D, F>
where
    F: FnOnce(D) -> T,
    T: State,
{
    type HookState = MemoHookState<D, T>;

    fn create_hook_state(self) -> (Self::HookState, Option<impl Effect>) {
        (
            MemoHookState {
                dependencies: self.dependencies.clone(),
                memoized: (self.compute)(self.dependencies),
            },
            None::<()>,
        )
    }

    fn update_hook_state(self, state: &mut Self::HookState) -> Option<impl Effect> {
        if state.dependencies != self.dependencies {
            state.dependencies = self.dependencies.clone();
            state.memoized = (self.compute)(self.dependencies);
        }
        None::<()>
    }
}

#[derive(Clone)]
struct MemoHookState<D, T> {
    dependencies: D,
    memoized: T,
}

impl<D: DependencyKey, T: State> HookState for MemoHookState<D, T> {
    fn clone_box(&self) -> Box<dyn HookState> {
        Box::new(self.clone())
    }
}

impl<'a> BuildContext<'a> {
    pub fn use_memo<D: DependencyKey, T: State>(
        &mut self,
        compute: impl FnOnce(D) -> T,
        dependencies: D,
    ) -> &T {
        let (val_ref, _index, _element_context) = self.use_hook(MemoHook {
            dependencies,
            compute,
        });
        &val_ref.memoized
    }
}

macro_rules! impl_use_memo {
    ($name: ident, $($input: ident : $input_type: ident),*) => {
        pub fn $name<$($input_type: DependencyKey),*, T: State>(
            &mut self,
            compute: impl FnOnce($($input_type),*) -> T,
            $($input: $input_type),*
        ) -> &T {
            self.use_memo(|($($input),*)| compute($($input),*), ($($input),*))
        }
    };
}

impl<'a> BuildContext<'a> {
    impl_use_memo!(use_memo_2, dep1: D1, dep2: D2);
    impl_use_memo!(use_memo_3, dep1: D1, dep2: D2, dep3: D3);
}
