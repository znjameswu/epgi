use crate::{
    foundation::{Arc, Asc, Protocol},
    sync::CommitBarrier,
    tree::{
        ArcElementContextNode, ChildElementWidgetPair, Element, ElementBase, ElementWidgetPair,
        Widget, WorkContext, WorkHandle,
    },
};

pub trait ChildElementWidgetPairAsyncBuildExt<P: Protocol> {
    fn rebuild_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );

    fn rebuild_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );
}

impl<E> ChildElementWidgetPairAsyncBuildExt<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: Element,
{
    fn rebuild_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        todo!()
        // let _ = self.element.rebuild_node_async(
        //     Work {
        //         widget: Some(self.widget),
        //         context: work_context,
        //     },
        //     parent_handle,
        //     barrier,
        // );
    }

    fn rebuild_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        self.rebuild_async(work_context, parent_handle, barrier)
    }
}

pub trait ChildWidgetAsyncInflateExt<PP: Protocol> {
    fn inflate_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_context: ArcElementContextNode,
        barrier: CommitBarrier,
        handle: WorkHandle,
    ) -> Box<dyn ChildElementWidgetPair<PP>>;
}

impl<T> ChildWidgetAsyncInflateExt<<<T as Widget>::Element as ElementBase>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_context: ArcElementContextNode,
        barrier: CommitBarrier,
        handle: WorkHandle,
    ) -> Box<dyn ChildElementWidgetPair<<<T as Widget>::Element as ElementBase>::ParentProtocol>>
    {
        todo!()
        // let node = ElementNode::<<T as Widget>::Element>::new_async_uninflated(
        //     self.clone().into_arc_widget(),
        //     work_context,
        //     parent_context,
        //     handle,
        //     barrier,
        // );
        // return Box::new(ElementWidgetPair {
        //     widget: self.into_arc_widget(),
        //     element: node,
        // });
    }
}
