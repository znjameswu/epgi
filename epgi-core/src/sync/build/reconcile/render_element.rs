use crate::{
    foundation::{Arc, AsIterator, HktContainer, Parallel},
    scheduler::get_current_scheduler,
    sync::{SubtreeRenderObjectChange, SubtreeRenderObjectChangeSummary},
    tree::{
        layer_render_function_table_of, ArcChildElementNode, ArcChildRenderObject,
        ArcElementContextNode, ChildRenderObjectsUpdateCallback, ContainerOf, ElementNode,
        LayerOrUnit, LayerRenderFunctionTable, MainlineState, Render, RenderAction, RenderElement,
        RenderObject, RenderObjectInner, RenderObjectSlots,
    },
};

impl<E> ElementNode<E>
where
    E: RenderElement,
    E::Render: Render<ChildContainer = E::ChildContainer>,
{
    #[inline(always)]
    pub(crate) fn visit_commit(
        &self,
        render_object: Option<Arc<RenderObject<E::Render>>>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        self_rebuild_suspended: bool,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        debug_assert!(
            render_object.is_none() || !self_rebuild_suspended,
            "Logic error in parameters: \
            This node cannot be in RebuildSuspended state if it has an attached render object"
        );
        let render_object_change_summary =
            SubtreeRenderObjectChange::summarize(render_object_changes.as_iter());
        if let Some(render_object) = render_object {
            self.visit_commit_attached(
                render_object,
                render_object_changes,
                render_object_change_summary,
            )
        } else {
            self.visit_commit_detached(
                render_object_changes,
                render_object_change_summary,
                self_rebuild_suspended,
            )
        }
    }

    #[inline(always)]
    pub(crate) fn visit_commit_attached(
        &self,
        render_object: Arc<RenderObject<E::Render>>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        render_object_change_summary: SubtreeRenderObjectChangeSummary,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        use SubtreeRenderObjectChangeSummary::*;
        match render_object_change_summary {
            KeepAll {
                child_render_action,
                subtree_has_action,
            } => {
                let render_action =
                    render_object.mark_render_action(child_render_action, subtree_has_action);
                return SubtreeRenderObjectChange::Keep {
                    // Absorb on boundaries.
                    child_render_action: render_action,
                    subtree_has_action,
                };
            }
            HasNewNoSuspend => {
                let render_action = render_object
                    .mark_render_action(RenderAction::Relayout, RenderAction::Relayout);
                {
                    let mut inner = render_object.inner.lock();
                    inner.update_children(
                        None,
                        render_object_changes,
                        render_object_change_summary,
                    );
                }
                return SubtreeRenderObjectChange::Keep {
                    child_render_action: render_action,
                    subtree_has_action: RenderAction::Relayout,
                };
            }
            HasSuspended => {
                if !<E::Render as Render>::NOOP_DETACH {
                    let mut inner = render_object.inner.lock();
                    inner.render.detach();
                }
                let mut snapshot = self.snapshot.lock();
                let state = snapshot
                    .inner
                    .mainline_mut()
                    .expect("An unmounted element node should not be reachable by a rebuild!")
                    .state
                    .as_mut()
                    .expect(
                        "State corrupted. \
                        This node has been previously designated to visit by a sync batch. \
                        However, when the visit returns, \
                        it found the sync state has been occupied.",
                    );
                use MainlineState::*;
                match state {
                    Ready { render_object, .. } => *render_object = None,
                    RebuildSuspended { .. } => panic!(
                        "State corrupted. \
                        This node has been previously visited and found to have attached render object. \
                        However, when the visit returns, \
                        it found the render object has been detached."
                    ),
                    InflateSuspended { .. } => panic!(
                        "State corrupted. \
                        This node has been previously designated to visit by a sync batch. \
                        However, when the visit returns, \
                        it found the node to be in an suspended inflated state."
                    ),
                }
                return SubtreeRenderObjectChange::Suspend;
            }
        }
    }

    pub(crate) fn visit_commit_detached(
        &self,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        render_object_change_summary: SubtreeRenderObjectChangeSummary,
        self_rebuild_suspended: bool,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        if let SubtreeRenderObjectChangeSummary::KeepAll { .. }
        | SubtreeRenderObjectChangeSummary::HasSuspended = render_object_change_summary
        {
            return SubtreeRenderObjectChange::Suspend;
        };

        if self_rebuild_suspended {
            return SubtreeRenderObjectChange::Suspend;
        }

        let mut snapshot = self.snapshot.lock();
        let snapshot_reborrow = &mut *snapshot;
        let state = &mut snapshot_reborrow
            .inner
            .mainline_mut()
            .expect("An unmounted element node should not be reachable by a rebuild!")
            .state;

        // We perform a "take-modify-insert" operation on the state to avoid using `replace_with` on child render objects
        //
        // The key flaw of any hypothetical `replace_with` implementations would be that
        // the `E::create_render` call could panic inside, and that is an external method supplied by the library users.
        // Moreover, `E::create_render` is hard to make `UnwindSafe` and therefore can't be handled by `catch_unwind`.
        // Moreover, it would also be impossible to migrate `E::create_render` out of the `replace_with` critical region without a significant cost.
        //
        // All `replace_with` occurrence within this crate MUST avoid panic from external implementations.
        // The only exception would be external panics from `HktContainer` implementations,
        // since its implementation is considered advanced while it actually takes considerable stupidity to mis-implement that contract.

        let old_state = state.take().expect(
            "State corrupted. \
                        This node has been previously designated to visit by a sync batch. \
                        However, when the visit returns, \
                        it found the sync state has been occupied.",
        );

        use MainlineState::*;
        let mut new_attached_render_object = None;
        let new_state = match old_state {
            Ready {
                element,
                render_object: None,
                hooks,
                children,
            } => {
                let render_object = Self::try_create_render_object(
                    &element,
                    &snapshot_reborrow.widget,
                    &self.context,
                    &children,
                    render_object_changes,
                );
                new_attached_render_object = render_object.clone();
                MainlineState::Ready {
                    element,
                    hooks,
                    children,
                    render_object,
                }
            }
            old_state @ RebuildSuspended { .. } => {
                debug_assert!(
                    false,
                    "State corrupted. \
                    This node has been previously found to have not been in RebuildSuspended state. \
                    However, when the visit returns, \
                    it found the node to have entered RebuildSuspended state."
                );
                old_state
            }
            Ready {
                render_object: Some(_),
                ..
            } => panic!(
                "State corrupted. \
                This node has been previously found to have been suspended by this visit. \
                However, when the visit returns, \
                it found the node to have resumed."
            ),
            InflateSuspended { .. } => panic!(
                "State corrupted. \
                This node has been previously designated to visit by a sync batch. \
                However, when the visit returns, \
                it found the node to be in an suspended inflated state."
            ),
        };
        *state = Some(new_state);
        drop(snapshot);

        if let Some(new_attached_render_object) = new_attached_render_object {
            if let LayerRenderFunctionTable::LayerNode {
                as_aweak_any_layer_node,
                ..
            } = layer_render_function_table_of::<E::Render>()
            {
                get_current_scheduler().push_layer_render_objects_needing_paint(
                    as_aweak_any_layer_node(&new_attached_render_object),
                )
            }
            return SubtreeRenderObjectChange::New(new_attached_render_object);
        } else {
            return SubtreeRenderObjectChange::Suspend;
        }
    }
}

