use hashbrown::{HashMap, HashSet};

use crate::{
    foundation::{
        Arc, BoolExpectExt, MapEntryExtenision, MapOccupiedEntryExtension, Provide, PtrEq,
        PtrEqExt, SyncMutex, SyncRwLock, TypeKey,
    },
    scheduler::{get_current_scheduler, LanePos},
    sync::CommitBarrier,
};

use super::{AweakAnyElementNode, AweakElementContextNode, ElementContextNode};

pub(crate) struct ProviderObject {
    value: SyncRwLock<Arc<dyn Provide>>, // TODO ArcSwapAny<ThinArcProvide>
    inner: SyncMutex<ProviderObjectInner>, // TODO Test against RwLock
    pub(crate) type_id: TypeKey,
}

struct ProviderObjectInner {
    consumers: HashSet<PtrEq<AweakElementContextNode>>,
    occupation: AsyncProviderOccupation,
}

enum AsyncProviderOccupation {
    Reading {
        new_readers: HashMap<LanePos, HashSet<PtrEq<AweakAnyElementNode>>>,
        backqueue_writer: Option<(LanePos, Arc<dyn Provide>, CommitBarrier)>,
    },
    Writing {
        writer: LanePos,
        value_to_write: Arc<dyn Provide>,
        backqueue_new_readers:
            HashMap<LanePos, (HashSet<PtrEq<AweakAnyElementNode>>, CommitBarrier)>,
    },
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
        subscriber: AweakAnyElementNode,
        lane_pos: LanePos,
        barrier: &CommitBarrier,
    ) -> Arc<dyn Provide> {
        let provider = self
            .provider
            .as_ref()
            .expect("The provider to be reserved should exist on the context node");
        let mut inner = provider.inner.lock();
        use AsyncProviderOccupation::*;
        match &mut inner.occupation {
            Reading {
                new_readers: new_consumers,
                ..
            } => new_consumers
                .entry(lane_pos)
                .or_insert(Default::default())
                .insert(PtrEq(subscriber)),
            Writing {
                backqueue_new_readers: backqueue_new_consumers,
                ..
            } => backqueue_new_consumers
                .entry(lane_pos)
                .or_insert_with(|| {
                    get_current_scheduler()
                        .schedule_reorder_provider_reservation(Arc::downgrade(self));
                    (Default::default(), barrier.clone())
                })
                .0
                .insert(PtrEq(subscriber)),
        };
        return provider.read();
    }

    pub(crate) fn unreserve_read(
        self: &Arc<Self>,
        subscriber: &AweakAnyElementNode,
        lane_pos: LanePos,
    ) {
        let provider = self
            .provider
            .as_ref()
            .expect("The provider to be unreserved should exist on the context node");
        let mut inner = provider.inner.lock();
        use AsyncProviderOccupation::*;
        match &mut inner.occupation {
            Reading {
                new_readers: new_consumers,
                backqueue_writer,
            } => {
                let removed_lane = new_consumers
                    .entry(lane_pos)
                    .occupied()
                    .expect("The lane of the reservation to be removed should exist")
                    .and_modify(|set| {
                        set.remove(subscriber.as_ref_ptr_eq())
                            .assert("The reservation to be removed must exist")
                    })
                    .remove_if(|set| set.is_empty())
                    .is_none();

                if removed_lane {
                    // Yield to writer if there are no reader left
                    if new_consumers.is_empty() {
                        if let Some((writer, value_to_write, _)) = backqueue_writer.take() {
                            inner.occupation = Writing {
                                writer,
                                value_to_write,
                                backqueue_new_readers: Default::default(),
                            }
                        }
                    } else {
                        get_current_scheduler()
                            .schedule_reorder_provider_reservation(Arc::downgrade(self));
                    }
                }
            }
            Writing {
                backqueue_new_readers: backqueue_new_consumers,
                ..
            } => {
                backqueue_new_consumers
                    .entry(lane_pos)
                    .occupied()
                    .expect("The lane of the reservation to be removed should exist")
                    .and_modify(|(set, _)| {
                        set.remove(subscriber.as_ref_ptr_eq())
                            .assert("The reservation to be removed must exist")
                    })
                    .remove_if(|(set, _)| set.is_empty());
            }
        };
    }

    // Returns mainline readers
    pub(crate) fn reserve_write_async(
        self: &Arc<Self>,
        lane_pos: LanePos,
        value_to_write: Arc<dyn Provide>,
        barrier: &CommitBarrier,
    ) -> Vec<AweakElementContextNode> {
        let provider = self
            .provider
            .as_ref()
            .expect("The provider to be reserved should exist on the context node");
        let mut inner = provider.inner.lock();
        let mainline_readers = inner
            .consumers
            .iter()
            .map(|ptr_eq| ptr_eq.0.clone())
            .collect();
        use AsyncProviderOccupation::*;
        let Reading{ new_readers, backqueue_writer } = &mut inner.occupation else {
            panic!("There should be no async writer when reserving a async writer")
        };
        assert!(
            backqueue_writer.is_none(),
            "There should be no async writer when reserving a async writer"
        );
        if new_readers.is_empty() {
            inner.occupation = Writing {
                writer: lane_pos,
                value_to_write,
                backqueue_new_readers: Default::default(),
            };
        } else {
            *backqueue_writer = Some((lane_pos, value_to_write, barrier.clone()));
            get_current_scheduler().schedule_reorder_provider_reservation(Arc::downgrade(self));
        }
        return mainline_readers;
    }

    pub(crate) fn unreserve_write_async(&self, lane_pos: LanePos) {
        let provider = self
            .provider
            .as_ref()
            .expect("The provider to be reserved should exist on the context node");
        let mut inner = provider.inner.lock();
        // let mainline_readers = inner
        //     .consumers
        //     .iter()
        //     .map(|ptr_eq| ptr_eq.0.clone())
        //     .collect();
        use AsyncProviderOccupation::*;
        let Writing { writer, value_to_write, backqueue_new_readers }= &mut inner.occupation else {
            panic!("The async writer to be unreserved must exist")
        };
        assert_eq!(
            *writer, lane_pos,
            "The async writer to be unreserved must exist"
        );
        inner.occupation = Reading {
            new_readers: std::mem::take(backqueue_new_readers)
                .into_iter()
                .map(|(lane_pos, (set, _))| (lane_pos, set))
                .collect(),
            backqueue_writer: None,
        };
        // return mainline_readers;
    }
}

