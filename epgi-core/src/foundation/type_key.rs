use std::{
    any::{type_name, TypeId},
    hash::Hash,
};

#[derive(Clone, Copy, Debug, Eq)]
pub struct TypeKey {
    id: TypeId,
    name: &'static str,
}

impl TypeKey {
    pub fn of<T: ?Sized + 'static>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl Hash for TypeKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for TypeKey {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
