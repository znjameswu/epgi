use std::any::TypeId;

use crate::{
    foundation::{Arc, Aweak, Canvas, Protocol, TransformHitPosition},
    tree::{Render, RenderObject},
};

use super::{ChildHitTestEntry, InLayerHitTestEntry, LinkedHitTestEntry};

// Reason to make this a concrete type and the target a trait object, rather than make this a trait and make the concrete type generic over R:Render.
// Becuase in this way walking down the tree requires half indirection within the same canvas!
// Concrete tree node with polymorphic payload is better than polymorphic tree node.
pub struct HitTestNode<C: Canvas> {
    pub target: Box<dyn TransformedHitTestTargetWithCanvas<C>>,
    pub children: Vec<HitTestNodeChild<C>>,
}

impl<C> HitTestNode<C>
where
    C: Canvas,
{
    /// Why not cast the interface for the result?
    ///
    /// Because the implementation depends on trait object virtual functions. And trait object method cannot be generic.
    /// And casting interface in our method requires a return-position generics.
    /// Therefore, we simply cannot cast the interface within our implementation.
    /// The interface cast should be performed by the caller later on. (Please see the following warning on doing that)
    ///
    /// Warning: this method will return ALL entries in the path to the first interface-compatible entry encountered in a post-order tree walk,
    /// which means, the result will contain entries that DO NOT have the interface in question.
    /// They are there simply because they stand in the path.
    /// If you wish to cast the interface later on, you need to filter out those entries.
    ///
    // TODO: is this behavior really necessary? This behavior is introduced on the premise that PointerEventHandler and GestureHandler to be two separate capabilities. 
    // Therefore, we then would probably (doubtly) need to retain GestureHandler entries after we find a PointerEventHandler entry.
    // But now they are simply unified into one interface.
    pub fn find_interface<T: ?Sized + 'static>(
        self,
        // The prepend_transform serves as an optimization
        // We can, of course, collect all child hit test entry and then prepend them with transforms in the return phase. But it would be O(N^2) complexity on node depth.
        // If we squash as many transform as we can (limited by trait object interface design), and pass them down in the recursion phase, it would be O(NM) complexity on M=canvas depth, which is usually just a few.
        // Hence the prepend_transform
        prepend_transform: Option<&C::Transform>,
    ) -> Vec<Box<dyn ChildHitTestEntry<C>>> {
        self.find_interface_id(TypeId::of::<T>(), prepend_transform)
    }
    pub fn find_interface_id(
        self,
        type_id: TypeId,
        // The prepend_transform serves as an optimization
        // We can, of course, collect all child hit test entry and then prepend them with transforms in the return phase. But it would be O(N^2) complexity on node depth.
        // If we squash as many transform as we can (limited by trait object interface design), and pass them down in the recursion phase, it would be O(NM) complexity on M=canvas depth, which is usually just a few.
        // Hence the prepend_transform
        prepend_transform: Option<&C::Transform>,
    ) -> Vec<Box<dyn ChildHitTestEntry<C>>> {
        let self_has_interface = self.target.has_interface(type_id);
        let mut result = Vec::new();
        for child in self.children {
            use HitTestNodeChild::*;
            result = match child {
                InLayer(node, transform) => match (prepend_transform, transform.as_ref()) {
                    (transform, None) | (None, transform @ Some(_)) => {
                        node.find_interface_id(type_id, transform)
                    }
                    (Some(prepend_transform), Some(transform)) => node.find_interface_id(
                        type_id,
                        Some(&C::mul_transform_ref(transform, prepend_transform)),
                    ),
                },
                NewLayer(child) => child.find_interface_id_box(type_id, prepend_transform),
            };
            if !result.is_empty() {
                // We find corresponding interface in this specific child's subtree. Cut other tree walks
                break;
            }
        }

        if self_has_interface || !result.is_empty() {
            result.push(self.target.into_entry_box(prepend_transform.cloned()));
        }
        result
    }
}

pub struct TransformedHitTestTarget<R: Render> {
    pub render_object: Aweak<RenderObject<R>>,
    pub transform: <R::ParentProtocol as Protocol>::Offset,
}

pub trait TransformedHitTestTargetWithCanvas<C: Canvas> {
    fn into_entry_box(
        self: Box<Self>,
        transform: Option<C::Transform>,
    ) -> Box<dyn ChildHitTestEntry<C>>;
    fn has_interface(&self, type_id: TypeId) -> bool;
}

impl<R> TransformedHitTestTargetWithCanvas<<R::ParentProtocol as Protocol>::Canvas>
    for TransformedHitTestTarget<R>
where
    R: Render,
{
    fn into_entry_box(
        self: Box<Self>,
        transform: Option<<<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform>,
    ) -> Box<dyn ChildHitTestEntry<<R::ParentProtocol as Protocol>::Canvas>> {
        Box::new(InLayerHitTestEntry::new(*self, transform))
    }

    fn has_interface(&self, type_id: TypeId) -> bool {
        R::all_hit_test_interfaces()
            .iter()
            .any(|(interface_id, cast)| type_id.eq(interface_id))
    }
}

pub enum HitTestNodeChild<C: Canvas> {
    InLayer(HitTestNode<C>, Option<C::Transform>),
    NewLayer(Box<dyn ChildHitTestNode<C>>),
}

pub struct HitTestNodeWithLayerTransform<PC: Canvas, CC: Canvas> {
    pub child: HitTestNode<CC>,
    pub transform: Arc<dyn TransformHitPosition<PC, CC>>,
}
pub trait ChildHitTestNode<C: Canvas> {
    fn find_interface_id_box(
        self: Box<Self>,
        type_id: TypeId,
        prepend_transform: Option<&C::Transform>,
    ) -> Vec<Box<dyn ChildHitTestEntry<C>>>;
}

impl<PC, CC> ChildHitTestNode<PC> for HitTestNodeWithLayerTransform<PC, CC>
where
    PC: Canvas,
    CC: Canvas,
{
    fn find_interface_id_box(
        self: Box<Self>,
        type_id: TypeId,
        prepend_transform: Option<&<PC as Canvas>::Transform>,
    ) -> Vec<Box<dyn ChildHitTestEntry<PC>>> {
        let child_result = self.child.find_interface_id(type_id, None);

        return child_result
            .into_iter()
            .map(|child| {
                Box::new(LinkedHitTestEntry::new(
                    prepend_transform.cloned(),
                    self.transform.clone(),
                    child,
                )) as _
            })
            .collect();
    }
}