impl ProviderObject {
    pub(crate) fn read(&self) -> Arc<dyn Provide> {
        self.value.read().clone()
    }

    pub(crate) fn register_read(&self, subscriber: AweakElementContextNode) -> Option<LanePos> {
        let mut inner = self.inner.lock();
        inner.consumers.insert(PtrEq(subscriber));
        use AsyncProviderOccupation::*;
        if let Reading {
            backqueue_writer: Some((lane_pos, ..)),
            ..
        } = &inner.occupation
        {
            return Some(*lane_pos);
        }
        return None;
    }

    pub(crate) fn unregister_read(&self, subscriber: &AweakElementContextNode) -> bool {
        self.inner
            .lock()
            .consumers
            .remove(subscriber.as_ref_ptr_eq());
        todo!()
    }

    pub(crate) fn write_sync(&self, value: Arc<dyn Provide>) -> ContendingProviderReaders {
        let inner = self.inner.lock();
        // TODO: type check
        *self.value.write() = value;
        use AsyncProviderOccupation::*;
        let Reading{ new_readers, backqueue_writer } = &inner.occupation else {
            panic!("There should be no async writer when reserving a async writer")
        };
        return ContendingProviderReaders {
            mainline: inner
                .consumers
                .iter()
                .map(|ptr_eq| ptr_eq.0.clone())
                .collect(),
            non_mainline: new_readers
                .iter()
                .flat_map(|(&lane_pos, set)| {
                    set.iter().map(move |ptr_eq| (lane_pos, ptr_eq.0.clone()))
                })
                .collect(),
        };
    }
}

pub(crate) struct ContendingProviderReaders {
    pub(crate) mainline: Vec<AweakElementContextNode>,
    pub(crate) non_mainline: Vec<(LanePos, AweakAnyElementNode)>,
}
