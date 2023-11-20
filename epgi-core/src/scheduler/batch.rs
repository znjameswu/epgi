use hashbrown::HashSet;

use crate::{
    foundation::{Inlinable64Vec, PtrEq},
    tree::AweakElementContextNode,
};

use super::{JobId, JobPriority};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BatchId(pub u64);

#[derive(Clone)]
pub struct BatchConf {
    pub id: BatchId,
    pub job_ids: Inlinable64Vec<JobId>,
    pub(crate) priority: JobPriority,
    // earliest_job: JobId,
    pub roots: HashSet<PtrEq<AweakElementContextNode>>,
}

impl BatchConf {
    pub fn is_sync(&self) -> bool {
        self.priority.is_sync()
    }
}

pub struct BatchData {
    pub conf: BatchConf,
}
