use std::{borrow::Borrow, hash::Hash};

use super::{Asc, Aweak};

/// A wrapper for reference-counting pointers to perform pointer-based equality comparison.
///
/// The pointer comparison ignores the vtable from the fat reference-counting pointers, as discussed in https://github.com/rust-lang/rust/issues/103763。
///
/// To avoid a false positive, it is required that the reference-counting pointer implementation must have no pointer aliasing under any circumstances. Most implementations do guarantee no pointer aliasing, including `std`. However, it is theoretically possible to implement a strong-only reference-counting pointer with allocation-free specialization on ZSTs such that ZSTs have pointer aliasing. Such implementations would be considered not standard-conforming after https://github.com/rust-lang/rust/pull/106450.
#[repr(transparent)]
#[derive(Clone)]
pub struct PtrEq<T: AsHeapPtr>(pub T);

pub trait AsHeapPtr {
    type Pointee: ?Sized;
    fn as_heap_ptr(&self) -> *const Self::Pointee;
}

impl<T> AsHeapPtr for Asc<T>
where
    T: ?Sized,
{
    type Pointee = T;
    fn as_heap_ptr(&self) -> *const Self::Pointee {
        Asc::as_ptr(self)
    }
}

impl<T> AsHeapPtr for Aweak<T>
where
    T: ?Sized,
{
    type Pointee = T;
    fn as_heap_ptr(&self) -> *const Self::Pointee {
        Aweak::as_ptr(self)
    }
}

impl<'a, T> AsHeapPtr for &'a T
where
    T: AsHeapPtr,
{
    type Pointee = <T as AsHeapPtr>::Pointee;

    fn as_heap_ptr(&self) -> *const Self::Pointee {
        T::as_heap_ptr(*self)
    }
}

impl<T> Hash for PtrEq<T>
where
    T: AsHeapPtr,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0.as_heap_ptr() as *const ()).hash(state);
    }
}

impl<T> PartialEq for PtrEq<T>
where
    T: AsHeapPtr,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.as_heap_ptr() as *const () == other.0.as_heap_ptr() as *const ()
    }
}

impl<T> Eq for PtrEq<T> where T: AsHeapPtr {}

impl<T> From<T> for PtrEq<T>
where
    T: AsHeapPtr,
{
    fn from(v: T) -> Self {
        Self(v)
    }
}

impl<T> Borrow<T> for PtrEq<T>
where
    T: AsHeapPtr,
{
    fn borrow(&self) -> &T {
        // SAFETY: repr transparent
        unsafe { &*(self as *const PtrEq<T> as *const T) }
    }
}

pub trait PtrEqExt: AsHeapPtr + Sized {
    fn as_ref_ptr_eq(&self) -> &PtrEq<Self>;
}

impl<T> PtrEqExt for T
where
    T: AsHeapPtr,
{
    fn as_ref_ptr_eq(&self) -> &PtrEq<Self> {
        unsafe { &*(self as *const _ as *const PtrEq<T>) }
    }
}
