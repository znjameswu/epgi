use core::sync::atomic::{AtomicBool, Ordering::*};

use hashbrown::HashMap;

use crate::{
    foundation::{Asc, Inlinable64Vec, Provide, TypeKey},
    scheduler::{BatchConf, JobId, LanePos},
};

#[derive(Clone)]
pub struct WorkContext {
    pub lane_pos: LanePos,
    pub(crate) batch: Asc<BatchConf>,
    // This contains two types of providers in the ancester work chain in this this batch: 1. subsrcibed providers 2. modified providers by this batch.
    // This field is necessary to cache the modified providers, otherwise every provider read in async batch needs to lock the element node to check for the same batch
    // Since we already have this field, it will also be useful to store alongside other provider values read by this batch.
    //    In some sense, this optimization also guarantees a consistent view of one provider across an entire async batch.
    //    No mid-way update on providers will cause a batch to read two different values.
    //    But this consistency is also enforced by batch aborting mechanism even without this caching field. Therefore, only an optimization, not an extra necessity.
    // The extra overhead to potentially update bookkeeping (i.e. invalidating entries) when encountering descendant provider widgets is inevitable.
    // However, we assume provider widgets to be rare.
    pub(crate) recorded_provider_values: HashMap<TypeKey, Asc<dyn Provide>>,
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
