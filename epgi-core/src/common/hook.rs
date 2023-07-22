use crate::{
    foundation::{AsAny, Error},
    scheduler::JobId,
};

use super::{AweakElementContextNode, ElementContextNode};

pub trait Hook: AsAny + 'static + Send + Sync {
    fn apply_updates(&self, job_ids: Box<[JobId]>) -> Box<dyn Hook> {
        let mut res = self.clone_box();
        res.apply_updates_in_place(job_ids);
        return res;
    }

    fn drain_updates(&self, job_id: Box<[JobId]>);

    fn apply_updates_in_place(&mut self, job_ids: Box<[JobId]>);

    fn clone_box(&self) -> Box<dyn Hook>;
}

impl Clone for Box<dyn Hook> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// #[derive(Clone)]
// pub struct Update {
//     pub(crate) hook_index: usize,
//     pub(crate) op: Box<dyn HookCallback>,
// }

#[derive(Clone)]
pub struct Update {
    hook_index: usize,
    op: Box<dyn HookCallback>,
}

pub trait HookCallback: FnOnce(&mut dyn Hook) -> Result<(), Error> + 'static + Send + Sync {
    fn clone_box(&self) -> Box<dyn HookCallback>;
}

impl<T> HookCallback for T
where
    T: FnOnce(&mut dyn Hook) -> Result<(), Error> + Clone + 'static + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn HookCallback> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn HookCallback> {
    fn clone(&self) -> Box<dyn HookCallback> {
        self.clone_box()
    }
}

pub(super) enum HookUpdateSink {
    Buffering(Vec<Update>),
    Mounted(AweakElementContextNode),
}

impl HookUpdateSink {
    pub(super) fn push(&mut self, job_id: JobId, update: Update) {
        match self {
            HookUpdateSink::Buffering(buffer) => {
                buffer.push(update);
            }
            HookUpdateSink::Mounted(context) => {
                if let Some(context) = context.upgrade() {
                    ElementContextNode::push_update(&context, job_id, update);
                }
            }
        };
    }
}
