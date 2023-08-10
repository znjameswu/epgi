use epgi_2d::BoxProtocol;
use epgi_core::{common::{ArcChildElementNode, ArcChildWidget, Element, Widget}, foundation::Never};

// #[derive(Debug)]
// pub struct ConstrainedBox {
//     child: ArcChildWidget<BoxProtocol>,
// }

// impl Widget for ConstrainedBox {
//     type Element = ConstrainedBoxElement;

//     fn create_element(self: epgi_core::foundation::Asc<Self>) -> Self::Element {
//         todo!()
//     }

//     fn into_arc_widget(
//         self: std::sync::Arc<Self>,
//     ) -> <Self::Element as epgi_core::common::Element>::ArcWidget {
//         todo!()
//     }
// }

// #[derive(Clone)]
// pub struct ConstrainedBoxElement {
//     child: ArcChildElementNode<BoxProtocol>,
// }

// impl Element for ConstrainedBoxElement {
//     type ArcWidget;

//     type ParentProtocol = BoxProtocol;

//     type ChildProtocol = BoxProtocol;

//     type Provided = Never;

//     fn perform_rebuild_element(
//         self,
//         widget: &Self::ArcWidget,
//         provider_values: epgi_core::foundation::InlinableDwsizeVec<
//             epgi_core::foundation::Arc<dyn epgi_core::foundation::Provide>,
//         >,
//         reconciler: impl epgi_core::common::Reconciler<Self::ChildProtocol>,
//     ) -> Result<Self, (Self, epgi_core::foundation::BuildSuspendedError)> {
//         todo!()
//     }

//     fn perform_inflate_element(
//         widget: &Self::ArcWidget,
//         provider_values: epgi_core::foundation::InlinableDwsizeVec<
//             epgi_core::foundation::Arc<dyn epgi_core::foundation::Provide>,
//         >,
//         reconciler: impl epgi_core::common::Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
//     ) -> Result<Self, epgi_core::foundation::BuildSuspendedError> {
//         todo!()
//     }

//     type ChildIter;

//     fn children(&self) -> Self::ChildIter {
//         todo!()
//     }

//     type ArcRenderObject;
// }
