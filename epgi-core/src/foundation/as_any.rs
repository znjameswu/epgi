use std::{
    any::{Any, TypeId},
    rc::Rc,
    sync::Arc,
};

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any_box(self: Box<Self>) -> Box<dyn Any>;
    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;

    fn type_name(&self) -> &'static str;
}

impl<T> AsAny for T
where
    T: Any + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait Identical {
    fn identical(&self, other: &Self) -> bool;
}

// Can support Arc<dyn Trait>, if Trait inherits from AsAny.
impl<T> Identical for Arc<T>
where
    T: AsAny + ?Sized + 'static,
{
    fn identical(&self, other: &Self) -> bool {
        // If both references are in scope, point to the same address and the same concrete type, then they must be identical.
        self.as_ref().type_id() == other.as_ref().type_id()
        // self.as_ref().as_any().type_id() == other.as_ref().as_any().type_id() // Compare inside concrete types
            && (self.as_ref() as *const _ as *const ()) == (other.as_ref() as *const _ as *const ())
    }
}
