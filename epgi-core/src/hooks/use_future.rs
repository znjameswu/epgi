use std::future::Future;

use futures::FutureExt;

use crate::{
    foundation::{Arc, BuildSuspendedError, DependencyKey},
    tree::{ArcSuspendWaker, BuildContext, Effect, Hook, HookState, SuspendWaker},
};

use super::State;

impl<'a> BuildContext<'a> {
    pub fn use_future_ref<
        D: DependencyKey,
        Fut: Future<Output = T> + Unpin + Send + Sync + 'static,
        T: State,
    >(
        &mut self,
        compute_future: impl FnOnce(D) -> Fut,
        dependencies: D,
    ) -> Result<&T, BuildSuspendedError> {
        let element_context = Arc::downgrade(self.element_context);
        let waker = SuspendWaker::new(element_context, self.lane_pos);
        let (hook_state, _index, _element_context) = self.use_hook(FutureHook {
            dependencies,
            compute_future,
        });
        hook_state.poll(waker)
    }

    pub fn use_future<
        D: DependencyKey,
        Fut: Future<Output = T> + Unpin + Send + Sync + 'static,
        T: State,
    >(
        &mut self,
        compute_future: impl FnOnce(D) -> Fut,
        dependencies: D,
    ) -> Result<T, BuildSuspendedError> {
        self.use_future_ref(compute_future, dependencies)
            .map(Clone::clone)
    }
}

macro_rules! impl_use_future {
    ($name: ident, $($input: ident : $input_type: ident),*) => {
        pub fn $name<
            $($input_type: DependencyKey),*,
            Fut: Future<Output = T> + Unpin + Send + Sync + 'static,
            T: State
        >(
            &mut self,
            compute_future: impl FnOnce($($input_type),*) -> Fut,
            $($input: $input_type),*
        ) -> Result<T, BuildSuspendedError> {
            self.use_future(|($($input),*)| compute_future($($input),*), ($($input),*))
        }
    };
}

impl<'a> BuildContext<'a> {
    impl_use_future!(use_future_2, dep1: D1, dep2: D2);
    impl_use_future!(use_future_3, dep1: D1, dep2: D2, dep3: D3);
}

// use_future should just be an async version of use_memo
struct FutureHook<F, D> {
    dependencies: D,
    compute_future: F,
}

impl<D: DependencyKey, F, Fut, T> Hook for FutureHook<F, D>
where
    F: FnOnce(D) -> Fut,
    Fut: Future<Output = T> + Send + Sync + 'static,
    T: State,
{
    type HookState = FutureHookState<D, Fut, T>;

    fn create_hook_state(self) -> (Self::HookState, Option<impl Effect>) {
        (
            FutureHookState {
                dependencies: self.dependencies.clone(),
                maybe_done: MaybeDone::Future((self.compute_future)(self.dependencies).shared()),
            },
            None::<()>,
        )
    }

    fn update_hook_state(self, state: &mut Self::HookState) -> Option<impl Effect> {
        if state.dependencies != self.dependencies {
            state.dependencies = self.dependencies.clone();
            state.maybe_done = MaybeDone::Future((self.compute_future)(self.dependencies).shared());
        }
        None::<()>
    }
}

struct FutureHookState<D, Fut: Future, T: Clone> {
    dependencies: D,
    maybe_done: MaybeDone<futures::future::Shared<Fut>, T>,
}

#[derive(Clone)]
enum MaybeDone<Fut, T> {
    Future(Fut),
    Done(T),
}

impl<D: DependencyKey, Fut, T> Clone for FutureHookState<D, Fut, T>
where
    Fut: Future<Output = T>,
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            dependencies: self.dependencies.clone(),
            maybe_done: self.maybe_done.clone(),
        }
    }
}

impl<D: DependencyKey, Fut, T> HookState for FutureHookState<D, Fut, T>
where
    Fut: Future<Output = T> + Send + Sync + 'static,
    T: State,
{
    fn clone_box(&self) -> Box<dyn HookState> {
        Box::new(self.clone())
    }
}

impl<D: DependencyKey, Fut, T> FutureHookState<D, Fut, T>
where
    Fut: Future<Output = T> + Unpin + Send + Sync + 'static,
    T: State,
{
    fn poll(&mut self, waker: ArcSuspendWaker) -> Result<&T, BuildSuspendedError> {
        let maybe_done = &mut self.maybe_done;
        match maybe_done {
            MaybeDone::Done(value) => Ok(value),
            MaybeDone::Future(fut) => {
                let future_waker = waker.clone().into_waker();
                let mut context = std::task::Context::from_waker(&future_waker);
                let poll_result = std::pin::pin!(fut).poll(&mut context);
                match poll_result {
                    std::task::Poll::Ready(value) => {
                        *maybe_done = MaybeDone::Done(value);
                        let MaybeDone::Done(value) = maybe_done else {
                            panic!("Impossible to fail")
                        };
                        waker.abort();
                        Ok(value)
                    }
                    std::task::Poll::Pending => Err(BuildSuspendedError { waker }),
                }
            }
        }
    }
}
