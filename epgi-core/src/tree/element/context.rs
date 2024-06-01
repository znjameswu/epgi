use std::sync::atomic::{AtomicBool, Ordering::*};

use hashbrown::HashMap;

use crate::{
    foundation::{Arc, Asc, Aweak, SyncMutex, TypeKey},
    scheduler::{JobBuilder, JobId},
    tree::Update,
};

use super::{AweakAnyElementNode, Element, ElementMark, ImplProvide, ProviderObject};

pub type ArcElementContextNode = Arc<ElementContextNode>;
pub type AweakElementContextNode = Aweak<ElementContextNode>;

pub type ProviderElementMap = HashMap<TypeKey, ArcElementContextNode>;

pub struct ElementContextNode {
    pub(crate) element_node: AweakAnyElementNode,
    pub(crate) unmounted: AtomicBool,
    pub(crate) depth: usize,
    // The context tree points upward, so a strong pointer
    parent: Option<ArcElementContextNode>,

    pub(crate) mark: ElementMark,
    pub(crate) mailbox: SyncMutex<HashMap<JobId, Vec<Update>>>,
    // Use Arc due to most of the node have the same provider map
    pub provider_map: Asc<ProviderElementMap>,
    // // Pre-calculated provider map for children nodes
    // // Loop references.............
    // // Abandon this optimization. Provider widget usually has only one child anyway
    // pub provider_map_for_child: Asc<ProviderElementMap>,
    pub(crate) provider_object: Option<Box<ProviderObject>>,
    // pub(crate) has_render: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct NotUnmountedToken(());

impl ElementContextNode {
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
                provider_object: provider,
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
                provider_object: provider,
                parent: None,
            }
        }
    }

    pub(crate) fn new_for<E: Element>(
        node: AweakAnyElementNode,
        parent_context: Option<ArcElementContextNode>,
        widget: &E::ArcWidget,
    ) -> Self {
        let provider = <E as Element>::Impl::option_get_provided_key_value_pair(widget)
            .map(|(provided, type_key)| Box::new(ProviderObject::new(provided, type_key)));
        Self::new(node, parent_context, provider)
    }

    #[inline(always)]
    fn get_provider_map_for_child(self: &Arc<ElementContextNode>) -> Asc<ProviderElementMap> {
        if let Some(provider) = self.provider_object.as_ref() {
            let mut provider_map = self.provider_map.as_ref().clone();
            provider_map.insert(provider.type_key, self.clone());
            Asc::new(provider_map)
        } else {
            self.provider_map.clone()
        }
    }

    pub(crate) fn push_update(self: &Arc<Self>, update: Update, job_builder: &mut JobBuilder) {
        let mut mailbox = self.mailbox.lock();
        let hook_index = update.hook_index;
        let job_id = job_builder.id();
        mailbox.entry(job_id).or_default().push(update);
        job_builder.add_root(
            Arc::downgrade(self),
            mailbox
                .iter()
                .filter_map(move |(&existing_job_id, exisiting_updates)| {
                    (existing_job_id != job_id
                        && exisiting_updates
                            .iter()
                            .any(|existing_update| existing_update.hook_index == hook_index))
                    .then_some(existing_job_id)
                }),
        );
    }

    pub(crate) fn is_unmounted(&self) -> Result<(), NotUnmountedToken> {
        if self.unmounted.load(Relaxed) {
            Ok(())
        } else {
            Err(NotUnmountedToken(()))
        }
    }

    pub(crate) fn assert_not_unmounted(&self) -> NotUnmountedToken {
        debug_assert!(
            !self.unmounted.load(Relaxed),
            "We assumed this element to not be unmounted"
        );
        NotUnmountedToken(())
    }

    pub(crate) fn parent(
        &self,
        not_unmounted: NotUnmountedToken,
    ) -> &Option<ArcElementContextNode> {
        let _ = not_unmounted;
        &self.parent
    }
}
