mod as_any;
mod constants;
mod container;
mod error;
mod key;
mod primitives;
mod protocol;
mod provide;
mod ptr_eq;
mod query_interface;
mod threadpool;
mod try_result;
mod type_key;
mod utils;
mod vec_push_last;

pub use as_any::*;
pub use constants::*;
pub use container::*;
pub use error::*;
pub use key::*;
pub use primitives::*;
pub use protocol::*;
pub use provide::*;
pub use ptr_eq::*;
pub use query_interface::*;
pub use threadpool::*;
pub use try_result::*;
pub use type_key::*;
pub use utils::*;
pub use vec_push_last::*;

#[derive(Debug)]
pub struct True;

#[derive(Debug)]
pub struct False;

pub trait ConstBool {
    const VALUE: bool;
}

impl ConstBool for True {
    const VALUE: bool = true;
}

impl ConstBool for False {
    const VALUE: bool = false;
}
