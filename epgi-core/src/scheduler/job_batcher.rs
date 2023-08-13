use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use smallvec::smallvec;

use crate::{
    tree::AweakElementContextNode,
    foundation::{Asc, Inlinable64Vec, InlinableUsizeVec, MpscQueue, SyncMutex},
};

use super::{BatchConf, BatchId, JobBuilder, JobConf, JobId, JobPriority};

pub(crate) struct JobData {
    conf: JobConf,
    batch: Option<BatchId>,
    // In theory, sequenced sync jobs would also affect our batching.
    // However, if the batcher is working well, sync jobs are known to be batched in the sync batch. Tracking them is meaningless.
    // Also, tracking sequenced async jobs can allow us to verify the closedness of async batches more easily.
    sequenced_jobs: Inlinable64Vec<JobId>,
}

impl JobData {
    fn new(conf: JobConf) -> Self {
        Self {
            conf,
            batch: None,
            sequenced_jobs: Inlinable64Vec::new(),
        }
    }
    fn is_sync(&self) -> bool {
        self.conf.is_sync()
    }
    fn priority(&self) -> &JobPriority {
        self.conf.priority_ref()
    }
    fn spawning_frame(&self) -> u64 {
        self.conf.id().spawning_frame()
    }
}

enum JobInterference {
    Sequenced(JobId, JobId),
    Racing(JobId, JobId),
}

pub(crate) struct JobSchedulerSink {
    pub(crate) new_jobs: SyncMutex<Vec<JobConf>>,
}

pub(crate) struct JobSchedulerSinkInner {
    pub(crate) new_jobs: SyncMutex<Vec<JobConf>>,
}
pub(crate) struct RootMarkResult {
    pub(crate) id: JobId,
    pub(crate) node: AweakElementContextNode,
    pub(crate) existing_sequenced_jobs: Inlinable64Vec<JobId>,
}

pub(crate) struct JobBatcher {
    job_datas: HashMap<JobId, JobData>,
    sink: Asc<JobSchedulerSink>,

    completed_async_batches: Asc<MpscQueue<BatchId>>,
    batches: HashMap<BatchId, Asc<BatchConf>>,
    batch_id_counter: u64,
}

pub(crate) struct BatchResult {
    pub(crate) expired_batches: Inlinable64Vec<BatchId>,
    pub(crate) new_async_batches: InlinableUsizeVec<Asc<BatchConf>>,
    pub(crate) new_sync_batch: Option<Asc<BatchConf>>,
}

/// Responsibility: Batch up jobs, report outdated batches, report batch conflict (recommend executable batches?)
impl JobBatcher {
    pub(super) fn new(sink: JobSchedulerSink) -> Self {
        Self {
            job_datas: Default::default(),
            sink: todo!(),
            completed_async_batches: todo!(),
            batches: Default::default(),
            batch_id_counter: 1,
        }
    }

