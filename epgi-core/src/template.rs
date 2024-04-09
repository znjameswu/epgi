mod element;
pub use element::*;

mod render;
pub use render::*;

/// Marker trait to signal some local types should use alternative implementations
pub trait ImplByTemplate {
    type Template;
}
