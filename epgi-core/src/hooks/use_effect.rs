use crate::{
    foundation::DependencyKey,
    tree::{BuildContext, Effect, EffectCleanup, Hook, HookState},
};

impl<'a> BuildContext<'a> {
    pub fn use_effect<D: DependencyKey, C: EffectCleanup>(
        &mut self,
        effect: impl FnOnce(D) -> C + Send + Sync + 'static,
        dependencies: D,
    ) {
        self.use_hook(EffectHook {
            dependencies,
            effect,
        });
    }

    pub fn use_effect_nodep<C: EffectCleanup>(
        &mut self,
        effect: impl FnOnce() -> C + Send + Sync + 'static,
    ) {
        self.use_effect(|_| effect(), NoDependency)
    }
}

macro_rules! impl_use_effect {
    ($name: ident, $($input: ident : $input_type: ident),*) => {
        pub fn $name<$($input_type: DependencyKey),*, C: EffectCleanup>(
            &mut self,
            effect: impl FnOnce($($input_type),*) -> C + Send + Sync + 'static,
            $($input: $input_type),*
        ) {
            self.use_effect(|($($input),*)| effect($($input),*), ($($input),*))
        }
    };
}

impl<'a> BuildContext<'a> {
    impl_use_effect!(use_effect_2, dep1: D1, dep2: D2);
    impl_use_effect!(use_effect_3, dep1: D1, dep2: D2, dep3: D3);
}

struct EffectHook<E, D> {
    dependencies: D,
    effect: E,
}

impl<C: EffectCleanup, D: DependencyKey, E: FnOnce(D) -> C + Send + Sync + 'static> Hook
    for EffectHook<E, D>
{
    type HookState = EffectHookState<D>;

    fn create_hook_state(self) -> (Self::HookState, Option<impl Effect>) {
        (
            EffectHookState {
                dependencies: self.dependencies.clone(),
            },
            Some(self.into_effect()),
        )
    }

    fn update_hook_state(self, state: &mut Self::HookState) -> Option<impl Effect> {
        if state.dependencies != self.dependencies {
            state.dependencies = self.dependencies.clone();
            Some(self.into_effect())
        } else {
            None
        }
    }
}

impl<C: EffectCleanup, D: DependencyKey, E: FnOnce(D) -> C + Send + Sync + 'static>
    EffectHook<E, D>
{
    fn into_effect(self) -> impl Effect {
        || {
            let cleanup = (self.effect)(self.dependencies);
            if cleanup.is_noop() {
                None
            } else {
                Some(Box::new(cleanup) as _)
            }
        }
    }
}

#[derive(Clone)]
struct EffectHookState<D> {
    dependencies: D,
}

impl<D: DependencyKey> HookState for EffectHookState<D> {
    fn clone_box(&self) -> Box<dyn HookState> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct NoDependency;

impl PartialEq for NoDependency {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
