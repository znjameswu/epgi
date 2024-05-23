use std::{any::Any, sync::Arc};

use hashbrown::HashSet;

use crate::{
    foundation::{Asc, Protocol, PtrEq, SyncMpscSender, SyncRwLock},
    scheduler::{LaneMask, LanePos},
    sync::{CommitBarrier, LaneScheduler, RenderObjectCommitResult},
    tree::{
        ArcAnyElementNode, ArcAnyLayerRenderObject, AsyncSuspendWaker, AweakAnyElementNode,
        AweakAnyLayerRenderObject, AweakElementContextNode, LayoutResults, Render, RenderElement,
        RenderObject, Widget, WorkContext, WorkHandle,
    },
};

use super::{BatchId, BatchResult, FrameResults, JobBatcher, SchedulerHandle};

// TODO: BuildAndLayout vs other event can be modeled as RwLock.
pub(super) enum SchedulerTask {
    NewFrame {
        frame_id: u64,
        requesters: Vec<SyncMpscSender<FrameResults>>,
    },
    SuspendReady {
        waker: std::sync::Arc<AsyncSuspendWaker>,
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
    Shutdown,
    SchedulerExtensionEvent(Box<dyn Any + Send + Sync>),
}

pub struct Scheduler<E: SchedulerExtension> {
    build_states: Asc<SyncRwLock<BuildStates>>,
    job_batcher: JobBatcher,
    extension: E,
}

pub struct BuildStates {
    scheduler: LaneScheduler,
    pub root_element: ArcAnyElementNode,
    pub root_render_object: ArcAnyLayerRenderObject,
}

impl BuildStates {
    pub(crate) fn apply_batcher_result(
        &mut self,
        result: BatchResult,
        point_rebuilds: HashSet<PtrEq<AweakElementContextNode>>,
    ) {
        self.scheduler
            .apply_batcher_result(result, point_rebuilds, &self.root_element)
    }

    pub(crate) fn dispatch_sync_batch(&mut self) -> Option<BatchId> {
        self.scheduler.dispatch_sync_batch(&self.root_element)
    }

    pub(crate) fn perform_layout(&mut self) {
        self.root_render_object.visit_and_layout();
    }

    pub(crate) fn perform_paint(
        &self,
        layer_render_objects: HashSet<PtrEq<AweakAnyLayerRenderObject>>,
    ) {
        rayon::scope(|scope| {
            for PtrEq(layer_render_object) in layer_render_objects {
                let Some(layer_render_objects) = layer_render_object.upgrade() else {
                    continue;
                };
                scope.spawn(move |_| layer_render_objects.repaint_if_attached());
            }
        })
    }

    pub(crate) fn perform_composite(&self) -> Asc<dyn Any + Send + Sync> {
        self.root_render_object.recomposite_into_memo()
    }

    pub(crate) fn dispatch_async_batches(&mut self) {
        self.scheduler
            .dispatch_async_batches(self.root_element.clone())
    }
}

pub trait SchedulerExtension: Send {
    fn on_frame_begin(&mut self, build_states: &BuildStates);

    fn on_layout_complete(&mut self, build_states: &BuildStates);

    fn on_frame_complete(build_states: &BuildStates);

