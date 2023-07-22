use super::AsAny;

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
