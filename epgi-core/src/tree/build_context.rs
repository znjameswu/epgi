use super::{ArcElementContextNode, HookContext};

// The entire BuildContext has to be refactored because
// we need the reference to element_context to avoid needless copies.
pub struct BuildContext<'a> {
    pub(crate) hooks: &'a mut HookContext,
    pub(crate) element_context: &'a ArcElementContextNode,
    // Passed to reconciler
    // pub(crate) node: AweakAnyElement,
    // pub(crate) scheduler: Asc<Scheduler<>>,
}

impl<'a> BuildContext<'a> {
    // pub(crate) fn new_inflate() -> Self {
    //     Self {
    //         hook_index: 0,
    //         work_mode: WorkMode::Inflate,
    //         hooks: todo!(),
    //         element_context: todo!(),
    //     }
    // }

    // pub(crate) fn new_rebuild(hooks: Hooks) -> Self {
    //     Self {
    //         hook_index: 0,
    //         work_mode: WorkMode::Rebuild,
    //         hooks,
    //         element_context: todo!()
    //     }
    // }

    // pub(crate) fn new_poll(hooks: Hooks) -> Self {
    //     Self {
    //         hook_index: 0,
    //         work_mode: WorkMode::Inflate,
    //         hooks: todo!(),
    //         element_context: todo!()
    //     }
    // }

    // pub fn use_consumer<T: Provide>(&mut self) -> Result<Arc<T>, Error> {
    //     let provider_element = self
    //         .providers
    //         .get(&TypeId::of::<T>())
    //         .ok_or(Error::HookError)?
    //         .upgrade()
    //         .ok_or(Error::BuildError)?;
    //     let ret = provider_element
    //         .provide()
    //         .read_and_register::<T>(self.node.clone(), todo!(), false)
    //         .or(Err(Error::BuildError));
    //     return ret; // Damned auto lifetime extensions.
    // }
    // pub fn use_consumer_mut<T: Provide>(&mut self) -> Result<(Arc<T>, impl Fn(T, JobId)), Error> {

    // }

    // pub fn use_consumer_mut<T:Provide>(&mut self) ->
}
