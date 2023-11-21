use crate::{
    foundation::{Asc, AsyncMpscSender, SyncRwLock},
    sync::CommitBarrier,
    tree::{AweakAnyElementNode, AweakElementContextNode, WorkContext, WorkHandle},
};

pub use crate::sync::BuildScheduler;

use super::{FrameResults, JobBatcher, SchedulerHandle};

// TODO: BuildAndLayout vs other event can be modeled as RwLock.
pub(super) enum SchedulerTask {
    NewFrame {
        frame_id: u64,
        requesters: Vec<AsyncMpscSender<FrameResults>>,
    },
    ReorderAsyncWork {
        node: AweakAnyElementNode,
    },
    ReorderProviderReservation {
        context: AweakElementContextNode, // TODO: Reorder reservation can be done in parallel
    },
    AsyncYieldSubtree {
        node: AweakAnyElementNode,
        work_context: Asc<WorkContext>,
        work_handle: WorkHandle,
        commit_barrier: CommitBarrier,
    },
    PointerEvent {},
    Shutdown,
}

pub struct Scheduler {
    build_scheduler: Asc<SyncRwLock<BuildScheduler>>,
    job_batcher: JobBatcher,
}

impl Scheduler {
    pub fn new(build_scheduler: BuildScheduler) -> Self {
        Self {
            build_scheduler: Asc::new(SyncRwLock::new(build_scheduler)),
            job_batcher: JobBatcher::new(),
        }
    }
    pub fn start_event_loop(mut self, handle: &SchedulerHandle) {
        // handle.push_layer_render_objects_needing_paint(self.build_scheduler.roo)
        let tasks = &handle.task_rx;
        loop {
            let task = tasks.recv();
            use SchedulerTask::*;
            match task {
                NewFrame {
                    frame_id,
                    requesters,
                } => {
                    let mut build_scheduler = self.build_scheduler.write();
                    // let commited_async_batches = build_scheduler.commit_completed_async_batches(&mut self.job_batcher);
                    // for commited_async_batch in commited_async_batches {
                    //     self.job_batcher.remove_commited_batch(&commited_async_batch)
                    // }
                    let new_jobs = {
                        let _guard = handle.sync_job_building_lock.write();
                        handle.job_id_counter.increment_frame();
                        std::mem::take(&mut *handle.accumulated_jobs.lock())
                    };
                    self.job_batcher.update_with_new_jobs(new_jobs);
                    let updates = self.job_batcher.get_batch_updates();
                    build_scheduler.apply_batcher_result(updates);
                    // build_scheduler.dispatch_async_batches();
                    let commited_sync_batch = build_scheduler.dispatch_sync_batch();
                    if let Some(commited_sync_batch) = commited_sync_batch {
                        self.job_batcher.remove_commited_batch(&commited_sync_batch);
                    }
                    // let commited_async_batches = build_scheduler.commit_completed_async_batches(&mut self.job_batcher);
                    // for commited_async_batch in commited_async_batches {
                    //     self.job_batcher.remove_commited_batch(&commited_async_batch)
                    // }
                    build_scheduler.perform_layout();
                    // We don't have RwLock downgrade in std, this is to simulate it by re-reading while blocking the event loop.
                    // TODO: Parking_lot owned downgradable guard
                    drop(build_scheduler);
                    let read_guard = self.build_scheduler.read();
                    let build_scheduler = self.build_scheduler.clone();
                    let layer_needing_repaint =
                        { std::mem::take(&mut *handle.layer_needing_repaint.lock()) };
                    let paint_started_event = event_listener::Event::new();
                    let paint_started = paint_started_event.listen();
                    handle.sync_threadpool.spawn(move || {
                        let scheduler = build_scheduler.read();
                        paint_started_event.notify(usize::MAX);
                        scheduler.perform_paint(layer_needing_repaint);
                        let result = scheduler.perform_composite();
                        // TODO: Composition
                        for requester in requesters {
                            let _ = requester.try_send(FrameResults {
                                composited: result.clone(),
                                id: frame_id,
                            }); // TODO: log failure
                        }
                    });
                    paint_started.wait();
                    drop(read_guard);
                }
                PointerEvent {} => {}
                ReorderAsyncWork { node } => {
                    let build_scheduler = self.build_scheduler.clone();
                    handle.sync_threadpool.spawn(move || {
                        let build_scheduler = build_scheduler.read();
                        build_scheduler.reorder_async_work(node);
                    })
                }
                ReorderProviderReservation { context } => {
                    let build_scheduler = self.build_scheduler.clone();
                    handle.sync_threadpool.spawn(move || {
                        let build_scheduler = build_scheduler.read();
                        build_scheduler.reorder_provider_reservation(context);
                    })
                }
                AsyncYieldSubtree {
                    node,
                    work_context,
                    work_handle,
                    commit_barrier,
                } => todo!(),
                Shutdown => break,
            }
        }
    }
}
