use hashbrown::HashSet;

use crate::{
    debug::debug_assert_sync_phase,
    foundation::{
        Arc, BoolExpectExt, MapEntryExtenision, MapOccupiedEntryExtension, Provide, PtrEq,
        PtrEqExt, SmallMap, SyncMutex, SyncRwLock, TypeKey,
    },
    scheduler::{get_current_scheduler, BatchConf, BatchId, JobPriority, LaneMask, LanePos},
    sync::{CommitBarrier, LaneScheduler},
};

use super::{
    AweakAnyElementNode, AweakElementContextNode, ElementContextNode, ElementLockHeldToken,
};

pub(crate) struct ProviderObject {
    value: SyncRwLock<Arc<dyn Provide>>, // TODO ArcSwapAny<ThinArcProvide>
    inner: SyncMutex<ProviderObjectInner>, // TODO Test against RwLock
    pub(crate) type_key: TypeKey,
}

struct ProviderObjectInner {
    consumers: HashSet<PtrEq<AweakElementContextNode>>,
    reservation: AsyncProviderReservation,
}

impl ProviderObject {
    // pub(crate) fn new<T: Provide>(value: Arc<T>) -> Self {
    //     Self {
    //         value: SyncRwLock::new(value),
    //         inner: SyncMutex::new(ProviderObjectInner {
    //             consumers: Default::default(),
    //             occupation: AsyncProviderOccupation::Reading {
    //                 new_readers: Default::default(),
    //                 backqueue_writer: None,
    //             },
    //         }),
    //         type_key: TypeKey::of::<T>(),
    //     }
    // }

    pub(crate) fn new(value: Arc<dyn Provide>, type_key: TypeKey) -> Self {
        Self {
            value: SyncRwLock::new(value),
            inner: SyncMutex::new(ProviderObjectInner {
                consumers: Default::default(),
                reservation: AsyncProviderReservation::ReservedForRead {
                    readers: Default::default(),
                    backqueue_writer: None,
                },
            }),
            type_key,
        }
    }
    pub(crate) fn get_value(&self) -> Arc<dyn Provide> {
        self.value.read().clone()
    }
    pub(crate) fn contains_reservation_from_lanes(&self, lane_mask: LaneMask) -> bool {
        use AsyncProviderReservation::*;
        match &self.inner.lock().reservation {
            ReservedForRead {
                readers,
                backqueue_writer,
            } => {
                if readers.keys().any(|&lane_pos| lane_mask.contains(lane_pos)) {
                    return true;
                }
                if backqueue_writer
                    .as_ref()
                    .is_some_and(|(writer, _)| lane_mask.contains(writer.lane_pos))
                {
                    return true;
                }
                return false;
            }
            ReservedForWrite {
                writer,
                backqueue_readers,
            } => {
                if lane_mask.contains(writer.lane_pos) {
                    return true;
                }
                if backqueue_readers
                    .keys()
                    .any(|&lane_pos| lane_mask.contains(lane_pos))
                {
                    return true;
                }
                return false;
            }
        }
    }
}

enum AsyncProviderReservation {
    ReservedForRead {
        readers: SmallMap<LanePos, ReservedReadingBatch>,
        backqueue_writer: Option<(ReservedWriter, CommitBarrier)>,
    },
    ReservedForWrite {
        writer: ReservedWriter,
        backqueue_readers: SmallMap<LanePos, (ReservedReadingBatch, CommitBarrier)>,
    },
}

struct ReservedReadingBatch {
    // id: BatchId,
    priority: JobPriority,
    nodes: HashSet<PtrEq<AweakAnyElementNode>>,
}

