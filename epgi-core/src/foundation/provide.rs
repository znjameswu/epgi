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

pub trait AscProvideExt {
    fn downcast<T: Provide>(self) -> Result<Asc<T>, Asc<dyn Provide>>;
}

impl AscProvideExt for Asc<dyn Provide> {
    fn downcast<T: Provide>(self) -> Result<Asc<T>, Asc<dyn Provide>> {
        if TypeId::of::<T>() == self.as_ref().type_id() {
            let ptr = Asc::into_raw(self);
            unsafe { Ok(Asc::from_raw(ptr.cast())) }
        } else {
            Err(self)
        }
    }
}
