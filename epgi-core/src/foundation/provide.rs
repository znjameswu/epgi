use std::any::TypeId;

use super::{AsAny, Asc};

pub trait Provide: AsAny + Send + Sync {
    fn eq_sized(&self, other: &Self) -> bool
    where
        Self: Sized;
    fn eq(&self, other: &dyn Provide) -> bool;
    fn clone_box(&self) -> Box<dyn Provide>;
}
impl<T> Provide for T
where
    T: PartialEq + AsAny + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn Provide> {
        Box::new(Clone::clone(self))
    }

    fn eq(&self, other: &dyn Provide) -> bool {
        match other.as_any().downcast_ref::<T>() {
            Some(other) => <Self as PartialEq>::eq(self, other),
            None => false,
        }
    }

    fn eq_sized(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        <Self as PartialEq>::eq(self, other)
    }
}

impl dyn Provide {
    // TODO: Replace all dyn Provide with dyn Any+Send+Sync. We don't really use virtual method on Provide anyway
    pub fn downcast_asc<T: Provide>(self: Asc<Self>) -> Option<Asc<T>> {
        if TypeId::of::<T>() == self.type_id() {
            let ptr = Asc::into_raw(self);
            unsafe { Some(Asc::from_raw(ptr.cast())) }
        } else {
            None
        }
    }
}
