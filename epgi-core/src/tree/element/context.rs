use std::sync::atomic::{AtomicBool, Ordering::*};

use hashbrown::HashMap;

use crate::{
    foundation::{Arc, Asc, Aweak, InlinableUsizeVec, Provide, SyncMutex, TypeKey},
    scheduler::JobId,
    sync::ElementMark,
    tree::{AscRenderContextNode, RenderContextNode, Update},
};

use super::{AweakAnyElementNode, ProviderObject};

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
    /// Pre-calculated provider map for children nodes
    pub provider_map_for_child: Asc<ProviderElementMap>,
    pub(crate) provider: Option<Box<ProviderObject>>,

    pub(crate) nearest_render_context: AscRenderContextNode,
    pub(crate) has_render: bool,
}

impl ElementContextNode {
    #[inline(always)]
    pub(crate) fn new_with_provide<T: Provide>(
        node: AweakAnyElementNode,
        parent_context: ArcElementContextNode,
        provider_value: Arc<T>,
        has_render: bool,
    ) -> Self {
        todo!()
    }

    #[inline(always)]
    pub(crate) fn new_no_provide(
        node: AweakAnyElementNode,
        parent_context: ArcElementContextNode,
        has_render: bool,
    ) -> Self {
        let nearest_render_context = if has_render {
            Asc::new(RenderContextNode::new(
                parent_context.nearest_render_context.clone(),
            ))
        } else {
            parent_context.nearest_render_context.clone()
        };
        Self {
            element_node: node,
            unmounted: false.into(),
            depth: parent_context.depth + 1,
            mark: ElementMark::new(),
            mailbox: Default::default(),
            provider_map: parent_context.provider_map_for_child.clone(),
            provider_map_for_child: parent_context.provider_map_for_child.clone(),
            provider: None,
            nearest_render_context,
            has_render,
            parent: Some(parent_context),
        }
    }

    pub(crate) fn new_root(node: AweakAnyElementNode) -> Self {
        let nearest_render_context = Asc::new(RenderContextNode::new_root());
        Self {
            element_node: node,
            unmounted: false.into(),
            depth: 0,
            mark: ElementMark::new(),
            mailbox: Default::default(),
            provider_map: Default::default(),
            provider_map_for_child: Default::default(),
            provider: None,
            nearest_render_context,
            has_render: true,
            parent: None,
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
