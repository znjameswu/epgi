use std::sync::atomic::{AtomicBool, Ordering::*};

use hashbrown::HashMap;

use crate::{
    foundation::{Arc, Asc, Aweak, InlinableUsizeVec, SyncMutex, TypeKey},
    scheduler::JobId,
    sync::ElementMark,
    tree::Update,
};

use super::{AweakAnyElementNode, Element, ProviderObject};

pub type ArcElementContextNode = Arc<ElementContextNode>;
pub type AweakElementContextNode = Aweak<ElementContextNode>;

pub type ProviderElementMap = HashMap<TypeKey, ArcElementContextNode>;

pub struct ElementContextNode {
    pub(crate) element_node: AweakAnyElementNode,
    pub(crate) unmounted: AtomicBool,
    pub(crate) depth: usize,
    // The context tree points upward, so a strong pointer
    pub(crate) parent: Option<ArcElementContextNode>,

    pub(crate) mark: ElementMark,
    pub(crate) mailbox: SyncMutex<HashMap<JobId, Vec<Update>>>,
    // Use Arc due to most of the node have the same provider map
    pub provider_map: Asc<ProviderElementMap>,
    // // Pre-calculated provider map for children nodes
    // // Loop references.............
    // // Abandon this optimization. Provider widget usually has only one child anyway
    // pub provider_map_for_child: Asc<ProviderElementMap>,
    pub(crate) provider: Option<Box<ProviderObject>>,
    // pub(crate) has_render: bool,
}

impl ElementContextNode {
    // #[inline(always)]
    // pub(crate) fn new_with_provide<T: Provide>(
    //     node: AweakAnyElementNode,
    //     parent_context: ArcElementContextNode,
    //     render_context: Option<AscRenderContextNode>,
    //     provider_value: Arc<T>,
    // ) -> Self {
    //     Self::new(node, Some(parent_context), render_context, None)
    // }

    // #[inline(always)]
    // pub(crate) fn new_no_provide(
    //     node: AweakAnyElementNode,
    //     parent_context: ArcElementContextNode,
    //     render_context: Option<AscRenderContextNode>,
    // ) -> Self {
    //     Self::new(node, Some(parent_context), render_context, None)
    // }

    pub(crate) fn new_root(node: AweakAnyElementNode) -> Self {
        Self::new(node, None, None)
    }

    #[inline(always)]
    fn new(
        node: AweakAnyElementNode,
        parent_context: Option<ArcElementContextNode>,
        provider: Option<Box<ProviderObject>>,
    ) -> Self {
        if let Some(parent_context) = parent_context {
            Self {
                element_node: node,
                unmounted: false.into(),
                depth: parent_context.depth + 1,
                mark: ElementMark::new(),
                mailbox: Default::default(),
                provider_map: parent_context.get_provider_map_for_child(),
                provider,
                parent: Some(parent_context),
            }
        } else {
            Self {
                element_node: node,
                unmounted: false.into(),
                depth: 0,
                mark: ElementMark::new(),
                mailbox: Default::default(),
                provider_map: Default::default(),
                provider,
                parent: None,
            }
        }
    }

    pub(crate) fn new_for<E: Element>(
        node: AweakAnyElementNode,
        parent_context: ArcElementContextNode,
        widget: &E::ArcWidget,
    ) -> Self {
        let provider = if let Some(get_provided_value) = E::GET_PROVIDED_VALUE {
            let provided = get_provided_value(&widget);
            Some(Box::new(ProviderObject::new(provided)))
        } else {
            None
        };
        Self::new(node, Some(parent_context), provider)
    }

    #[inline(always)]
    fn get_provider_map_for_child(self: &Arc<ElementContextNode>) -> Asc<ProviderElementMap> {
        if let Some(provider) = self.provider.as_ref() {
            let mut provider_map = self.provider_map.as_ref().clone();
            provider_map.insert(provider.type_key, self.clone());
            Asc::new(provider_map)
        } else {
            self.provider_map.clone()
        }
    }

    pub(crate) fn push_update(this: &Arc<Self>, job_id: JobId, update: Update) {
        let jobs = {
            let mut mailbox = this.mailbox.lock();
            mailbox.entry(job_id).or_insert(Vec::new()).push(update);
            mailbox
                .keys()
                .filter_map(|&x| (x != job_id).then_some(x))
                .collect::<InlinableUsizeVec<_>>()
        };
        // t
    }

    pub fn is_unmounted(&self) -> bool {
        self.unmounted.load(Relaxed)
    }
}
