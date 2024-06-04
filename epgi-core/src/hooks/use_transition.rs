use crate::{
    scheduler::{get_current_scheduler, JobBuilder},
    tree::BuildContext,
};

use super::SetState;

impl<'a> BuildContext<'a> {
    pub fn use_transition(&mut self) -> (bool, StartTransition) {
        let (pending, set_pending) = self.use_state(false);
        (pending, StartTransition { set_pending })
    }
}

#[derive(Clone)]
pub struct StartTransition {
    set_pending: SetState<bool>,
}

impl StartTransition {
    pub fn start(&self, transition: impl FnOnce(&mut JobBuilder), job_builder: &mut JobBuilder) {
        // One important requirement for transition is that:
        // The `pending` flip job and the transition job MUST be pushed (i.e. complete building) at the same time
        //
        // If the flip job is pushed and the transition job is not:
        // It is possible that the second flip job will be dispatched without realizing the exisitence of the first flip job
        // And a wrong commit sequence can occur: first flip -> second flip -> first transstion (invalid state) -> second transition.
        //
        // If the transition job is pushed and the flip job is not:
        // It is just plain stupid
        //
        // How we uphold such requirement:
        // If start transtion is called in a sync job builder,
        // then SchedulerHandle::create_sync_job guarantees that the async job creation will be completed before the sync job
        // If start transtion is called in an async job builder,
        // then we directly merge flip job and transition job into one
        //
        // Why merge jobs inside async job builder?
        // 1. ~~Because async job building is not synced with scheduler, would require extra API to push multiple jobs atomically~~
        // 2. Because as long as they are pushed at the same time, the two jobs would be sequenced and end up batched up anyway. So why not merge it right here?
        if job_builder.id().is_sync() {
            self.set_pending.set(true, job_builder);
            get_current_scheduler().create_async_job(|job_builder| {
                transition(job_builder);
                self.set_pending.set(false, job_builder);
            });
        } else {
            transition(job_builder)
        }
    }
}