impl<E> ElementNode<E>
where
    E: RenderElement,
    E::Render: Render<ChildContainer = E::ChildContainer>,
{
    pub(crate) fn rebuild_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
        children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        render_object: Option<Arc<RenderObject<E::Render>>>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (
        Option<Arc<RenderObject<E::Render>>>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        if let Some(render_object) = render_object {
            Self::rebuild_success_process_attached(
                widget,
                shuffle,
                render_object,
                render_object_changes,
                is_new_widget,
            )
        } else {
            Self::rebuild_success_process_detached(
                element,
                widget,
                element_context,
                children,
                render_object_changes,
            )
        }
    }
    #[inline(always)]
    pub(crate) fn rebuild_success_process_attached(
        widget: &E::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
        render_object: Arc<RenderObject<E::Render>>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        is_new_widget: bool,
    ) -> (
        Option<Arc<RenderObject<E::Render>>>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let render_object_change_summary =
            SubtreeRenderObjectChange::summarize(render_object_changes.as_iter());

        use SubtreeRenderObjectChangeSummary::*;

        if render_object_change_summary.is_suspended() {
            if !<E::Render as Render>::NOOP_DETACH {
                let mut inner = render_object.inner.lock();
                inner.render.detach();
            }
            return (None, SubtreeRenderObjectChange::Suspend);
        }

        let mut self_render_action = RenderAction::None;

        if shuffle.is_some()
            || !render_object_change_summary.is_keep_all()
            || (is_new_widget && !E::NOOP_UPDATE_RENDER_OBJECT)
        {
            let mut inner = render_object.inner.lock();
            if is_new_widget && !E::NOOP_UPDATE_RENDER_OBJECT {
                self_render_action = E::update_render(&mut inner.render, widget);
            }
            inner.update_children(shuffle, render_object_changes, render_object_change_summary);
        }

        let (child_render_action, subtree_has_action) = if let KeepAll {
            child_render_action,
            subtree_has_action,
        } = render_object_change_summary
        {
            (child_render_action, subtree_has_action)
        } else {
            (RenderAction::Relayout, RenderAction::Relayout)
        };

        let child_render_action =
            render_object.mark_render_action(child_render_action, subtree_has_action);

        let change = SubtreeRenderObjectChange::Keep {
            child_render_action: std::cmp::max(self_render_action, child_render_action),
            subtree_has_action: std::cmp::max(self_render_action, subtree_has_action),
        };

        return (Some(render_object), change);
    }

    pub(crate) fn rebuild_success_process_detached(
        element: &E,
        widget: &E::ArcWidget,
        element_context: &ArcElementContextNode,
        children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
    ) -> (
        Option<Arc<RenderObject<E::Render>>>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let render_object_change_summary =
            SubtreeRenderObjectChange::summarize(render_object_changes.as_iter());

        if let SubtreeRenderObjectChangeSummary::KeepAll { .. }
        | SubtreeRenderObjectChangeSummary::HasSuspended = render_object_change_summary
        {
            return (None, SubtreeRenderObjectChange::Suspend);
        };

        let render_object = Self::try_create_render_object(
            element,
            widget,
            element_context,
            children,
            render_object_changes,
        );

        if let Some(render_object) = render_object {
            if let LayerRenderFunctionTable::LayerNode {
                as_aweak_any_layer_node,
                ..
            } = layer_render_function_table_of::<E::Render>()
            {
                get_current_scheduler().push_layer_render_objects_needing_paint(
                    as_aweak_any_layer_node(&render_object),
                )
            }
            let change = SubtreeRenderObjectChange::New(render_object.clone());
            (Some(render_object), change)
        } else {
            return (None, SubtreeRenderObjectChange::Suspend);
        }
    }

    pub(crate) fn rebuild_suspend_commit(
        render_object: Option<Arc<RenderObject<E::Render>>>,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        if let Some(render_object) = render_object {
            if !<E::Render as Render>::NOOP_DETACH {
                let mut inner = render_object.inner.lock();
                inner.render.detach();
            }
        }
        SubtreeRenderObjectChange::Suspend
    }
}

