use linear_map::LinearMap;

use crate::{
    foundation::{Arc, Container, InlinableDwsizeVec, LinearMapEntryExt, Provide, TypeKey},
    scheduler::{get_current_scheduler, LanePos},
    sync::LaneScheduler,
    tree::{ArcElementContextNode, Element, ElementContextNode, FullElement, ImplProvide},
};

pub(super) fn read_and_update_subscriptions_sync(
    new_consumed_types: &[TypeKey],
    old_consumed_types: &[TypeKey],
    element_context: &ArcElementContextNode,
    lane_scheduler: &LaneScheduler,
) -> InlinableDwsizeVec<Arc<dyn Provide>> {
    let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);

    // Unregister
    for consumed in old_consumed_types.iter() {
        if !new_consumed_types.contains(consumed) {
            let removed = element_context
                .provider_map
                .get(consumed)
                .expect("ProviderMap should be consistent")
                .provider
                .as_ref()
                .expect("Element should provide types according to ProviderMap")
                .unregister_read(&Arc::downgrade(element_context));
            debug_assert!(removed)
        }
    }

    // Why do we need to restart contending async writers at all?
    // Because if we are registering a new read, they will be unaware of us as a secondary root.

    // We only need to cancel contending async writers only if this is a new subscription.
    // Because a contending async writer on an old subsciption will naturally find this node as a secondary root.

    // We only need to cancel the topmost contending writes from a single lane. Because all its subtree will be purged.
    let mut async_work_needs_restarting = LinearMap::<LanePos, ArcElementContextNode>::new();

    let consumed_values = new_consumed_types
        .iter()
        .map(|consumed| {
            let is_old = is_old_consumed_types || old_consumed_types.contains(consumed);
            let subscription = element_context
                .provider_map
                .get(consumed)
                .expect("Requested provider should exist");
            let provider = subscription
                .provider
                .as_ref()
                .expect("Element should provide types according to ProviderMap");
            if !is_old {
                let contending_writer = provider.register_read(Arc::downgrade(element_context));
                if let Some(contending_lane) = contending_writer {
                    async_work_needs_restarting
                        .entry(contending_lane)
                        .and_modify(|v| {
                            if v.depth < subscription.depth {
                                *v = subscription.clone()
                            }
                        })
                        .or_insert_with(|| subscription.clone());
                }
            }
            provider.read()
        })
        .collect();
    let async_work_needs_restarting: Vec<_> = async_work_needs_restarting.into();
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
            .provider
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
            reader.mark_consumer_root(LanePos::Sync, reader.assert_not_unmounted());
        }
    }
}
