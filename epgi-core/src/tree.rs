mod build_context;
pub use build_context::*;

mod element;
pub use element::*;

mod hook;
pub use hook::*;

mod layer;
pub use layer::*;

mod reconcile_item;
pub use reconcile_item::*;

mod render;
pub use render::*;

mod widget;
pub use widget::*;

mod work;
pub use work::*;

use crate::foundation::{HktContainer, Protocol};

pub trait TreeNode: Send + Sync {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type ChildContainer: HktContainer;
}

#[allow(type_alias_bounds)]
pub type ContainerOf<E: TreeNode, T> = <E::ChildContainer as HktContainer>::Container<T>;
