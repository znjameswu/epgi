

// use_future should just be an async version of use_memo
pub(super) struct FutureHook<F, D> {
    pub(super) dependencies: D,
    pub(super) future: F,
}


pub(super) struct FutureHookState<D, F> {
    pub(super) dependencies: D,
    pub(super) future: F,
}