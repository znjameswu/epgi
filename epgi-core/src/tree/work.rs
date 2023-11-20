use core::sync::atomic::{AtomicBool, Ordering::*};

use hashbrown::HashMap;

use crate::{
    foundation::{Asc, Inlinable64Vec, Provide, TypeKey},
    scheduler::{BatchConf, JobId, LanePos},
};

#[derive(Clone)]
pub(crate) struct Work<ArcWidget> {
    pub(crate) widget: Option<ArcWidget>,
    pub(crate) context: Asc<WorkContext>,
}

#[derive(Clone)]
pub struct WorkContext {
    pub lane_pos: LanePos,
    pub(crate) batch: Asc<BatchConf>,
    // This contains two types of providers in the ancester work chain in this this batch: 1. subsrcibed providers 2. modified providers by this batch.
    pub(crate) reserved_provider_values: HashMap<TypeKey, Asc<dyn Provide>>,
}

impl WorkContext {
    pub(crate) fn job_ids(&self) -> &Inlinable64Vec<JobId> {
        &self.batch.job_ids
    }
}

#[derive(Clone)]
pub struct WorkHandle(Asc<AtomicBool>);

impl WorkHandle {
    pub fn new() -> Self {
        Self(Asc::new(AtomicBool::new(false)))
    }
    pub fn is_aborted(&self) -> bool {
        self.0.load(Relaxed)
    }

    pub fn abort(&self) {
        self.0.store(true, Relaxed)
    }
}
