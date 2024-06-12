use std::collections::VecDeque;

use hashbrown::HashMap;

use crate::{
    foundation::{Arc, InlinableDwsizeVec, Protocol, PtrEq, VecContainer},
    tree::RenderObjectSlots,
};

use super::{
    try_convert_if_same_type, ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, ArcWidget,
    ChildRenderObjectsUpdateCallback, ElementNode, ElementReconcileItem, FullElement,
};

impl<E: FullElement> ElementNode<E> {
    pub(crate) fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<E::ParentProtocol>,
    ) -> Result<
        ElementReconcileItem<E::ParentProtocol>,
        (Arc<Self>, ArcChildWidget<E::ParentProtocol>),
    > {
        let old_widget = self.widget();
        if widget.key() != old_widget.key() {
            return Err((self, widget));
        }
        match try_convert_if_same_type(&old_widget, widget) {
            Ok(widget) => {
                if PtrEq(&old_widget) == PtrEq(&widget) {
                    Ok(ElementReconcileItem::Keep(self))
                } else {
                    Ok(ElementReconcileItem::new_update::<E>(self, widget))
                }
            }
            Err(widget) => Err((self, widget)),
        }
    }
}

pub struct ElementWidgetPair<E: FullElement> {
    pub element: Arc<ElementNode<E>>,
    pub widget: E::ArcWidget,
}

impl<E> Clone for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn clone(&self) -> Self {
        Self {
            element: self.element.clone(),
            widget: self.widget.clone(),
        }
    }
}

pub trait ChildElementWidgetPair<P: Protocol>:
    crate::sync::ChildElementWidgetPairSyncBuildExt<P>
    + crate::r#async::ChildElementWidgetPairAsyncBuildExt<P>
    + Send
    + Sync
    + 'static
{
    fn element(&self) -> ArcChildElementNode<P>;
}

impl<E> ChildElementWidgetPair<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn element(&self) -> ArcChildElementNode<E::ParentProtocol> {
        self.element.clone() as _
    }
}

// Uses flutter's Element::update_children logic
pub fn default_reconcile_vec<P: Protocol>(
    old_children: Vec<ArcChildElementNode<P>>,
    new_widgets: Vec<ArcChildWidget<P>>,
    nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<P>>,
) -> (
    Vec<ElementReconcileItem<P>>,
    Option<ChildRenderObjectsUpdateCallback<VecContainer, P>>,
) {
    let old_children_count = old_children.len();
    let new_widgets_count = new_widgets.len();

    let mut old_children = VecDeque::from(old_children); // Zero-cost!
    let mut new_widgets = VecDeque::from(new_widgets);

    let mut items = VecDeque::with_capacity(new_widgets.len());

    // We can't use zip, because we have to 1. register reconcile failure and 2. work from back in a back-aligned fashion.
    loop {
        let Some(old_child) = old_children.pop_front() else {
            break;
        };
        let Some(new_widget) = new_widgets.pop_front() else {
            old_children.push_front(old_child); // Give back moved-out values
            break;
        };
        match old_child.can_rebuild_with(new_widget) {
            Ok(item) => items.push_back(item),
            Err((old_child, new_widget)) => {
                old_children.push_front(old_child);
                new_widgets.push_front(new_widget);
                break;
            }
        };
    }

    // Now the `items` layout: [..front_of_reconcile_items]
    let front_item_count = items.len();

    loop {
        let Some(old_child) = old_children.pop_back() else {
            break;
        };
        let Some(new_widget) = new_widgets.pop_back() else {
            old_children.push_back(old_child); // Give back moved-out values
            break;
        };
        match old_child.can_rebuild_with(new_widget) {
            Ok(item) => items.push_front(item),
            Err((old_child, new_widget)) => {
                old_children.push_back(old_child);
                new_widgets.push_back(new_widget);
                break;
            }
        };
    }

    // Now the `items` layout: [..back_of_reconcile_items, ..front_of_reconcile_items]
    let back_item_count = items.len() - front_item_count;

    // Now we scan for keyed children in the middile
    let mut old_keyed_children = HashMap::new();
    for (mid_index, old_child) in old_children.into_iter().enumerate() {
        let old_index = mid_index + front_item_count;
        if let Some(key) = old_child.widget_key() {
            old_keyed_children.insert(key, (old_index, old_child));
        } else {
            nodes_needing_unmount.push(old_child)
        }
    }

    let mut mid_index_mapping = HashMap::new();
    // Now we reconcile middle by key
    items.extend(
        new_widgets
            .into_iter()
            .enumerate()
            .map(|(new_mid_index, new_widget)| {
                if let Some(key) = new_widget.key() {
                    if let Some((old_index, old_child)) = old_keyed_children.remove(key) {
                        match old_child.can_rebuild_with(new_widget) {
                            Ok(item) => {
                                let new_index = new_mid_index + front_item_count;
                                let key_collision = mid_index_mapping.insert(old_index, new_index);
                                debug_assert!(
                                    key_collision.is_none(),
                                    "Impossible to trigger key collision"
                                );
                                return item;
                            }
                            Err((old_child, new_widget)) => {
                                nodes_needing_unmount.push(old_child);
                                return ElementReconcileItem::new_inflate(new_widget);
                            }
                        };
                    }
                }
                return ElementReconcileItem::new_inflate(new_widget);
            }),
    );

    // Now the `items` layout: [..back_of_reconcile_items, ..front_of_reconcile_items, ..mid_of_reconcile_items]

    items.rotate_left(back_item_count);

    // Now the `items` layout: [..front_of_reconcile_items, ..mid_of_reconcile_items, ..back_of_reconcile_items]

    // Any unclaimed middile child will be unmounted
    nodes_needing_unmount.extend(
        old_keyed_children
            .into_values()
            .map(|(_, old_child)| old_child),
    );

    let shuffle_render_object = move |old_render_objects: Vec<ArcChildRenderObject<P>>| {
        debug_assert!(old_children_count == old_render_objects.len());

        let mut result = std::iter::repeat(RenderObjectSlots::<P>::Inflate)
            .take(new_widgets_count)
            .collect::<Vec<_>>();

        for (old_index, old_render_object) in old_render_objects.into_iter().enumerate() {
            if old_index < front_item_count {
                result[old_index] = RenderObjectSlots::Reuse(old_render_object);
            } else if old_index > old_children_count - back_item_count {
                result[old_index + new_widgets_count - old_children_count] =
                    RenderObjectSlots::Reuse(old_render_object);
            } else {
                if let Some(&new_index) = mid_index_mapping.get(&old_index) {
                    result[new_index] = RenderObjectSlots::Reuse(old_render_object);
                }
            }
        }
        return result;
    };

    let has_shuffled = front_item_count + back_item_count == old_children_count
        && old_children_count == new_widgets_count;
    let shuffle_render_object = has_shuffled.then(|| Box::new(shuffle_render_object) as _);

    (items.into(), shuffle_render_object)
}
