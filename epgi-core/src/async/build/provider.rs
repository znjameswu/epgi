use hashbrown::HashMap;

use crate::{
    foundation::{Arc, Asc, InlinableDwsizeVec, InlinableUsizeVec, Provide, TypeKey},
    sync::CommitBarrier,
    tree::{
        ArcElementContextNode, ElementNode, FullElement, ProviderElementMap, SubscriptionDiff,
        WorkContext,
    },
};

impl<E: FullElement> ElementNode<E> {
    pub(super) fn calc_subscription_diff(
        new_consumed_types: &[TypeKey],
        old_consumed_types: &[TypeKey],
        reserved_provider_values: &HashMap<TypeKey, Asc<dyn Provide>>,
        provider_map: &ProviderElementMap,
    ) -> SubscriptionDiff {
        let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);
        if is_old_consumed_types {
            return Default::default();
        }
        let remove = old_consumed_types
            .iter()
            .filter(|consumed_type| !new_consumed_types.contains(consumed_type))
            .map(|consumed_type| {
                provider_map
                    .get(consumed_type)
                    .expect("Requested provider should exist")
                    .clone()
            })
            .collect();
        let mut register = InlinableUsizeVec::<ArcElementContextNode>::default();
        let mut reserve = InlinableUsizeVec::<ArcElementContextNode>::default();

        // Filter and group-by
        for consumed_type in new_consumed_types.iter() {
            let is_old = old_consumed_types.contains(consumed_type);
            if !is_old {
                let subscription = provider_map
                    .get(consumed_type)
                    .expect("Requested provider should exist")
                    .clone();
                if reserved_provider_values.contains_key(consumed_type) {
                    register.push(subscription);
                } else {
                    reserve.push(subscription);
                }
            }
        }
        return SubscriptionDiff {
            register,
            reserve,
            remove,
        };
    }

    // Warning 1: This method will acquire provider locks one by one. Make sure your hold no other lock than the single element snapshot lock in question.
    // Warning 2: You must hold the element snapshot lock before calling this method.
    //      Otherwise another contending async writing commit may trace back to this node (by the reservation you left) at anytime
    //      The contending commit may decide cancel your async work while you are still reserving. And then create a mess of racing conditions.
    //
    //      This could have been solved by requiring a lock guard as parameter.
    //      However, the two callsites do not share a common inner type as guard.
    //
    //      The correct design under a cooperative cancellation framework should reqruie a cooperative WorkHandle while reserving.
    //      However, since we already hold the element snapshot lock. We decide to do this clever optimization.
    pub(super) fn read_consumed_values_async(
        self: &Arc<Self>,
        new_consumed_types: &[TypeKey],
        old_consumed_types: &[TypeKey],
        work_context: &WorkContext,
        barrier: &CommitBarrier,
    ) -> InlinableDwsizeVec<Arc<dyn Provide>> {
        let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);

        let consumed_values = new_consumed_types
            .iter()
            .map(|consumed_type| {
                work_context
                    .reserved_provider_values
                    .get(consumed_type)
                    .cloned()
                    .unwrap_or_else(|| {
                        let subscription = self
                            .context
                            .provider_map
                            .get(consumed_type)
                            .expect("The context node of the requested provider should exist");
                        if is_old_consumed_types || old_consumed_types.contains(consumed_type) {
                            subscription
                                .provider
                                .as_ref()
                                .expect("The requested provider should exist")
                                .read()
                        } else {
                            subscription.reserve_read(
                                Arc::downgrade(self) as _,
                                work_context.lane_pos,
                                barrier,
                            )
                        }
                    })
            })
            .collect();
        return consumed_values;
    }
}