impl<E> ElementNode<E>
where
    E: RenderElement,
    E::Render: Render<ChildContainer = E::ChildContainer>,
{
    pub(crate) fn inflate_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
    ) -> (
        Option<Arc<RenderObject<E::Render>>>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let render_object_change_summary =
            SubtreeRenderObjectChange::summarize(render_object_changes.as_iter());

        debug_assert!(
            !render_object_changes.any(SubtreeRenderObjectChange::is_keep_render_object),
            "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
        );

        if render_object_change_summary.is_suspended() {
            return (None, SubtreeRenderObjectChange::Suspend);
        }

        use SubtreeRenderObjectChange::*;
        let child_render_objects = render_object_changes.map_collect(|change| match change {
            New(child) => child,
            Suspend | Keep { .. } => {
                panic!("Fatal logic bug in epgi-core reconcile logic. Please file issue report.")
            }
        });

        let new_render_object = Arc::new(RenderObject::new(
            E::create_render(&element, &widget), //TODO: This could panic
            child_render_objects,
            element_context.clone(),
        ));

        if let LayerRenderFunctionTable::LayerNode {
            as_aweak_any_layer_node,
            ..
        } = layer_render_function_table_of::<E::Render>()
        {
            get_current_scheduler().push_layer_render_objects_needing_paint(
                as_aweak_any_layer_node(&new_render_object),
            )
        }

        let change = SubtreeRenderObjectChange::New(new_render_object.clone());

        (Some(new_render_object), change)
    }
}

