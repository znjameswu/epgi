mod batch;
mod handle;
mod job;
mod job_batcher;
mod lane;

pub use batch::*;
pub use handle::*;
pub use job::*;
pub use job_batcher::*;
pub use lane::*;

use hashbrown::HashSet;

use portable_atomic::AtomicU64;

use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    num::NonZeroU64,
    sync::atomic::{AtomicBool, Ordering::*},
};

use crate::{
    tree::{
        AweakAnyElementNode, AweakAnyRenderObject, AweakElementContextNode, WorkContext, WorkHandle,
    },
    foundation::{Asc, AsyncMpscSender, MpscQueue, PtrEq, SyncMutex, SyncRwLock},
    sync::CommitBarrier,
};

pub use crate::sync::TreeScheduler;

// TODO: BuildAndLayout vs other event can be modeled as RwLock.
enum SchedulerTask {
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
    tree_scheduler: Asc<SyncRwLock<TreeScheduler>>,
    job_batcher: JobBatcher,
}

impl Scheduler {
    pub fn new(tree_scheduler: TreeScheduler) -> Self {
        Self {
            tree_scheduler: Asc::new(SyncRwLock::new(tree_scheduler)),
            job_batcher: JobBatcher::new(todo!()),
        }
    }
    pub fn start_event_loop(mut self, handle: &SchedulerHandle) {
        let tasks = &handle.task_rx;
        let jobs = Asc::new(SyncMutex::new(Vec::default()));
        loop {
            let task = tasks.recv();
            use SchedulerTask::*;
            match task {
                NewFrame {
                    frame_id,
                    requesters,
                } => {
                    let mut tree_scheduler = self.tree_scheduler.write();
                    tree_scheduler.commit_completed_async_batches(&mut self.job_batcher);
                    let new_jobs = std::mem::take(&mut *jobs.lock());
                    self.job_batcher.update_with_new_jobs(new_jobs);
                    let updates = self.job_batcher.get_batch_updates();
                    tree_scheduler.apply_batcher_result(updates);
                    tree_scheduler.dispatch_async_batches();
                    tree_scheduler.dispatch_sync_batch();
                    tree_scheduler.commit_completed_async_batches(&mut self.job_batcher);
                    let boundaries_needing_relayout =
                        { std::mem::take(&mut *handle.boundaries_needing_relayout.lock()) };
                    // TODO: Skip layout if empty
                    tree_scheduler.perform_layout(boundaries_needing_relayout);
                    // We don't have RwLock downgrade in std, this is to simulate it by re-reading while blocking the event loop.
                    // TODO: Parking_lot owned downgradable guard
                    drop(tree_scheduler);
                    let read_guard = self.tree_scheduler.read();
                    let tree_scheduler = self.tree_scheduler.clone();
                    let paint_started_event = event_listener::Event::new();
                    let paint_started = paint_started_event.listen();
                    handle.sync_threadpool.spawn(move || {
                        let scheduler = tree_scheduler.read();
                        paint_started_event.notify(usize::MAX);
                        scheduler.perform_paint();
                        for requester in requesters {
                            // let layer = todo!();
                            // layer.composite_to();
                        }
                    });
                    paint_started.wait();
                    // handle.new_frame_ready.notify(usize::MAX);
                    drop(read_guard);
                }
                PointerEvent {} => {}
                ReorderAsyncWork { node } => {
                    let tree_scheduler = self.tree_scheduler.clone();
                    handle.sync_threadpool.spawn(move || {
                        let tree_scheduler = tree_scheduler.read();
                        tree_scheduler.reorder_async_work(node);
                    })
                }
                ReorderProviderReservation { context } => {
                    let tree_scheduler = self.tree_scheduler.clone();
                    handle.sync_threadpool.spawn(move || {
                        let tree_scheduler = tree_scheduler.read();
                        tree_scheduler.reorder_provider_reservation(context);
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
