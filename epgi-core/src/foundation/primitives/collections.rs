pub type InlinableUsizeVec<T, const N: usize = 2> = smallvec::SmallVec<[T; N]>;
pub type Inlinable64Vec<T> = smallvec::SmallVec<[T; (std::mem::size_of::<usize>() * 2) / 8]>;
pub type InlinableDwsizeVec<T, const N: usize = 1> = smallvec::SmallVec<[T; N]>;

pub type SmallMap<K, V> = linear_map::LinearMap<K, V>;
pub type SmallSet<T> = linear_map::set::LinearSet<T>;

pub trait LinearMapEntryExt<'a, K, V> {
    fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V);
}

impl<'a, K, V> LinearMapEntryExt<'a, K, V> for linear_map::Entry<'a, K, V> {
    fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            linear_map::Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                linear_map::Entry::Occupied(entry)
            }
            linear_map::Entry::Vacant(entry) => linear_map::Entry::Vacant(entry),
        }
    }
}

pub trait MapEntryExtenision {
    type OccupiedEntry;
    type VacantEntry;

    fn occupied(self) -> Option<Self::OccupiedEntry>;
    fn vacant(self) -> Option<Self::VacantEntry>;
}

impl<'a, K, V> MapEntryExtenision for std::collections::hash_map::Entry<'a, K, V> {
    type OccupiedEntry = std::collections::hash_map::OccupiedEntry<'a, K, V>;
    type VacantEntry = std::collections::hash_map::VacantEntry<'a, K, V>;
    #[inline(always)]
    fn occupied(self) -> Option<Self::OccupiedEntry> {
        match self {
            std::collections::hash_map::Entry::Occupied(e) => Some(e),
            std::collections::hash_map::Entry::Vacant(_) => None,
        }
    }
    #[inline(always)]
    fn vacant(self) -> Option<Self::VacantEntry> {
        match self {
            std::collections::hash_map::Entry::Occupied(_) => None,
            std::collections::hash_map::Entry::Vacant(e) => Some(e),
        }
    }
}

impl<'a, K, V, S> MapEntryExtenision for hashbrown::hash_map::Entry<'a, K, V, S> {
    type OccupiedEntry = hashbrown::hash_map::OccupiedEntry<'a, K, V, S>;
    type VacantEntry = hashbrown::hash_map::VacantEntry<'a, K, V, S>;
    #[inline(always)]
    fn occupied(self) -> Option<Self::OccupiedEntry> {
        match self {
            hashbrown::hash_map::Entry::Occupied(e) => Some(e),
            hashbrown::hash_map::Entry::Vacant(_) => None,
        }
    }
    #[inline(always)]
    fn vacant(self) -> Option<Self::VacantEntry> {
        match self {
            hashbrown::hash_map::Entry::Occupied(_) => None,
            hashbrown::hash_map::Entry::Vacant(e) => Some(e),
        }
    }
}

impl<'a, K, V> MapEntryExtenision for linear_map::Entry<'a, K, V> {
    type OccupiedEntry = linear_map::OccupiedEntry<'a, K, V>;
    type VacantEntry = linear_map::VacantEntry<'a, K, V>;
    #[inline(always)]
    fn occupied(self) -> Option<Self::OccupiedEntry> {
        match self {
            linear_map::Entry::Occupied(e) => Some(e),
            linear_map::Entry::Vacant(_) => None,
        }
    }
    #[inline(always)]
    fn vacant(self) -> Option<linear_map::VacantEntry<'a, K, V>> {
        match self {
            linear_map::Entry::Occupied(_) => None,
            linear_map::Entry::Vacant(e) => Some(e),
        }
    }
}

pub trait MapOccupiedEntryExtension: Sized {
    type Value;
    fn and_modify(self, f: impl FnOnce(&mut Self::Value)) -> Self;
    fn remove_if(self, f: impl FnOnce(&Self::Value) -> bool) -> Option<Self>;
}

impl<'a, K, V> MapOccupiedEntryExtension for std::collections::hash_map::OccupiedEntry<'a, K, V> {
    type Value = V;
    #[inline(always)]
    fn and_modify(mut self, f: impl FnOnce(&mut Self::Value)) -> Self {
        f(self.get_mut());
        self
    }
    #[inline(always)]
    fn remove_if(self, f: impl FnOnce(&Self::Value) -> bool) -> Option<Self> {
        if f(self.get()) {
            self.remove();
            return None;
        }
        Some(self)
    }
}
impl<'a, K, V, S> MapOccupiedEntryExtension for hashbrown::hash_map::OccupiedEntry<'a, K, V, S> {
    type Value = V;
    #[inline(always)]
    fn and_modify(mut self, f: impl FnOnce(&mut Self::Value)) -> Self {
        f(self.get_mut());
        self
    }
    #[inline(always)]
    fn remove_if(self, f: impl FnOnce(&Self::Value) -> bool) -> Option<Self> {
        if f(self.get()) {
            self.remove();
            return None;
        }
        Some(self)
    }
}
impl<'a, K, V> MapOccupiedEntryExtension for linear_map::OccupiedEntry<'a, K, V> {
    type Value = V;
    #[inline(always)]
    fn and_modify(mut self, f: impl FnOnce(&mut Self::Value)) -> Self {
        f(self.get_mut());
        self
    }
    #[inline(always)]
    fn remove_if(self, f: impl FnOnce(&Self::Value) -> bool) -> Option<Self> {
        if f(self.get()) {
            self.remove();
            return None;
        }
        Some(self)
    }
}