    pub(super) fn update_with_new_jobs(&mut self, job_builders: Vec<JobBuilder>) {
        #[cfg(debug_assertions)]
        self.debug_validate_state_integrity();

        let (new_job_datas, job_feedbacks): (Vec<_>, Vec<_>) = job_builders
            .into_iter()
            .map(
                |JobBuilder {
                     conf,
                     existing_sequenced_jobs,
                 }| {
                    let id = conf.id();
                    ((id, JobData::new(conf)), (id, existing_sequenced_jobs))
                },
            )
            .unzip();

        self.job_datas.extend(new_job_datas);

        for (job_id, existing_sequenced_jobs) in job_feedbacks {
            for sequenced_job_id in existing_sequenced_jobs {
                if let Some([job_data, sequenced_job_data]) =
                    self.job_datas.get_many_mut([&job_id, &sequenced_job_id])
                {
                    job_data.sequenced_jobs.push(sequenced_job_id);
                    sequenced_job_data.sequenced_jobs.push(job_id);
                } else {
                    debug_assert!(
                        sequenced_job_id.spawning_frame() <= job_id.spawning_frame(),
                        "A job cannot be sequenced with an job from a future frame"
                    );
                    debug_assert!(
                        sequenced_job_id.spawning_frame() < job_id.spawning_frame(),
                        "Job from the same frame should be inserted at the same time"
                    );
                }
            }
        }
    }
    pub(crate) fn remove_commited_batch(&mut self, commited_batch: &BatchId) {
        let commited_batch_confs = self
            .batches
            .remove(commited_batch)
            .expect("You should not commit a dead batch");
        for commited_job in commited_batch_confs.jobs.iter() {
            let job_data = self.job_datas.remove(commited_job).unwrap();
            for sequenced_job in job_data.sequenced_jobs.into_iter() {
                let other_sequenced_jobs = &mut self
                    .job_datas
                    .get_mut(&sequenced_job)
                    .unwrap()
                    .sequenced_jobs;
                other_sequenced_jobs.swap_remove(
                    other_sequenced_jobs
                        .iter()
                        .position(|x| x == commited_job)
                        .unwrap(),
                );
            }
        }
    }

