use std::{fmt::Debug, marker::PhantomData};

use crate::{
    common::{AweakElementContextNode, BuildContext, Hook, Update},
    foundation::Arc,
    scheduler::{JobBuilder, JobId},
};

pub trait State: 'static + Debug + Send + Sync + Clone {}

impl<T> State for T where T: 'static + Debug + Send + Sync + Clone {}

#[derive(Clone)]
pub struct SetState<T> {
    node: AweakElementContextNode,
    self_index: usize,
    phantom: PhantomData<T>,
}

impl<T> SetState<T>
where
    T: State,
{
    fn set(&self, value: T, job_builder: &mut JobBuilder) -> bool {
        let Some(node) = self.node.upgrade() else {
            return false;
        };
        {
            let mut mailbox = node.mailbox.lock();
            job_builder.extend_sequenced_jobs_in(&mut *mailbox, self.self_index);
            mailbox
                .entry(job_builder.id())
                .or_default()
                .push(Update::new::<StateHook<T>>(self.self_index, move |hook| {
                    hook.val = value
                }));
        }
        return true;
    }
}

impl<'a> BuildContext<'a> {
    pub fn use_state<T: State>(&mut self, init: impl FnOnce() -> T) -> (&T, SetState<T>) {
        let (hook_ref, index) = self.hooks.use_hook(|| StateHook { val: init() });
        (
            &hook_ref.val,
            SetState {
                node: Arc::downgrade(self.element_context),
                self_index: todo!(),
                phantom: PhantomData,
            },
        )
    }
}

#[derive(Clone)]
pub struct StateHook<T> {
    pub(crate) val: T,
}

impl<T> Hook for StateHook<T>
where
    T: State,
{
    fn clone_box(&self) -> Box<dyn Hook> {
        Box::new(self.clone())
    }
}
