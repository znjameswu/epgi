use std::{
    sync::atomic::{Ordering, Ordering::*},
    time::Instant,
};

use hashbrown::HashSet;

use crate::{
    foundation::{Inlinable64Vec, PtrEq},
    tree::AweakElementContextNode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(u64);

// The layout of JobId is (From high bit to low bit)
// 47 bit of frame counter, 1 bit of syncness, 16 bit of job counter
impl JobId {
    const N_BITS_JOB_COUNTER: u8 = 16;
    // const INITIAL: Self = JobId(1 << (Self::N_BITS_JOB_COUNTER + 1));
    pub fn spawning_frame(&self) -> u64 {
        return self.0 >> (Self::N_BITS_JOB_COUNTER + 1);
    }
    pub fn is_sync(&self) -> bool {
        return (self.0 & (1 << Self::N_BITS_JOB_COUNTER)) == 0;
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct JobPriority {
    deadline: std::time::Instant,
    job_id: JobId,
}

impl JobPriority {
    pub fn is_sync(&self) -> bool {
        self.job_id.is_sync()
    }
}

impl PartialOrd for JobPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JobPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match other.is_sync().cmp(&self.is_sync()) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.deadline.cmp(&other.deadline) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.job_id.cmp(&other.job_id)
    }
}

pub struct JobConf {
    priority: JobPriority,
    roots: HashSet<PtrEq<AweakElementContextNode>>,
}

impl JobConf {
    pub fn id(&self) -> JobId {
        self.priority.job_id
    }
    pub fn is_sync(&self) -> bool {
        self.priority.is_sync()
    }
    pub(crate) fn priority_ref(&self) -> &JobPriority {
        &self.priority
    }
    pub(crate) fn roots_ref(&self) -> &HashSet<PtrEq<AweakElementContextNode>> {
        &self.roots
    }
    pub(crate) fn roots_mut(&mut self) -> &mut HashSet<PtrEq<AweakElementContextNode>> {
        &mut self.roots
    }
}

pub struct JobBuilder {
    pub(super) conf: JobConf,
    pub(super) existing_sequenced_jobs: Inlinable64Vec<JobId>,
}

impl JobBuilder {
    pub fn new(job_id: JobId, deadline: Instant) -> Self {
        Self {
            conf: JobConf {
                priority: JobPriority { deadline, job_id },
                roots: Default::default(),
            },
            existing_sequenced_jobs: Default::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.conf.roots.is_empty() && self.existing_sequenced_jobs.is_empty()
    }

    pub fn id(&self) -> JobId {
        self.conf.id()
    }

    pub(crate) fn add_root(
        &mut self,
        node: AweakElementContextNode,
        sequenced_jobs: impl IntoIterator<Item = JobId>,
    ) {
        self.conf.roots_mut().insert(PtrEq(node));
        self.existing_sequenced_jobs.extend(sequenced_jobs);
    }
}

pub(super) struct AtomicJobIdCounter(portable_atomic::AtomicU64);

impl AtomicJobIdCounter {
    const BITS_JOB_COUNTER: u8 = 16;

    pub(super) fn new() -> Self {
        Self(portable_atomic::AtomicU64::new(0))
    }

    pub(super) fn load_frame(&self, order: Ordering) -> u64 {
        return self.0.load(order) >> (Self::BITS_JOB_COUNTER + 1);
    }

    pub(super) fn increment_frame(&self) {
        loop {
            let prev = self.0.load(Relaxed);
            let new = (prev >> (Self::BITS_JOB_COUNTER + 1) + 1) << (Self::BITS_JOB_COUNTER + 1);
            if let Ok(_) = self.0.compare_exchange_weak(prev, new, Relaxed, Relaxed) {
                break;
            }
        }
    }

    pub(super) fn generate_sync_job_id(&self) -> JobId {
        let state = self.0.fetch_add(1, Relaxed);
        if (state + 1) & (1 << Self::BITS_JOB_COUNTER) != 0 {
            panic!("Too many jobs generated during one frame!")
        }
        return JobId(state & !(1 << Self::BITS_JOB_COUNTER));
    }

    pub(super) fn generate_async_job_id(&self) -> JobId {
        let state = self.0.fetch_add(1, Relaxed);
        if (state + 1) & (1 << Self::BITS_JOB_COUNTER) != 0 {
            panic!("Too many jobs generated during one frame!")
        }
        return JobId(state | (1 << Self::BITS_JOB_COUNTER));
    }
}