impl<E> ElementNode<E>
where
    E: RenderElement,
    E::Render: Render<ChildContainer = E::ChildContainer>,
{
    #[inline(never)]
    pub(crate) fn try_create_render_object(
        element: &E,
        widget: &E::ArcWidget,
        element_context: &ArcElementContextNode,
        children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
    ) -> Option<Arc<RenderObject<E::Render>>> {
        let mut suspended = false;
        let option_child_render_objects =
            children.zip_ref_collect(render_object_changes, |child, change| {
                if suspended {
                    return None;
                }
                use SubtreeRenderObjectChange::*;
                match change {
                    Keep { .. } => {
                        let child_render_object = child.get_current_subtree_render_object();
                        if child_render_object.is_none() {
                            suspended = true;
                        }
                        child_render_object
                    }
                    New(child_render_object) => Some(child_render_object),
                    Suspend => panic!("Serious logic bug"),
                }
            });

        if suspended {
            None
        } else {
            let new_render_children =
                option_child_render_objects.map_collect(|child| child.expect("Impossible to fail"));
            let new_render_object = Arc::new(RenderObject::new(
                E::create_render(&element, &widget), //TODO: This could panic
                new_render_children,
                element_context.clone(),
            ));
            Some(new_render_object)
        }
    }
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    #[inline(always)]
    pub(crate) fn update_children(
        &mut self,
        shuffle: Option<
            Box<
                dyn FnOnce(
                    <R::ChildContainer as HktContainer>::Container<
                        ArcChildRenderObject<R::ChildProtocol>,
                    >,
                ) -> <R::ChildContainer as HktContainer>::Container<
                    RenderObjectSlots<R::ChildProtocol>,
                >,
            >,
        >,
        render_object_changes: <R::ChildContainer as HktContainer>::Container<
            SubtreeRenderObjectChange<R::ChildProtocol>,
        >,
        render_object_change_summary: SubtreeRenderObjectChangeSummary,
    ) {
        if let Some(shuffle) = shuffle {
            replace_with::replace_with_or_abort(&mut self.children, move |children| {
                let slots = (shuffle)(children);
                slots.zip_collect(render_object_changes, |slot, change| {
                    use RenderObjectSlots::*;
                    use SubtreeRenderObjectChange::*;
                    match (slot, change) {
                        (Reuse(render_object), Keep { .. }) => render_object,
                        (_, New(render_object)) => render_object,
                        (_, Suspend) => panic!(
                            "Fatal logic bug in epgi-core reconcile logic. \
                            Please file issue report."
                        ),
                        (Inflate, Keep { .. }) => panic!(
                            "Render object update callback bug: \
                            Slot requested for a new render object \
                            but the child is not producing one"
                        ),
                    }
                })
            })
        } else if !render_object_change_summary.is_keep_all() {
            replace_with::replace_with_or_abort(&mut self.children, move |children| {
                children.zip_collect(render_object_changes, |child, change| {
                    use SubtreeRenderObjectChange::*;
                    match change {
                        Keep { .. } => child,
                        New(render_object) => render_object,
                        Suspend => panic!(
                            "Fatal logic bug in epgi-core reconcile logic. \
                                Please file issue report."
                        ),
                    }
                })
            })
        }
    }

    // //https://users.rust-lang.org/t/compiler-hint-for-unlikely-likely-for-if-branches/62102/4
    // #[inline(always)]
    // fn detach_and_cache_children(
    //     &mut self,
    //     shuffle: Option<
    //         Box<
    //             dyn FnOnce(
    //                 <R::ChildContainer as HktContainer>::Container<
    //                     ArcChildRenderObject<R::ChildProtocol>,
    //                 >,
    //             ) -> <R::ChildContainer as HktContainer>::Container<
    //                 RenderObjectSlots<R::ChildProtocol>,
    //             >,
    //         >,
    //     >,
    //     render_object_changes: <R::ChildContainer as HktContainer>::Container<
    //         SubtreeRenderObjectChange<R::ChildProtocol>,
    //     >,
    // ) -> <R::ChildContainer as HktContainer>::Container<
    //     MaybeSuspendChildRenderObject<R::ChildProtocol>,
    // > {
    //     self.render.detach();

    //     let maybe_suspend_child_render_object = if let Some(shuffle) = shuffle {
    //         let slots = (shuffle)(self.children.map_ref_collect(Clone::clone));
    //         slots.zip_collect(render_object_changes, |slot, change| {
    //             use MaybeSuspendChildRenderObject::*;
    //             use RenderObjectSlots::*;
    //             use SubtreeRenderObjectChange::*;
    //             match (slot, change) {
    //                 (Reuse(render_object), Keep { .. }) => Ready(render_object),
    //                 (_, New(render_object)) => Ready(render_object),
    //                 (_, SuspendNew(render_object)) => ElementSuspended(render_object),
    //                 (Reuse(render_object), SuspendKeep) => ElementSuspended(render_object),
    //                 (_, Detach) => Detached,
    //                 (Inflate, Keep { .. } | SuspendKeep) => panic!(
    //                     "Render object update callback bug: \
    //                     Slot requested for a new render object \
    //                     but the child is not producing one"
    //                 ),
    //             }
    //         })
    //     } else {
    //         self.children
    //             .zip_ref_collect(render_object_changes, |child, change| {
    //                 use MaybeSuspendChildRenderObject::*;
    //                 use SubtreeRenderObjectChange::*;
    //                 match change {
    //                     Keep { .. } => Ready(child.clone()),
    //                     New(render_object) => Ready(render_object),
    //                     SuspendKeep => ElementSuspended(child.clone()),
    //                     SuspendNew(render_object) => ElementSuspended(render_object),
    //                     Detach => Detached,
    //                 }
    //             })
    //     };
    //     maybe_suspend_child_render_object
    // }
}
