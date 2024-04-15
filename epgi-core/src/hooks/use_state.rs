use std::{fmt::Debug, marker::PhantomData};

use crate::{
    scheduler::JobBuilder,
    tree::{AweakElementContextNode, Effect, Hook, HookIndex, HookState, Update},
};

pub trait State: 'static + Debug + Send + Sync + Clone {}

impl<T> State for T where T: 'static + Debug + Send + Sync + Clone {}

pub(super) struct StateHook<T: State, F: FnOnce() -> T> {
    pub(super) init: F,
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

    fn update_hook_state(self, state: &mut Self::HookState) -> Option<impl Effect> {
        None::<()>
    }
}

#[derive(Clone)]
pub(super) struct StateHookState<T: State> {
    pub(super) value: T,
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
    pub(super) fn new(node: AweakElementContextNode, self_index: HookIndex) -> Self {
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
        {
            let mut mailbox = node.mailbox.lock();
            job_builder.extend_sequenced_jobs_in(self.node.clone(), &mut *mailbox, self.self_index);
            mailbox
                .entry(job_builder.id())
                .or_default()
                .push(Update::new::<StateHookState<T>>(
                    self.self_index,
                    move |hook| hook.value = value,
                ));
        }
        return true;
    }
}

// #[derive(Clone)]
// pub struct StateHook<T> {
//     pub val: T,
// }

// impl<T> HookState for StateHook<T>
// where
//     T: State,
// {
//     fn clone_box(&self) -> Box<dyn HookState> {
//         Box::new(self.clone())
//     }
// }
