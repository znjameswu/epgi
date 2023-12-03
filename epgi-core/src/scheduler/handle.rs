use std::{
    any::Any,
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering::*},
    time::Instant,
};

use hashbrown::HashSet;

use crate::{
    foundation::{
        bounded_channel_sync, Asc, MpscQueue, PtrEq, SyncMpscReceiver, SyncMpscSender, SyncMutex,
        SyncRwLock,
    },
    sync::CommitBarrier,
    tree::{
        AweakAnyElementNode, AweakAnyLayerRenderObject, AweakAnyRenderObject,
        AweakElementContextNode, WorkContext, WorkHandle,
    },
};

use super::{AtomicJobIdCounter, JobBuilder, SchedulerTask};

// The following unsafe code is following https://users.rust-lang.org/t/uninitialised-static-mut/62215/3
struct SchedulerHandleCell(UnsafeCell<MaybeUninit<SchedulerHandle>>);

unsafe impl Sync for SchedulerHandleCell where SchedulerHandle: Sync {}

static _GLOBAL_SCHEDULER_HANDLE: SchedulerHandleCell =
    SchedulerHandleCell(UnsafeCell::new(MaybeUninit::uninit()));

pub fn get_current_scheduler() -> &'static SchedulerHandle {
    // https://users.rust-lang.org/t/uninitialised-static-mut/62215/3
    unsafe { &*(*_GLOBAL_SCHEDULER_HANDLE.0.get()).as_ptr() }
}

pub unsafe fn setup_scheduler(scheduler_handle: SchedulerHandle) {
    let scheduler_ref = unsafe { &mut *_GLOBAL_SCHEDULER_HANDLE.0.get() };
    *scheduler_ref = MaybeUninit::new(scheduler_handle);
}

pub struct SchedulerHandle {
    pub sync_threadpool: rayon::ThreadPool,
    pub async_threadpool: rayon::ThreadPool,

    pub(super) task_rx: SchedulerTaskReceiver,

    pub(super) sync_job_building_lock: SyncRwLock<()>,
    pub(super) job_id_counter: AtomicJobIdCounter,

    // mode: LatencyMode,
    nodes_needing_paint: MpscQueue<AweakAnyRenderObject>,
    nodes_needing_layout: MpscQueue<AweakAnyRenderObject>,

    pub(super) accumulated_jobs: SyncMutex<Vec<JobBuilder>>,
    // pub(super) boundaries_needing_relayout: SyncMutex<HashSet<PtrEq<AweakAnyRenderObject>>>,
    pub(super) layer_needing_repaint: SyncMutex<HashSet<PtrEq<AweakAnyLayerRenderObject>>>,
}

impl SchedulerHandle {
    pub fn new(sync_threadpool: rayon::ThreadPool, async_threadpool: rayon::ThreadPool) -> Self {
        Self {
            sync_threadpool,
            async_threadpool,
            task_rx: SchedulerTaskReceiver::new(),
            sync_job_building_lock: SyncRwLock::new(()),
            job_id_counter: AtomicJobIdCounter::new(),
            // is_executing_sync: (),
            nodes_needing_paint: Default::default(),
            nodes_needing_layout: Default::default(),
            accumulated_jobs: Default::default(),
            // boundaries_needing_relayout: Default::default(),
            layer_needing_repaint: Default::default(),
        }
    }

    pub fn create_sync_job(&self, builder: impl FnOnce(&mut JobBuilder)) {
        // Note the additional lock compared to the async version.
        // This lock is to ensure the scheduler could not process jobs before all sync jobs create in the previous frame have finished building.
        // Therefore, the scheduler will never see an outdated sync job from previous frames.
        // However, it also means that blocking in the job builder will block the entire event loop.
        let guard = self.sync_job_building_lock.read();
        let job_id = self.job_id_counter.generate_sync_job_id();
        let mut job_builder = JobBuilder::new(job_id, Instant::now());
        builder(&mut job_builder);
        if !job_builder.is_empty() {
            get_current_scheduler()
                .accumulated_jobs
                .lock()
                .push(job_builder);
        }
        drop(guard);
    }

    pub fn create_async_job(&self, builder: impl FnOnce(&mut JobBuilder)) {
        // Note: if the builder takes a long time, then we can see this very outdated async job in a later frame. Which is perfectly fine
        let job_id = self.job_id_counter.generate_async_job_id();
        let mut job_builder = JobBuilder::new(job_id, Instant::now());
        builder(&mut job_builder);
        if !job_builder.is_empty() {
            get_current_scheduler()
                .accumulated_jobs
                .lock()
                .push(job_builder);
        }
    }

    pub fn request_new_frame(&self) -> SyncMpscReceiver<FrameResults> {
        let (tx, rx) = bounded_channel_sync(1);
        {
            self.task_rx.request_frame.lock().requesters.push(tx);
        }
        self.task_rx.new_scheduler_task.notify(usize::MAX);
        return rx;
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

    // pub(crate) fn mark_boundary_needs_layout(&self, object: AweakAnyRenderObject) {
    //     self.boundaries_needing_relayout
    //         .lock()
    //         .insert(PtrEq(object));
    // }

    pub(crate) fn push_layer_render_objects_needing_paint(
        &self,
        layer_render_object: AweakAnyLayerRenderObject,
    ) {
        self.layer_needing_repaint
            .lock()
            .insert(PtrEq(layer_render_object));
    }

    // pub fn schedule_idle_callback
}

pub(super) struct SchedulerTaskReceiver {
    // requested_new_frame: AtomicBool,
    // occupy_node_requests: MpscQueue<()>,
    // event: event_listener::Event,
    request_shutdown: AtomicBool,
    request_frame: SyncMutex<RequestFrame>,
    other_scheduler_tasks: MpscQueue<SchedulerTask>,
    new_scheduler_task: event_listener::Event,
}

struct RequestFrame {
    next_frame_id: u64,
    requesters: Vec<SyncMpscSender<FrameResults>>,
}

pub struct FrameResults {
    pub composited: Asc<dyn Any + Send + Sync>,
    pub id: u64,
}

impl SchedulerTaskReceiver {
    fn new() -> Self {
        Self {
            request_shutdown: AtomicBool::new(false),
            request_frame: SyncMutex::new(RequestFrame {
                next_frame_id: 0,
                requesters: Vec::new(),
            }),
            other_scheduler_tasks: Default::default(),
            new_scheduler_task: event_listener::Event::new(),
        }
    }
    pub(super) fn try_recv(&self) -> Option<SchedulerTask> {
        if self.request_shutdown.load(Acquire) {
            return Some(SchedulerTask::Shutdown);
        }
        {
            let mut request_frame = self.request_frame.lock();
            if !request_frame.requesters.is_empty() {
                let frame_id = request_frame.next_frame_id;
                request_frame.next_frame_id += 1;
                return Some(SchedulerTask::NewFrame {
                    frame_id,
                    requesters: std::mem::take(&mut request_frame.requesters),
                });
            }
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