impl ReservedReadingBatch {
    fn new_empty(batch_conf: &BatchConf) -> Self {
        Self {
            // id: batch_conf.id,
            priority: batch_conf.priority,
            nodes: Default::default(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ReservedWriter {
    lane_pos: LanePos,
    batch_id: BatchId,
    priority: JobPriority,
    value_to_write: Arc<dyn Provide>,
}

impl ElementContextNode {
    /// Try to register a non-mainline top-level reserve on this provider
    ///
    /// If this async read has higher priority than an async write that has already occupied this provider,
    /// this method will call the scheduler to resolve this conflict.
    /// However, it will still successfully perform the read nonetheless.
    ///
    /// This operation is atomic.
    pub(crate) fn reserve_read(
        self: &Arc<Self>,
        reserver: AweakAnyElementNode,
        lane_pos: LanePos,
        batch_conf: &BatchConf,
        barrier: &CommitBarrier,
        element_lock_held: &ElementLockHeldToken,
    ) -> Arc<dyn Provide> {
        let _ = element_lock_held;
        let provider = self
            .provider_object
            .as_ref()
            .expect("The provider to be reserved should exist on the context node");
        let mut inner = provider.inner.lock();
        use AsyncProviderReservation::*;
        match &mut inner.reservation {
            ReservedForRead { readers, .. } => readers
                .entry(lane_pos)
                .or_insert(ReservedReadingBatch::new_empty(batch_conf))
                .nodes
                .insert(PtrEq(reserver)),
            ReservedForWrite {
                backqueue_readers, ..
            } => backqueue_readers
                .entry(lane_pos)
                .or_insert_with(|| {
                    get_current_scheduler()
                        .schedule_reorder_provider_reservation(Arc::downgrade(self));
                    (ReservedReadingBatch::new_empty(batch_conf), barrier.clone())
                })
                .0
                .nodes
                .insert(PtrEq(reserver)),
        };
        return provider.read();
    }

    pub(crate) fn unreserve_read(
        self: &Arc<Self>,
        reserver: &AweakAnyElementNode,
        lane_pos: LanePos,
    ) {
        debug_assert_sync_phase();

        let provider = self
            .provider_object
            .as_ref()
            .expect("The provider to be unreserved should exist on the context node");
        let mut inner = provider.inner.lock();
        use AsyncProviderReservation::*;
        match &mut inner.reservation {
            ReservedForRead {
                readers,
                backqueue_writer,
            } => {
                let removed_lane = readers
                    .entry(lane_pos)
                    .occupied()
                    .expect("The lane of the reservation to be removed should exist")
                    .and_modify(|reader| {
                        reader
                            .nodes
                            .remove(reserver.as_ref_ptr_eq())
                            .assert("The reservation to be removed must exist")
                    })
                    .remove_if(|reader| reader.nodes.is_empty())
                    .is_ok();

                if removed_lane {
                    if let Some(reservation) =
                        self.on_reader_reservation_removed(readers, backqueue_writer)
                    {
                        inner.reservation = reservation;
                    }
                }
            }
            ReservedForWrite {
                backqueue_readers, ..
            } => {
                let _ = backqueue_readers
                    .entry(lane_pos)
                    .occupied()
                    .expect("The lane of the reservation to be removed should exist")
                    .and_modify(|(reader, _)| {
                        reader
                            .nodes
                            .remove(reserver.as_ref_ptr_eq())
                            .assert("The reservation to be removed must exist")
                    })
                    .remove_if(|(reader, _)| reader.nodes.is_empty());
            }
        };
    }

    fn on_reader_reservation_removed(
        self: &Arc<Self>,
        readers: &mut SmallMap<LanePos, ReservedReadingBatch>,
        backqueue_writer: &mut Option<(ReservedWriter, CommitBarrier)>,
    ) -> Option<AsyncProviderReservation> {
        if backqueue_writer.is_some() {
            // Yield to writer if there are no reader left
            if readers.is_empty() {
                let (writer, barrier) = backqueue_writer.take().expect("Impossible to fail");
                drop(barrier);
                return Some(AsyncProviderReservation::ReservedForWrite {
                    writer,
                    backqueue_readers: Default::default(),
                });
            } else {
                // Otherwise, we have to determine the priority between the rest of the readers and the writer
                get_current_scheduler().schedule_reorder_provider_reservation(Arc::downgrade(self));
            }
        }
        return None;
    }

    /// The entire lane of the reserving work will be removed from the occupier.
    /// A lane may have multiple reserving work on this provider. Therefore, it is okay if the lane has already been removed by a previous call from the same lane.
    #[must_use]
    pub(crate) fn register_reserved_read(
        self: &Arc<Self>,
        subscriber: AweakElementContextNode,
        reserver: &AweakAnyElementNode,
        lane_pos: LanePos,
    ) -> Option<LanePos> {
        debug_assert_sync_phase();

        let provider = self
            .provider_object
            .as_ref()
            .expect("The provider to be unreserved should exist on the context node");

        let mut inner = provider.inner.lock();
        inner.consumers.insert(PtrEq(subscriber));
        use AsyncProviderReservation::*;
        match &mut inner.reservation {
            ReservedForRead {
                backqueue_writer,
                readers,
            } => {
                let removed_lane = readers
                    .entry(lane_pos)
                    .occupied()
                    .expect("The lane of the reservation to be removed should exist")
                    .and_modify(|reader| {
                        reader
                            .nodes
                            .remove(reserver.as_ref_ptr_eq())
                            .assert("The reservation to be removed must exist")
                    })
                    .remove_if(|reader| reader.nodes.is_empty())
                    .is_ok();
                // This ensures that we only report contending writers once during a batch commit
                // We do not wish to restart the writer multiple times during our commit
                // But we need to make sure that we only restart the contending writer AFTER we have registered all reserved reads
                if removed_lane {
                    if let Some((writer, _barrier)) = backqueue_writer {
                        let writer_lane_pos = writer.lane_pos;
                        if let Some(reservation) =
                            self.on_reader_reservation_removed(readers, backqueue_writer)
                        {
                            inner.reservation = reservation;
                        }
                        return Some(writer_lane_pos);
                    }
                }
                return None;
            }
            ReservedForWrite { .. } => panic!(
                "The provider is reserved for write,\
                which means all its reserved read should not be able to commit"
            ),
        }
    }

    // Returns mainline readers
    pub(crate) fn reserve_write_async(
        self: &Arc<Self>,
        lane_pos: LanePos,
        value_to_write: Arc<dyn Provide>,
        batch_conf: &BatchConf,
        barrier: &CommitBarrier,
    ) -> Vec<AweakElementContextNode> {
        let provider = self
            .provider_object
            .as_ref()
            .expect("The provider to be reserved should exist on the context node");
        let mut inner = provider.inner.lock();
        let mainline_readers = inner
            .consumers
            .iter()
            .map(|ptr_eq| ptr_eq.0.clone())
            .collect();
        use AsyncProviderReservation::*;
        let ReservedForRead {
            readers,
            backqueue_writer,
        } = &mut inner.reservation
        else {
            panic!("There should be no async writer when reserving a async writer")
        };
        assert!(
            backqueue_writer.is_none(),
            "There should be no async writer when reserving a async writer"
        );
        let writer = ReservedWriter {
            lane_pos,
            batch_id: batch_conf.id,
            priority: batch_conf.priority,
            value_to_write,
        };
        if readers.is_empty() {
            inner.reservation = ReservedForWrite {
                writer,
                backqueue_readers: Default::default(),
            };
        } else {
            *backqueue_writer = Some((writer, barrier.clone()));
            get_current_scheduler().schedule_reorder_provider_reservation(Arc::downgrade(self));
        }
        return mainline_readers;
    }

    pub(crate) fn unreserve_write_async(&self, lane_pos: LanePos) {
        let provider = self
            .provider_object
            .as_ref()
            .expect("The provider to be unreserved should exist on the context node");
        let mut inner = provider.inner.lock();
        use AsyncProviderReservation::*;
        match &mut inner.reservation {
            ReservedForRead {
                backqueue_writer, ..
            } => {
                let (writer, _barrier) = backqueue_writer
                    .take()
                    .expect("The async writer to be unreserved must exist");
                assert_eq!(
                    writer.lane_pos, lane_pos,
                    "The async writer to be unreserved must exist"
                );
            }
            ReservedForWrite {
                writer,
                backqueue_readers,
            } => {
                assert_eq!(
                    writer.lane_pos, lane_pos,
                    "The async writer to be unreserved must exist"
                );
                inner.reservation = ReservedForRead {
                    readers: std::mem::take(backqueue_readers)
                        .into_iter()
                        .map(|(lane_pos, (set, _))| (lane_pos, set))
                        .collect(),
                    backqueue_writer: None,
                };
            }
        }
    }

    pub(crate) fn reorder_reservation(&self, lane_scheduler: &LaneScheduler) {
        let provider = self
            .provider_object
            .as_ref()
            .expect("The provider to be reordered should exist on the context node");
        let mut inner = provider.inner.lock();
        use AsyncProviderReservation::*;
        match &mut inner.reservation {
            ReservedForRead {
                readers,
                backqueue_writer,
            } => {
                let Some((writer, _barrier)) = backqueue_writer else {
                    return;
                };
                if readers
                    .values()
                    .all(|reader| writer.priority < reader.priority)
                {
                    let (writer, barrier) = backqueue_writer.take().expect("Impossible to fail");
                    drop(barrier); // Just to show where we dropped it
                    inner.reservation = ReservedForWrite {
                        writer,
                        backqueue_readers: std::mem::take(readers)
                            .into_iter()
                            .map(|(lane_pos, reader)| {
                                (
                                    lane_pos,
                                    (
                                        reader,
                                        lane_scheduler
                                            .get_commit_barrier_for(lane_pos)
                                            .expect("Lane should exist"),
                                    ),
                                )
                            })
                            .collect(),
                    }
                }
            }
            ReservedForWrite {
                writer,
                backqueue_readers,
            } => {
                if backqueue_readers
                    .values()
                    .any(|(reader, _barrier)| reader.priority < writer.priority)
                {
                    inner.reservation = ReservedForRead {
                        readers: std::mem::take(backqueue_readers)
                            .into_iter()
                            .map(|(lane_pos, (reader, barrier))| {
                                drop(barrier); // Just to show where we dropped it
                                (lane_pos, reader)
                            })
                            .collect(),
                        backqueue_writer: Some((
                            writer.clone(),
                            lane_scheduler
                                .get_commit_barrier_for(writer.lane_pos)
                                .expect("Lane should exist"),
                        )),
                    }
                }
            }
        }
    }
}

impl ProviderObject {
    pub(crate) fn read(&self) -> Arc<dyn Provide> {
        self.value.read().clone()
    }

    #[must_use]
    pub(crate) fn register_read(&self, subscriber: AweakElementContextNode) -> Option<LanePos> {
        debug_assert_sync_phase();

        let mut inner = self.inner.lock();
        inner.consumers.insert(PtrEq(subscriber));
        use AsyncProviderReservation::*;
        match &inner.reservation {
            ReservedForRead {
                backqueue_writer: Some((writer, ..)),
                ..
            }
            | ReservedForWrite { writer, .. } => Some(writer.lane_pos),
            _ => None,
        }
    }

    #[must_use]
    pub(crate) fn unregister_read(&self, subscriber: &AweakElementContextNode) -> Option<LanePos> {
        debug_assert_sync_phase();

        let mut inner = self.inner.lock();
        let removed = inner.consumers.remove(subscriber.as_ref_ptr_eq());
        debug_assert!(
            removed,
            "The provider to be unregistered should recognize this consumer"
        );
        use AsyncProviderReservation::*;
        match &inner.reservation {
            ReservedForRead {
                backqueue_writer: Some((writer, ..)),
                ..
            }
            | ReservedForWrite { writer, .. } => Some(writer.lane_pos),
            _ => None,
        }
    }

    pub(crate) fn write_sync(&self, value: Arc<dyn Provide>) -> ContendingProviderReaders {
        let inner = self.inner.lock();
        // TODO: type check
        *self.value.write() = value;
        use AsyncProviderReservation::*;
        let ReservedForRead {
            readers,
            backqueue_writer: None,
        } = &inner.reservation
        else {
            panic!("There should be no async writer when reserving a sync writer")
        };
        return ContendingProviderReaders {
            mainline: inner
                .consumers
                .iter()
                .map(|ptr_eq| ptr_eq.0.clone())
                .collect(),
            non_mainline: readers
                .iter()
                .flat_map(|(&lane_pos, reader)| {
                    reader
                        .nodes
                        .iter()
                        .map(move |ptr_eq| (lane_pos, ptr_eq.0.clone()))
                })
                .collect(),
        };
    }

    pub(crate) fn commit_async_write(
        &self,
        lane_pos: LanePos,
        batch_id: BatchId,
        lane_scheduler: &LaneScheduler,
    ) {
        let mut inner = self.inner.lock();
        use AsyncProviderReservation::*;
        let ReservedForWrite {
            writer,
            backqueue_readers,
        } = std::mem::replace(
            &mut inner.reservation,
            ReservedForRead {
                readers: Default::default(),
                backqueue_writer: None,
            },
        )
        else {
            panic!(
                "The provider object should be reserved for write when committing an async write,
                otherwise there would be a commit barrier preventing this commit"
            );
        };
        debug_assert_eq!(
            writer.lane_pos, lane_pos,
            "Committed async batch provider write should have the correct lane pos"
        );
        debug_assert_eq!(
            writer.batch_id, batch_id,
            "Committed async batch provider write should have the correct batch id"
        );
        {
            *self.value.write() = writer.value_to_write;
        }
        inner.reservation = ReservedForRead {
            readers: backqueue_readers
                .into_iter()
                .map(|(lane_pos, (reader, barrier))| {
                    reader.nodes.iter().for_each(|PtrEq(node)| {
                        if let Some(node) = node.upgrade() {
                            node.restart_async_work(lane_pos, lane_scheduler)
                        }
                    });
                    drop(barrier); //Symbolic
                    (lane_pos, reader)
                })
                .collect(),
            backqueue_writer: None,
        };
    }
}

pub(crate) struct ContendingProviderReaders {
    pub(crate) mainline: Vec<AweakElementContextNode>,
    pub(crate) non_mainline: Vec<(LanePos, AweakAnyElementNode)>,
}
