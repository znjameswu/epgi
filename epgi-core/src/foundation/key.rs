use std::{
    any::Any,
    fmt::Debug,
    hash::{Hash, Hasher},
};

pub trait Key: Any + Debug + Send + Sync {
    fn eq(&self, other: &dyn Key) -> bool;
    fn hash(&self, state: &mut dyn std::hash::Hasher);
    fn as_any(&self) -> &dyn Any;
}

impl Hash for dyn Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash(state)
    }
}

impl PartialEq for dyn Key {
    fn eq(&self, other: &Self) -> bool {
        self.eq(other)
    }
}

impl Eq for dyn Key {}

impl<T> Key for T
where
    T: Any + Debug + Hash + Eq + Send + Sync,
{
    fn eq(&self, other: &dyn Key) -> bool {
        match other.as_any().downcast_ref::<T>() {
            Some(other) => self.eq(other),
            None => false,
        }
    }

    fn hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValueKey<T: Clone + Debug + PartialEq + Eq + Hash> {
    pub value: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlobalKey {
    id: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UniqueKey {
    id: usize,
}
