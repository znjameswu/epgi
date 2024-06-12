use std::{
    any::Any,
    fmt::Debug,
    hash::{Hash, Hasher},
};

pub trait Key: Any + Debug + Send + Sync {
    fn eq_key(&self, other: &dyn Key) -> bool;
    fn hash(&self, state: &mut dyn std::hash::Hasher);
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn Key>;
}

impl Hash for dyn Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash(state)
    }
}

impl PartialEq for dyn Key {
    fn eq(&self, other: &Self) -> bool {
        self.eq_key(other)
    }
}

impl Eq for dyn Key {}

impl<T> Key for T
where
    T: Clone + Any + Debug + Hash + Eq + Send + Sync,
{
    fn eq_key(&self, other: &dyn Key) -> bool {
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

    fn clone_box(&self) -> Box<dyn Key> {
        Box::new(self.clone())
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

/// An alternate version of [Key].
///
/// It does not need cross-type `Eq`, does not need `Hash`, does not need `Debug`. But it does require `Clone`
///
/// Because you can't really be picky on what people will send into `use_memo` and `use_effect`.
/// But you do need to clone them out before perform async rebuild.
pub trait DependencyKey: PartialEq + Clone + Send + Sync + 'static {}

impl<T> DependencyKey for T where T: PartialEq + Clone + Send + Sync + 'static {}
