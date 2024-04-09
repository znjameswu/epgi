mod element;
pub use element::*;

mod render;
pub use render::*;

mod widget;
pub use widget::*;

mod single_child;
pub use single_child::*;

mod proxy;
pub use proxy::*;

/// Marker trait to signal some local types should use alternative implementations
pub trait ImplByTemplate {
    type Template;
}
