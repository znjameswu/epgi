use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    num::NonZeroU64,
    sync::atomic::{AtomicBool, Ordering::*},
};

use hashbrown::HashSet;
use portable_atomic::AtomicU64;

use crate::{
    common::{
        AweakAnyElementNode, AweakAnyRenderObject, AweakElementContextNode, WorkContext, WorkHandle,
    },
    foundation::{Asc, MpscQueue, PtrEq, SyncMutex, SyncRwLock},
    sync::{CommitBarrier, TreeScheduler},
};

use super::{JobBuilder, SchedulerTask};

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

    pub(super) task_rx: SchedulerTaskReceiver,
    pub(crate) new_frame_ready: event_listener::Event,
    is_executing_sync: AtomicBool,

    // mode: LatencyMode,
    nodes_needing_paint: MpscQueue<AweakAnyRenderObject>,
    nodes_needing_layout: MpscQueue<AweakAnyRenderObject>,

    pub(super) accumulated_jobs: SyncMutex<Vec<JobBuilder>>,
    pub(super) boundaries_needing_relayout: SyncMutex<HashSet<PtrEq<AweakAnyRenderObject>>>,
}

impl SchedulerHandle {
    fn new() -> Self {
        todo!()
    }

    pub(crate) fn schedule_new_frame(&self) {
        
    }

    pub(crate) fn schedule_reorder_async_work(&self, node: AweakAnyElementNode) {
        self.task_rx
            .other_scheduler_tasks
            .push(SchedulerTask::ReorderAsyncWork { node });
        self.task_rx.new_scheduler_task.notify(usize::MAX);
    }

    pub(crate) fn schedule_reorder_provider_reservation(&self, context: AweakElementContextNode) {
        self.task_rx
            .other_scheduler_tasks
            .push(SchedulerTask::ReorderProviderReservation { context });
        self.task_rx.new_scheduler_task.notify(usize::MAX);
    }

    pub(crate) fn schedule_async_yield_subtree(
        &self,
        node: AweakAnyElementNode,
        work_context: Asc<WorkContext>,
        work_handle: WorkHandle,
        commit_barrier: CommitBarrier,
    ) {
        self.task_rx
            .other_scheduler_tasks
            .push(SchedulerTask::AsyncYieldSubtree {
                node,
                work_context,
                work_handle,
                commit_barrier,
            })
    }

    pub(crate) fn mark_boundary_needs_layout(&self, object: AweakAnyRenderObject) {
        self.boundaries_needing_relayout
            .lock()
            .insert(PtrEq(object));
    }
}

pub(super) struct SchedulerTaskReceiver {
    // requested_new_frame: AtomicBool,
    // occupy_node_requests: MpscQueue<()>,
    // event: event_listener::Event,
    request_shutdown: AtomicBool,
    request_frame_id: AtomicU64,
    other_scheduler_tasks: MpscQueue<SchedulerTask>,
    new_scheduler_task: event_listener::Event,
}

impl SchedulerTaskReceiver {
    pub(super) fn try_recv(&self) -> Option<SchedulerTask> {
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
    pub(super) fn recv(&self) -> SchedulerTask {
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
