use linear_map::LinearMap;

use crate::{
    foundation::{Arc, Container, InlinableDwsizeVec, Provide, TypeKey},
    scheduler::{get_current_scheduler, LanePos},
    sync::LaneScheduler,
    tree::{ArcElementContextNode, Element, ElementContextNode, FullElement, ImplProvide},
};

pub(super) struct AsyncWorkNeedsRestarting(LinearMap<LanePos, ArcElementContextNode>);

impl AsyncWorkNeedsRestarting {
    pub(super) fn new() -> Self {
        Self(Default::default())
    }
    pub(super) fn push_ref(&mut self, lane_pos: LanePos, node: &ArcElementContextNode) {
        match self.0.entry(lane_pos) {
            linear_map::Entry::Occupied(mut entry) => {
                if entry.get().depth < node.depth {
                    entry.insert(node.clone());
                }
            }
            linear_map::Entry::Vacant(entry) => {
                entry.insert(node.clone());
            }
        }
    }

    pub(super) fn push(&mut self, lane_pos: LanePos, node: ArcElementContextNode) {
        match self.0.entry(lane_pos) {
            linear_map::Entry::Occupied(mut entry) => {
                if entry.get().depth < node.depth {
                    entry.insert(node);
                }
            }
            linear_map::Entry::Vacant(entry) => {
                entry.insert(node);
            }
        }
    }

    pub(super) fn execute_restarts(self, lane_scheduler: &LaneScheduler) {
        let async_work_needs_restarting: Vec<_> = self.0.into();
        async_work_needs_restarting.par_for_each(
            &get_current_scheduler().sync_threadpool,
            |(lane_pos, context)| {
                let node = context
                    .element_node
                    .upgrade()
                    .expect("ElementNode should be alive");
                node.restart_async_work(lane_pos, lane_scheduler)
            },
        );
    }
}

pub(super) fn read_and_update_subscriptions_sync(
    new_consumed_types: &[TypeKey],
    old_consumed_types: &[TypeKey],
    element_context: &ArcElementContextNode,
    lane_scheduler: &LaneScheduler,
) -> InlinableDwsizeVec<Arc<dyn Provide>> {
    let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);

    // Why do we need to restart contending async writers at all?
    // Because if we are registering a new read, they will be unaware of us as a secondary root.

    // We only need to cancel contending async writers only if this is a new subscription.
    // Because a contending async writer on an old subsciption will naturally find this node as a secondary root.

    // We only need to cancel the topmost contending writes from a single lane. Because all its subtree will be purged.
    let mut async_work_needs_restarting = AsyncWorkNeedsRestarting::new();

    // Unregister
    for consumed in old_consumed_types.iter() {
        if !new_consumed_types.contains(consumed) {
            let provider_node = element_context
                .provider_map
                .get(consumed)
                .expect("ProviderMap should be consistent");
            let contending_writer = provider_node
                .provider_object
                .as_ref()
                .expect("Element should provide types according to ProviderMap")
                .unregister_read(&Arc::downgrade(element_context));
            if let Some(contending_lane) = contending_writer {
                async_work_needs_restarting.push_ref(contending_lane, provider_node)
            }
        }
    }

    let consumed_values = new_consumed_types
        .iter()
        .map(|consumed| {
            let is_old = is_old_consumed_types || old_consumed_types.contains(consumed);
            let provider_node = element_context
                .provider_map
                .get(consumed)
                .expect("Requested provider should exist");
            let provider_object = provider_node
                .provider_object
                .as_ref()
                .expect("Element should provide types according to ProviderMap");
            if !is_old {
                let contending_writer =
                    provider_object.register_read(Arc::downgrade(element_context));
                if let Some(contending_lane) = contending_writer {
                    async_work_needs_restarting.push_ref(contending_lane, provider_node)
                }
            }
            provider_object.read()
        })
        .collect();

    async_work_needs_restarting.execute_restarts(lane_scheduler);
    return consumed_values;
}

pub(super) fn update_provided_value<E: FullElement>(
    old_widget: &E::ArcWidget,
    new_widget: &E::ArcWidget,
    element_context: &ElementContextNode,
    lane_scheduler: &LaneScheduler,
) {
    if let Some((new_provided_value, _, true)) =
        <E as Element>::Impl::diff_provided_value(old_widget, new_widget)
    {
        let contending_readers = element_context
            .provider_object
            .as_ref()
            .expect("Element with a provided value should have a provider")
            .write_sync(new_provided_value);

        contending_readers.non_mainline.par_for_each(
            &get_current_scheduler().sync_threadpool,
            |(lane_pos, node)| {
                let node = node.upgrade().expect("ElementNode should be alive");
                node.restart_async_work(lane_pos, lane_scheduler)
            },
        );

        // This is the a operation, we do not fear any inconsistencies caused by cancellation.
        for reader in contending_readers.mainline {
            let reader: ArcElementContextNode = reader.upgrade().expect("Readers should be alive");
            reader.mark_consumer_root(LanePos::SYNC, reader.assert_not_unmounted());
        }
    }
}