    fn on_extension_event(&mut self, event: Box<dyn Any + Send + Sync>);
}

impl<E> Scheduler<E>
where
    E: SchedulerExtension,
{
    pub fn new<W: Widget<Element = EL>, EL: RenderElement<Render = R>, R: Render>(
        root_widget: Arc<W>,
        initial_layout: LayoutResults<R::ParentProtocol, R::LayoutMemo>,
        initial_offset: <R::ParentProtocol as Protocol>::Offset,
        scheduler_handle: &SchedulerHandle,
        extension: E,
    ) -> Self {
        let lane_scheduler = LaneScheduler::new();
        use crate::sync::ChildWidgetSyncInflateExt;
        let (root_element, commit_result) = scheduler_handle
            .sync_threadpool
            .scope(|_| root_widget.inflate_sync(None, &lane_scheduler));

        let RenderObjectCommitResult::New(root_render_object) = commit_result.render_object else {
            panic!("Root widget inflate failed!");
        };
        let root_render_object = root_render_object
            .downcast_arc_any_layer_render_object()
            .expect("Root render object should have a layer");
        let _ = root_render_object
            .as_any()
            .downcast_ref::<RenderObject<R>>()
            .expect("Impossible to fail")
            .inner
            .lock()
            .cache
            .insert_layout_results(initial_layout)
            .paint_offset
            .insert(initial_offset);

        scheduler_handle
            .push_layer_render_objects_needing_paint(Arc::downgrade(&root_render_object));
        Self {
            build_states: Asc::new(SyncRwLock::new(BuildStates {
                scheduler: lane_scheduler,
                root_element: root_element.as_any_arc(),
                root_render_object,
            })),
            job_batcher: JobBatcher::new(),

            extension,
        }
    }
    pub fn start_event_loop(mut self, handle: &SchedulerHandle) {
        // handle.push_layer_render_objects_needing_paint(self.lane_scheduler.roo)
        let tasks = &handle.task_rx;
        loop {
            let task = tasks.recv();
            use SchedulerTask::*;
            match task {
                // TODO: backpressure to prevent new frame event overrun
                NewFrame {
                    frame_id,
                    requesters,
                } => {
                    let mut build_states = self.build_states.write();
                    // let commited_async_batches = lane_scheduler.commit_completed_async_batches(&mut self.job_batcher);
                    // for commited_async_batch in commited_async_batches {
                    //     self.job_batcher.remove_commited_batch(&commited_async_batch)
                    // }
                    let (new_jobs, point_rebuilds) = {
                        let _guard = handle.global_sync_job_build_lock.write();
                        handle.job_id_counter.increment_frame();
                        (
                            std::mem::take(&mut *handle.accumulated_jobs.lock()),
                            std::mem::take(&mut *handle.accumulated_point_rebuilds.lock()),
                        )
                    };
                    let updates = self.job_batcher.update_with_new_jobs(new_jobs);
                    build_states.apply_batcher_result(
                        updates,
                        point_rebuilds
                            .into_iter()
                            .filter(|waker| !waker.completed())
                            .map(|waker| PtrEq(waker.node.clone()))
                            .collect(),
                    );
                    build_states.dispatch_async_batches();
                    self.extension.on_frame_begin(&build_states);
                    let commited_sync_batch = build_states.dispatch_sync_batch();
                    if let Some(commited_sync_batch) = commited_sync_batch {
                        self.job_batcher.remove_commited_batch(&commited_sync_batch);
                    }
                    // let commited_async_batches = lane_scheduler.commit_completed_async_batches(&mut self.job_batcher);
                    // for commited_async_batch in commited_async_batches {
                    //     self.job_batcher.remove_commited_batch(&commited_async_batch)
                    // }
                    build_states.perform_layout();
                    self.extension.on_layout_complete(&build_states);
                    // We don't have RwLock downgrade in std, this is to simulate it by re-reading while blocking the event loop.
                    // TODO: Parking_lot owned downgradable guard
                    drop(build_states);
                    let read_guard = self.build_states.read();
                    let build_states = self.build_states.clone();
                    let layer_needing_repaint =
                        { std::mem::take(&mut *handle.layer_needing_repaint.lock()) };
                    let paint_started_event = event_listener::Event::new();
                    let paint_started = paint_started_event.listen();
                    handle.sync_threadpool.spawn(move || {
                        let build_states = build_states.read();
                        paint_started_event.notify(usize::MAX);
                        build_states.perform_paint(layer_needing_repaint);
                        let result = build_states.perform_composite();
                        for requester in requesters {
                            let _ = requester.try_send(FrameResults {
                                composited: result.clone(),
                                id: frame_id,
                            }); // TODO: log failure
                        }

                        E::on_frame_complete(&build_states);
                        drop(build_states)
                    });
                    paint_started.wait();
                    drop(read_guard);
                }
                SuspendReady { waker } => {}
                ReorderAsyncWork { node } => {
                    let build_states = self.build_states.clone();
                    handle.sync_threadpool.spawn(move || {
                        let build_states = build_states.read();
                        build_states.scheduler.reorder_async_work(node);
                    })
                }
                ReorderProviderReservation { context } => {
                    let build_states = self.build_states.clone();
                    handle.sync_threadpool.spawn(move || {
                        let build_states = build_states.read();
                        build_states.scheduler.reorder_provider_reservation(context);
                    })
                }
                AsyncYieldSubtree {
                    node,
                    work_context,
                    work_handle,
                    commit_barrier,
                } => self.handle_async_yield(node, work_context, work_handle, commit_barrier),
                SchedulerExtensionEvent(event) => {
                    self.extension.on_extension_event(event);
                }
                Shutdown => break,
            }
        }
    }

    fn handle_async_yield(
        &self,
        node: AweakAnyElementNode,
        work_context: Asc<WorkContext>,
        work_handle: WorkHandle,
        commit_barrier: CommitBarrier,
    ) {
        let Some(node) = node.upgrade() else {
            return;
        };
        let mut ancestor_lanes = LaneMask::new();
        let mut current = node.context();
        // No need to OR a consumer lane, since the consumer work is either spawned by an ancestor work or the current continuing work, which we will get rid of.
        let subtree_lanes = current.descendant_lanes() | current.mailbox_lanes();
        debug_assert!(
            !subtree_lanes.contains(LanePos::SYNC),
            "We should not see a sync lane while processing unsync batches"
        );
        loop {
            let Err(not_unmounted) = current.is_unmounted() else {
                // This node is unmounted, return
                return;
            };
            let Some(parent) = current.parent(not_unmounted) else {
                break;
            };
            // No need to OR the occupier lane, since an occupying work is always spawned by a root mailbox work.
            // No need to OR a consumer lanes, since consumer work is always spawned by a root mailbox work.
            ancestor_lanes = ancestor_lanes | parent.mailbox_lanes();
            current = parent;
        }
        let mut executable_lanes: Vec<LanePos> =
            (subtree_lanes - ancestor_lanes - work_context.lane_pos)
                .into_iter()
                .collect();

        let build_states = self.build_states.read();

        executable_lanes.sort_unstable_by_key(|&lane_pos| {
            build_states
                .scheduler
                .get_batch_conf_for_async(lane_pos)
                .expect("Batch should exist")
                .priority
        });

        node.visit_and_continue_work_async(
            &(work_context, work_handle, commit_barrier),
            executable_lanes.as_slice(),
            &build_states.scheduler,
        )
    }
}