    pub(super) fn get_batch_updates(&mut self) -> BatchResult {
        #[cfg(debug_assertions)]
        self.debug_validate_state_integrity();
        // #[cfg(debug_assertions)]
        // self.debug_validate_no_expired_sync_jobs();

        // Remove batches that become open under the sequenced relation
        //
        // All the batches we have now should be the async batches left from previous frames. If they become open (whether they are sequenced with another async job or a current-frame sync job), they are absolutely expired.
        let mut expired_batches = Inlinable64Vec::default();
        let mut batched_job_count = 0;
        self.batches.retain(|&batch_id, batch_conf| {
            let is_closed = batch_conf.jobs.iter().all(|job_id| {
                return self.job_datas[job_id]
                    .sequenced_jobs
                    .iter()
                    .all(
                        |sequenced_job_id| match self.job_datas[sequenced_job_id].batch {
                            Some(sequenced_job_batch_id) => batch_id == sequenced_job_batch_id,
                            None => false,
                        },
                    );
            });
            if !is_closed {
                debug_assert!(
                    !batch_conf.is_sync(),
                    "A sync batch should always complete successfully, not expire."
                );
                // Remove batch that becomes open
                expired_batches.push(batch_id);
                for job_id in batch_conf.jobs.iter() {
                    self.job_datas.get_mut(job_id).unwrap().batch = None;
                }
            }
            batched_job_count += batch_conf.jobs.len();
            is_closed
        });

        #[cfg(debug_assertions)]
        self.debug_validate_state_integrity();
        // #[cfg(debug_assertions)]
        // self.debug_validate_async_batch_closedness();

        // This is the breadth-first visit function that starts from a initial set of job_ids
        fn bfs_visit<F: Fn(&JobData) -> bool>(
            mut batch_jobs: Inlinable64Vec<JobId>,
            job_datas: &mut HashMap<JobId, JobData>,
            new_batch_id: BatchId,
            mut priority: JobPriority,
            should_visit: F,
        ) -> BatchConf {
            let mut num_searched = 0;
            let mut root_sets = Vec::with_capacity(batch_jobs.capacity());
            while num_searched < batch_jobs.len() {
                let job_id = batch_jobs[num_searched];
                let job_data = job_datas.get_mut(&job_id).unwrap();
                root_sets.push(job_data.conf.roots_ref().clone());
                priority = priority.min(job_data.priority().clone());
                for sequenced_job_id in job_data.sequenced_jobs.clone() {
                    let sequenced_job_data = job_datas.get_mut(&sequenced_job_id).unwrap();
                    if should_visit(&sequenced_job_data) {
                        if sequenced_job_data.batch.is_none() {
                            sequenced_job_data.batch = Some(new_batch_id);
                            batch_jobs.push(sequenced_job_id);
                        } else {
                            debug_assert_eq!(sequenced_job_data.batch, Some(new_batch_id));
                        }
                    }
                }
                num_searched += 1;
            }
            fn merge_sets<T: Clone + Hash + Eq>(mut sets: Vec<HashSet<T>>) -> HashSet<T> {
                if sets.is_empty() {
                    return HashSet::<T>::default();
                }
                sets.sort_unstable_by_key(|set| set.len());
                // Clone the largest set to save us from repeated insertion
                let mut res = sets.pop().unwrap().clone();
                for set in sets.into_iter().rev() {
                    res.extend(set);
                }
                return res;
            }
            let roots = merge_sets(root_sets);
            BatchConf {
                jobs: batch_jobs,
                id: new_batch_id,
                priority,
                roots,
            }
        }

        // Collect sync batch
        let sync_batch_id = BatchId(self.batch_id_counter);
        self.batch_id_counter = self.batch_id_counter.wrapping_add(1);
        let mut sync_job_priority = None;
        let sync_job_ids = self
            .job_datas
            .iter_mut()
            .filter_map(|(&job_id, job_data)| {
                if job_id.is_sync() {
                    job_data.batch = Some(sync_batch_id);
                    let new_priority = job_data.priority().clone();
                    match &mut sync_job_priority {
                        Some(x) => *x = std::cmp::min(*x, new_priority),
                        None => sync_job_priority = Some(new_priority),
                    }
                    Some(job_id)
                } else {
                    None
                }
            })
            .collect::<Inlinable64Vec<_>>();
        let new_sync_batch = if let Some(sync_job_priority) = sync_job_priority {
            let sync_batch = Asc::new(bfs_visit(
                sync_job_ids,
                &mut self.job_datas,
                sync_batch_id,
                sync_job_priority,
                JobData::is_sync,
            ));
            debug_assert!(sync_batch.is_sync());
            self.batches.insert(sync_batch_id, sync_batch.clone());
            Some(sync_batch)
        } else {
            self.batch_id_counter = self.batch_id_counter.wrapping_sub(1);
            None
        };

        // Collect async batch
        let mut new_async_batches = InlinableUsizeVec::default();
        let keys = self.job_datas.keys().cloned().collect::<Vec<_>>();
        for job_id in keys {
            let job_data = &mut self.job_datas.get_mut(&job_id).unwrap();
            if job_data.batch.is_none() {
                let new_batch_id = BatchId(self.batch_id_counter);
                self.batch_id_counter = self.batch_id_counter.wrapping_add(1);
                job_data.batch = Some(new_batch_id);
                let priority = job_data.priority().clone();
                let batch = Asc::new(bfs_visit(
                    smallvec![job_id],
                    &mut self.job_datas,
                    new_batch_id,
                    priority,
                    |job_data| job_data.batch.is_none(),
                ));
                debug_assert!(!batch.is_sync());
                new_async_batches.push(batch);
            }
        }

        self.batches.extend(
            new_async_batches
                .iter()
                .map(|new_batch| (new_batch.id, new_batch.clone())),
        );

        debug_assert!(
            self.job_datas
                .iter()
                .all(|(_, job_data)| job_data.batch.is_some()),
            "All jobs should have been assigned a batch"
        );

        // // Recalculate inter-batch relations
        // let batch_conflicts = self
        //     .batches
        //     .iter()
        //     .map(|(&batch_id, batch_data)| {
        //         (
        //             batch_id,
        //             batch_data
        //                 .jobs
        //                 .iter()
        //                 .flat_map(|job_id| {
        //                     self.job_datas[job_id]
        //                         .conflicters
        //                         .iter()
        //                         .map(|conflictor_id| self.job_datas[conflictor_id].batch.unwrap())
        //                 })
        //                 .collect::<HashSet<_>>(),
        //         )
        //     })
        //     .collect::<HashMap<_, _>>();

        // Publish New batch

        #[cfg(debug_assertions)]
        self.debug_validate_state_integrity();
        // #[cfg(debug_assertions)]
        // self.debug_validate_async_batch_closedness();
        return BatchResult {
            expired_batches,
            new_sync_batch,
            new_async_batches,
        };
    }

