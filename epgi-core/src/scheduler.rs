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
    common::{
        AweakAnyElementNode, AweakAnyRenderObject, AweakElementContextNode, WorkContext, WorkHandle,
    },
    foundation::{Asc, MpscQueue, PtrEq, SyncMutex, SyncRwLock},
    sync::{CommitBarrier, TreeScheduler},
};

// The following unsafe code is following https://users.rust-lang.org/t/uninitialised-static-mut/62215/3
struct SchedulerHandleCell(UnsafeCell<MaybeUninit<SchedulerHandle>>);

unsafe impl Sync for SchedulerHandleCell where SchedulerHandle: Sync {}

static _GLOBAL_SCHEDULER_HANDLE: SchedulerHandleCell =
    SchedulerHandleCell(UnsafeCell::new(MaybeUninit::uninit()));

pub fn get_current_scheduler() -> &'static SchedulerHandle {
    // https://users.rust-lang.org/t/uninitialised-static-mut/62215/3
    unsafe { &*(*_GLOBAL_SCHEDULER_HANDLE.0.get()).as_ptr() }
}

unsafe fn setup_scheduler() {
    let scheduler_ref = unsafe { &mut *_GLOBAL_SCHEDULER_HANDLE.0.get() };
    *scheduler_ref = MaybeUninit::new(todo!());
}

pub struct SchedulerHandle {
    pub sync_threadpool: rayon::ThreadPool,
    pub async_threadpool: rayon::ThreadPool,

    scheduler_inbox: Asc<SchedulerInbox>,
    is_executing_sync: AtomicBool,

    // mode: LatencyMode,
    nodes_needing_paint: MpscQueue<AweakAnyRenderObject>,
    nodes_needing_layout: MpscQueue<AweakAnyRenderObject>,
}

impl SchedulerHandle {
    fn new() -> Self {
        todo!()
    }

    pub(crate) fn schedule_reorder_async_work(&self, node: AweakAnyElementNode) {
        self.scheduler_inbox
            .other_scheduler_tasks
            .push(SchedulerTask::ReorderAsyncWork { node });
        self.scheduler_inbox.new_scheduler_task.notify(usize::MAX);
    }

    pub(crate) fn schedule_reorder_provider_reservation(&self, context: AweakElementContextNode) {
        self.scheduler_inbox
            .other_scheduler_tasks
            .push(SchedulerTask::ReorderProviderReservation { context });
        self.scheduler_inbox.new_scheduler_task.notify(usize::MAX);
    }

    pub(crate) fn schedule_async_yield_subtree(
        &self,
        node: AweakAnyElementNode,
        work_context: Asc<WorkContext>,
        work_handle: WorkHandle,
        commit_barrier: CommitBarrier,
    ) {
        self.scheduler_inbox
            .other_scheduler_tasks
            .push(SchedulerTask::AsyncYieldSubtree {
                node,
                work_context,
                work_handle,
                commit_barrier,
            })
    }

    pub(crate) fn mark_boundary_needs_layout(&self, object: AweakAnyRenderObject) {
        self.scheduler_inbox
            .boundaries_needing_relayout
            .lock()
            .insert(PtrEq(object));
    }
}

// TODO: BuildAndLayout vs other event can be modeled as RwLock.
enum SchedulerTask {
    NewFrame {
        frame_id: NonZeroU64,
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

struct SchedulerInbox {
    // requested_new_frame: AtomicBool,
    // occupy_node_requests: MpscQueue<()>,
    // event: event_listener::Event,
    request_shutdown: AtomicBool,
    request_frame_id: AtomicU64,
    other_scheduler_tasks: MpscQueue<SchedulerTask>,
    new_scheduler_task: event_listener::Event,
    accumulated_jobs: SyncMutex<Vec<JobBuilder>>,
    boundaries_needing_relayout: SyncMutex<HashSet<PtrEq<AweakAnyRenderObject>>>,
}

impl SchedulerInbox {
    fn try_recv(&self) -> Option<SchedulerTask> {
        if self.request_shutdown.load(Acquire) {
            return Some(SchedulerTask::Shutdown);
        }
        if let Some(frame_id) = NonZeroU64::new(self.request_frame_id.load(Acquire)) {
            return Some(SchedulerTask::NewFrame { frame_id });
        }
        if let Some(e) = self.other_scheduler_tasks.pop() {
            return Some(e);
        }
        return None;
    }
    fn recv(&self) -> SchedulerTask {
        loop {
            if let Some(e) = self.try_recv() {
                return e;
            }
            let listener = self.new_scheduler_task.listen();
            if let Some(e) = self.try_recv() {
                return e;
            }
            listener.wait();
        }
    }
}

struct Scheduler {
    tree_scheduler: Asc<SyncRwLock<TreeScheduler>>,
    job_batcher: JobBatcher,
}

impl Scheduler {
    fn start_event_loop(mut self, inbox: &SchedulerInbox) {
        let jobs = Asc::new(SyncMutex::new(Vec::default()));
        loop {
            let task = inbox.recv();
            use SchedulerTask::*;
            match task {
                NewFrame { frame_id } => {
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
                        { std::mem::take(&mut *inbox.boundaries_needing_relayout.lock()) };
                    tree_scheduler.perform_layout(boundaries_needing_relayout);
                    // We don't have RwLock downgrade in std, this is to simulate it by re-reading while blocking the event loop.
                    // TODO: Parking_lot owned downgradable guard
                    drop(tree_scheduler);
                    let read_guard = self.tree_scheduler.read();
                    let tree_scheduler = self.tree_scheduler.clone();
                    let paint_started_event = event_listener::Event::new();
                    let paint_started = paint_started_event.listen();
                    get_current_scheduler().sync_threadpool.spawn(move || {
                        let scheduler = tree_scheduler.read();
                        paint_started_event.notify(usize::MAX);
                        scheduler.perform_paint();
                    });
                    paint_started.wait();
                    drop(read_guard);
                }
                PointerEvent {} => {}
                ReorderAsyncWork { node } => {
                    let tree_scheduler = self.tree_scheduler.clone();
                    get_current_scheduler().sync_threadpool.spawn(move || {
                        let tree_scheduler = tree_scheduler.read();
                        tree_scheduler.reorder_async_work(node);
                    })
                }
                ReorderProviderReservation { context } => {
                    let tree_scheduler = self.tree_scheduler.clone();
                    get_current_scheduler().sync_threadpool.spawn(move || {
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
