use std::{
    any::Any,
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering::*},
    time::Instant,
};

use event_listener::Listener;
use hashbrown::HashSet;

use crate::{
    foundation::{
        bounded_channel_sync, Asc, MpscQueue, PtrEq, SyncMpscReceiver, SyncMpscSender, SyncMutex,
        SyncRwLock,
    },
    sync::CommitBarrier,
    tree::{
        ArcSuspendWaker, AweakAnyElementNode, AweakAnyLayerRenderObject, AweakElementContextNode,
        WorkContext, WorkHandle,
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

    pub request_redraw: AtomicBool,

    pub(super) task_rx: SchedulerTaskReceiver,

    pub(super) global_sync_job_build_lock: SyncRwLock<()>,
    pub(super) job_id_counter: AtomicJobIdCounter,

    // mode: LatencyMode,
    pub(super) accumulated_jobs: SyncMutex<Vec<JobBuilder>>,
    pub(super) accumulated_wakeups: SyncMutex<Vec<ArcSuspendWaker>>,
    // pub(super) boundaries_needing_relayout: SyncMutex<HashSet<PtrEq<AweakAnyRenderObject>>>,
    pub(super) layer_needing_repaint: SyncMutex<HashSet<PtrEq<AweakAnyLayerRenderObject>>>,
}

impl SchedulerHandle {
    pub fn new(sync_threadpool: rayon::ThreadPool, async_threadpool: rayon::ThreadPool) -> Self {
        Self {
            sync_threadpool,
            async_threadpool,
            request_redraw: AtomicBool::new(false),
            task_rx: SchedulerTaskReceiver::new(),
            global_sync_job_build_lock: SyncRwLock::new(()),
            job_id_counter: AtomicJobIdCounter::new(),
            // is_executing_sync: (),
            accumulated_jobs: Default::default(),
            accumulated_wakeups: Default::default(),
            // boundaries_needing_relayout: Default::default(),
            layer_needing_repaint: Default::default(),
        }
    }

    pub fn broadcast(&self, op: impl Fn() + Sync) {
        self.sync_threadpool.broadcast(|_| op());
        self.async_threadpool.broadcast(|_| op());
    }

    pub fn create_sync_job(&self, builder: impl FnOnce(&mut JobBuilder)) {
        // Note the additional lock compared to the async version.
        // This lock is to ensure the scheduler could not process jobs before all sync jobs create in the previous frame have finished building.
        // Therefore, the scheduler will never see an outdated sync job from previous frames.
        // However, it also means that blocking in the job builder will block the entire event loop.
        let guard = self.global_sync_job_build_lock.read();
        let job_id = self.job_id_counter.generate_sync_job_id();
        let mut job_builder = JobBuilder::new(job_id, Instant::now());
        builder(&mut job_builder);
        if !job_builder.is_empty() {
            get_current_scheduler()
                .accumulated_jobs
                .lock()
                .push(job_builder);
            self.request_redraw.store(true, Release);
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
            self.request_redraw.store(true, Release);
        }
    }

    pub(crate) fn push_suspend_wake(&self, waker: ArcSuspendWaker) {
        let mut accumulated_wakeups = self.accumulated_wakeups.lock();
        if waker.lane_pos().is_sync() {
            accumulated_wakeups.push(waker);
            self.request_redraw.store(true, Release);
        } else {
            accumulated_wakeups.push(waker.clone());
            self.task_rx
                .other_tasks
                .push(SchedulerTask::AsyncSuspendReady { waker })
        }
    }

    pub fn request_new_frame(&self) -> SyncMpscReceiver<FrameResults> {
        let (tx, rx) = bounded_channel_sync(1);
        {
            self.task_rx.request_frame.lock().requesters.push(tx);
        }
        self.task_rx.new_task_event.notify(usize::MAX);
        return rx;
    }

    pub(crate) fn schedule_reorder_async_work(&self, node: AweakAnyElementNode) {
        self.task_rx
            .other_tasks
            .push(SchedulerTask::ReorderAsyncWork { node });
        self.task_rx.new_task_event.notify(usize::MAX);
    }

    pub(crate) fn schedule_reorder_provider_reservation(&self, context: AweakElementContextNode) {
        self.task_rx
            .other_tasks
            .push(SchedulerTask::ReorderProviderReservation { context });
        self.task_rx.new_task_event.notify(usize::MAX);
    }

    pub(crate) fn schedule_async_continue_work(
        &self,
        node: AweakAnyElementNode,
        work_context: Asc<WorkContext>,
        work_handle: WorkHandle,
        commit_barrier: CommitBarrier,
    ) {
        self.task_rx
            .other_tasks
            .push(SchedulerTask::AsyncContinueWork {
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

    pub fn push_extension_event(&self, event: Box<dyn Any + Send + Sync>) {
        self.task_rx
            .other_tasks
            .push(SchedulerTask::SchedulerExtensionEvent(event))
    }

    // pub fn schedule_idle_callback
}

impl SchedulerHandle {
    /// Returns accumulated jobs and point rebuilds
    pub(super) fn process_new_frame(&self) -> (Vec<JobBuilder>, Vec<ArcSuspendWaker>) {
        let _guard = self.global_sync_job_build_lock.write();
        self.job_id_counter.increment_frame();
        let accumulated_jobs = std::mem::take(&mut *self.accumulated_jobs.lock());
        let mut point_rebuilds = Vec::new();
        let mut accumulated_wakeups = self.accumulated_wakeups.lock();
        // Workaround for extract_if/drain_filter
        *accumulated_wakeups = std::mem::take(&mut *accumulated_wakeups)
            .into_iter()
            .filter_map(|waker| {
                if waker.is_aborted() {
                    return None;
                }
                if waker.lane_pos().is_sync() {
                    point_rebuilds.push(waker);
                    return None;
                }
                Some(waker)
            })
            .collect();
        (accumulated_jobs, point_rebuilds)
    }
}

pub(super) struct SchedulerTaskReceiver {
    // requested_new_frame: AtomicBool,
    // occupy_node_requests: MpscQueue<()>,
    // event: event_listener::Event,
    request_shutdown: AtomicBool,
    request_frame: SyncMutex<RequestFrame>,
    other_tasks: MpscQueue<SchedulerTask>,
    new_task_event: event_listener::Event,
}

struct RequestFrame {
    next_frame_id: u64,
    requesters: Vec<SyncMpscSender<FrameResults>>,
}

pub struct FrameResults {
    pub composited: Asc<dyn Any + Send + Sync>,
    pub id: u64,
    pub metrics: FrameMetrics,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct FrameMetrics {
    pub build_time: u64,
    pub sync_build_time: u64,
    pub layout_time: u64,
    pub paint_time: u64,
    pub composite_time: u64,
}

impl FrameMetrics {
    pub fn frame_time(&self) -> u64 {
        self.build_time + self.layout_time + self.paint_time + self.composite_time
    }
}

#[derive(Default)]
pub(super) struct FrameMetricsBuilder {
    frame_start: Option<Instant>,
    current_build_start: Option<Instant>,
    sync_batch_start: Option<Instant>,
    sync_batch_end: Option<Instant>,
    layout_start: Option<Instant>,
    paint_start: Option<Instant>,
    composite_start: Option<Instant>,
    frame_end: Option<Instant>,
}

impl FrameMetricsBuilder {
    pub(super) fn new() -> Self {
        Default::default()
    }

    pub(super) fn frame_start(&mut self) {
        self.frame_start = Some(Instant::now())
    }
    pub(super) fn current_build_start(&mut self) {
        self.current_build_start = Some(Instant::now())
    }
    pub(super) fn sync_batch_start(&mut self) {
        self.sync_batch_start = Some(Instant::now())
    }
    pub(super) fn sync_batch_end(&mut self) {
        self.sync_batch_end = Some(Instant::now())
    }
    pub(super) fn layout_start(&mut self) {
        self.layout_start = Some(Instant::now())
    }
    pub(super) fn paint_start(&mut self) {
        self.paint_start = Some(Instant::now())
    }
    pub(super) fn composite_start(&mut self) {
        self.composite_start = Some(Instant::now())
    }
    pub(super) fn frame_end(&mut self) {
        self.frame_end = Some(Instant::now())
    }

    pub(super) fn build(self) -> FrameMetrics {
        let build_time =
            (self.layout_start.unwrap() - self.frame_start.unwrap()).as_micros() as u64;
        let sync_build_time =
            (self.sync_batch_end.unwrap() - self.sync_batch_start.unwrap()).as_micros() as u64;
        let layout_time =
            (self.paint_start.unwrap() - self.paint_start.unwrap()).as_micros() as u64;
        let paint_time =
            (self.composite_start.unwrap() - self.paint_start.unwrap()).as_micros() as u64;
        let composite_time =
            (self.frame_end.unwrap() - self.composite_start.unwrap()).as_micros() as u64;
        FrameMetrics {
            build_time,
            sync_build_time,
            layout_time,
            paint_time,
            composite_time,
        }
    }
}

impl SchedulerTaskReceiver {
    fn new() -> Self {
        Self {
            request_shutdown: AtomicBool::new(false),
            request_frame: SyncMutex::new(RequestFrame {
                next_frame_id: 0,
                requesters: Vec::new(),
            }),
            other_tasks: Default::default(),
            new_task_event: event_listener::Event::new(),
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
        if let Some(e) = self.other_tasks.pop() {
            return Some(e);
        }
        return None;
    }
    pub(super) fn recv(&self) -> SchedulerTask {
        loop {
            if let Some(e) = self.try_recv() {
                return e;
            }
            let listener = self.new_task_event.listen();
            if let Some(e) = self.try_recv() {
                return e;
            }
            listener.wait();
        }
    }
}
