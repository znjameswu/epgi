mod adapter;
pub use adapter::*;

mod element;
pub use element::*;

mod leaf;
pub use leaf::*;

mod multi_child;
pub use multi_child::*;

mod proxy;
pub use proxy::*;

mod render;
pub use render::*;

mod single_child;
pub use single_child::*;

mod widget;
pub use widget::*;

/// Marker trait to signal some local types should use alternative implementations
pub trait ImplByTemplate {
    type Template;
}
