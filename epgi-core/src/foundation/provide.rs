use std::{any::TypeId, fmt::Debug};

use super::{AsAny, Asc};

// Cannot directly alias [PartialEq] since it would make us not object-safe
pub trait Provide: Debug + AsAny + Send + Sync {
    fn eq_sized(&self, other: &Self) -> bool
    where
        Self: Sized;
    fn eq_dyn(&self, other: &dyn Provide) -> bool;
}

impl<T> Provide for T
where
    T: PartialEq + Debug + AsAny + Clone + Send + Sync,
{
    fn eq_dyn(&self, other: &dyn Provide) -> bool {
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
