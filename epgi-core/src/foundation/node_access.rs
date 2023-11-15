use crate::tree::{
    ArcChildRenderObject, ContainerOf, Element, ElementNode, ElementSnapshot, RenderContextNode,
    RenderOrUnit,
};

pub(crate) trait NodeAccessor<Node: AccessNode> {
    type Probe;

    type Return;

    fn can_bypass(self, node: &Node) -> Result<Self::Return, Self::Probe>;

    fn access(inner: &mut Node::Inner<'_>, probe: Self::Probe) -> Self::Return;
}

pub(crate) trait AccessNode {
    type Inner<'a>;

    fn with_inner<R>(&self, op: impl FnOnce(Self::Inner<'_>) -> R) -> R;
}

pub(crate) fn access_node<N: AccessNode, A: NodeAccessor<N>>(node: &N, accessor: A) -> A::Return {
    let can_bypass = accessor.can_bypass(&node);

    can_bypass
        .unwrap_or_else(|probe| node.with_inner(move |mut inner| A::access(&mut inner, probe)))
}

pub(crate) fn access_node2<N: AccessNode, A1: NodeAccessor<N>, A2: NodeAccessor<N>>(
    node: &N,
    accessor1: A1,
    accessor2: A2,
) -> (A1::Return, A2::Return) {
    let can_bypass1 = accessor1.can_bypass(&node);
    let can_bypass2 = accessor2.can_bypass(&node);
    match (can_bypass1, can_bypass2) {
        (Ok(res1), Ok(res2)) => (res1, res2),
        (can_bypass1, can_bypass2) => node.with_inner(move |mut inner| {
            let res1 = can_bypass1.unwrap_or_else(|probe1| A1::access(&mut inner, probe1));
            let res2 = can_bypass2.unwrap_or_else(|probe2| A2::access(&mut inner, probe2));
            (res1, res2)
        }),
    }
}

pub(crate) fn access_node3<
    N: AccessNode,
    A1: NodeAccessor<N>,
    A2: NodeAccessor<N>,
    A3: NodeAccessor<N>,
>(
    node: &N,
    accessor1: A1,
    accessor2: A2,
    accessor3: A3,
) -> (A1::Return, A2::Return, A3::Return) {
    let can_bypass1 = accessor1.can_bypass(&node);
    let can_bypass2 = accessor2.can_bypass(&node);
    let can_bypass3 = accessor3.can_bypass(&node);
    match (can_bypass1, can_bypass2, can_bypass3) {
        (Ok(res1), Ok(res2), Ok(res3)) => (res1, res2, res3),
        (can_bypass1, can_bypass2, can_bypass3) => node.with_inner(move |mut inner| {
            let res1 = can_bypass1.unwrap_or_else(|probe1| A1::access(&mut inner, probe1));
            let res2 = can_bypass2.unwrap_or_else(|probe2| A2::access(&mut inner, probe2));
            let res3 = can_bypass3.unwrap_or_else(|probe3| A3::access(&mut inner, probe3));
            (res1, res2, res3)
        }),
    }
}

pub(crate) struct AccessArcRenderObject<E: Element>(
    pub(crate) <E::RenderOrUnit as RenderOrUnit<E>>::ArcRenderObject,
);

impl<E> AccessNode for AccessArcRenderObject<E>
where
    E: Element,
{
    type Inner<'a> = (
        &'a mut E::RenderOrUnit,
        &'a mut ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
        &'a RenderContextNode,
    );

    fn with_inner<R>(&self, op: impl FnOnce(Self::Inner<'_>) -> R) -> R {
        <E::RenderOrUnit as RenderOrUnit<E>>::with_inner(
            &self.0,
            |render, children, render_context| op((render, children, render_context)),
        )
    }
}

impl<E> AccessNode for ElementNode<E>
where
    E: Element,
{
    type Inner<'a> = (&'a mut ElementSnapshot<E>);

    fn with_inner<R>(&self, op: impl FnOnce(Self::Inner<'_>) -> R) -> R {
        let mut snapshot = self.snapshot.lock();
        op(&mut snapshot)
    }
}
