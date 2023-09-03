mod r#box;
mod paint;
mod text;
mod vello_integrations;

pub use paint::*;
pub use r#box::*;
pub use text::*;
pub use vello_integrations::*;

use epgi_core::nodes::Provider;

pub type BoxProvider<T> = Provider<T, BoxProtocol>;