    pub(super) fn debug_validate_state_integrity(&self) {
        for (batch_id, batch_conf) in self.batches.iter() {
            assert_eq!(
                *batch_id, batch_conf.id,
                "Batch data map should contain correct batch data for given batch id"
            )
        }
        for (job_id, job_data) in self.job_datas.iter() {
            if let Some(job_batch) = &job_data.batch {
                assert!(
                    self.batches
                        .get(job_batch)
                        .expect("A job should not have its batch id pointing to a dead batch")
                        .jobs
                        .contains(job_id),
                    "A job should not have its batch id pointing to a batch that does not contain this job"
                );
            }
            for sequenced_job_id in job_data.sequenced_jobs.iter() {
                assert_ne!(
                    *sequenced_job_id, *job_id,
                    "A job should not have list itself as its sequenced job"
                );
                assert!(
                    self.job_datas
                        .get(sequenced_job_id)
                        .expect("The sequenced job of a living job must be alive as well, since they are either executed in the same batch or one get desequenced on completion of the sync batch.")
                        .sequenced_jobs
                        .contains(job_id),
                    "The sequenced relation should be bi-lateral"
                )
            }
            // for conflicter_id in job_data.conflicters.iter() {
            //     assert_ne!(
            //         *conflicter_id, *job_id,
            //         "A job should not have itself as its conflicter"
            //     );
            //     if let Some(conflicter_data) = self.job_datas.get(conflicter_id) {
            //         assert!(
            //             conflicter_data.conflicters.contains(job_id),
            //             "Conflict should be bi-lateral"
            //         )
            //     }
            // }
        }
        for (batch_id, batch_conf) in self.batches.iter() {
            for job_id in batch_conf.jobs.iter() {
                let job_data = self
                    .job_datas
                    .get(job_id)
                    .expect("A job inside a living batch should be living");
                let job_batch = job_data
                    .batch
                    .expect("A job inside a living batch should have a batch id");
                assert_eq!(
                    job_batch, *batch_id,
                    "A job inside a living batch should have the batch id of that batch"
                );
            }
        }
    }
    pub(super) fn debug_validate_sync_jobs_from_same_frame(&self) {
        let sync_job_spawning_frame: HashSet<_> = self
            .job_datas
            .values()
            .filter(|&data| data.is_sync())
            .map(|data| data.spawning_frame())
            .collect();
        assert!(sync_job_spawning_frame.len() == 0 || sync_job_spawning_frame.len() == 1,
        "All sync jobs being processed at any point of time should be spawned in the same frame. That is, there should be no left-over sync jobs from the earlier frames, nor any future sync jobs from a future frame.")
    }

    // fn debug_validate_async_batch_closedness(&self) {
    //     for (batch_id, batch_data) in self.batches.iter() {
    //         for job_id in batch_data.jobs.iter() {
    //             let job_data = self
    //                 .job_datas
    //                 .get(job_id)
    //                 .expect("A job inside a living batch should be living");
    //             for sequenced_asyn_job_id in job_data.sequenced_jobs.iter() {
    //                 let sequenced_job_batch = self.job_datas
    //                     .get(sequenced_asyn_job_id)
    //                     .expect(
    //                         "A job inside a living batch should have all its entangled jobs living",
    //                     )
    //                     .batch
    //                     .as_ref()
    //                     .expect("A job inside a living batch should have all its entangled jobs having a batch id");
    //                 assert_eq!(*sequenced_job_batch, *batch_id, "A job inside a living batch should have all its entangled jobs having the same batch id as itself");
    //             }
    //         }
    //     }
    // }
}
