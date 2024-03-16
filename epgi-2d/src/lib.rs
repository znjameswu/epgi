mod r#box;
mod paint;
mod shape;
mod text;
mod vello_integrations;

pub use paint::*;
pub use r#box::*;
pub use shape::*;
pub use text::*;
pub use vello_integrations::*;

mod affine;
pub use affine::*;

use epgi_core::nodes::Provider;

pub type BoxProvider<T> = Provider<T, BoxProtocol>;
