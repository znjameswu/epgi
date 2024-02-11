//! ## Interface query
//! By "interface query", we refer to the process of downcasting a *selected* trait object pointer to
//! other *arbitrary* trait object pointers defined in downstream crates. It usually has a signature like the following
//! ```rust
//! trait TraitA {
//!     fn all_interfaces(&self) -> &[(TypeId, fn(&Self) -> Box<dyn Any>)] where Self: Sized;
//!     fn cast_interface(&self, type_id: TypeId) -> Option<Box<dyn Any>>;
//! }
//! impl dyn TraitA {
//!     // T can be any trait object
//!     fn query_interface<T: ?Sized + 'static>(&self) -> Option<&T> {
//!         self
//!             .cast_interface(TypeId::of::<*const T>())
//!             .map(|box_any| *(box_any.downcast::<*const T>().unwrap()))
//!     }
//! }
//! ```
//!
//! Note how implementers can supply their custom interface conversion entries in `TraitA::all_interfaces`.
//! This is different from normal trait object casting, which can only cast into *selected* trait object pointers
//! within the definition scope of `TraitA`, and every cast must be explicitly special-cased in the base trait signature.
//! ```rust
//! trait TraitA {
//!     // TraitB must be in scope when we define `TraitA`
//!     fn as_b(&self) -> Option<&TraitB>;
//!     fn as_c(&self) -> Option<&TraitC>;
//!     // ...
//! }
//! ```
//!
//! The reason we need interface query is to allow downstream crates to extend functionalities on core EPGI types,
//! therefore separating optional functionalities into other crates. Also, this would prevent those functionalities
//! from (*insert jargon for premature API designs to invade into and permanently leave its trace in core API design*) our core trait signatures.
//! For example, we use interface query to implement pointer, gesture, and keyboard functionalities outside of our core crate.
//!
//! In an interface query, there are usually following stages at play
//! 1. Source trait object pointer (e.g. `&dyn TraitA`) gets vtable dispatched into the concrete type pointer (e.g. `&Self`)
//! 2. Interface conversion table converts the concrete type pointer (e.g. `&Self`) into target trait object pointer (e.g. `*dyn TraitB`)
//! 3. Target trait object pointer was wrapped (e.g. `Box<*dyn TraitB>`) and upcasted (e.g. `Box<dyn Any>`), before returned from the virtual function.
//! 4. Caller receives the upcasted wrapper, and then downcasts (e.g. `Box<*dyn TraitB>`) and unwraps into the target trait object pointer (e.g. `&dyn TraitB`)
//!
//! ## Downcast on stack
//! By default, there exist a major limitation in Rust's trait object downcast mechanism.
//! We either perform the downcast on heap (i.e. `Box<dyn Any>::downcast`), or by ref (i.e. `<dyn Any>::downcast_ref/mut`).
//! There does not exist a way to downcast a trait object on stack with ownership.
//! Since the upcast result must be a local variable, we will be unable to return it by reference.
//! Therefore we are forced to return the upcast result on heap and downcast in heap.
//!
//! Part of the reason for this Rust design, is that the trait object is unsized and cannot be placed on stack.
//! Another reason is that sometimes LLVM backend can completely inline the heap allocation away, so it is okay to downcast on heap.
//!
//! However, the reasoning does not stand in our interface query use-case.
//! 1. Our target trait object pointer is at max two pointer wide, which makes the upcast result (any fat pointer) possible to be placed on stack.
//! 2. Our upcast result will also be a return value from a virtual function call,
//!     disabling compiler inline optimization and certainly triggering heap allocation cost.
//!
//! Therefore, to exploint the use-case, we created a sized container to hold our upcast result on stack.
//! The sized container contains a minimal vtable and double-pointer-size of buffer,
//! making it very much similar to a **flattened `Box<dyn Any>` with its contents (the upcast result) inlined on stack**.
//! Depending on whether the target trait object pointer has a destructor effect, the container has two flavors:
//! [AnyPointer] and [AnyRawPointer].

use std::{any::TypeId, mem::MaybeUninit};

pub struct AnyPointer {
    get_type_id: fn() -> TypeId,
    drop: fn(&mut [*const (); 2]),
    buffer: [*const (); 2],
}

impl AnyPointer {
    pub fn new<T: 'static>(ptr: T) -> Self {
        if std::mem::size_of::<T>() > std::mem::size_of::<[*const (); 2]>() {
            panic!("PointerBox can only hold data up to two pointer size!")
        }
        let mut buffer = MaybeUninit::uninit();
        unsafe {
            (buffer.as_mut_ptr() as *mut T).write(ptr);
            Self {
                get_type_id: TypeId::of::<T>,
                drop: |buffer| std::ptr::drop_in_place(buffer as *mut _ as *mut T),
                buffer: buffer.assume_init(),
            }
        }
    }

    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        if TypeId::of::<T>() == (self.get_type_id)() {
            unsafe {
                let result = std::mem::transmute_copy::<_, T>(&self.buffer);
                std::mem::forget(self);
                Ok(result)
            }
        } else {
            Err(self)
        }
    }
}

impl Drop for AnyPointer {
    fn drop(&mut self) {
        (self.drop)(&mut self.buffer)
    }
}

// Emulating std's dyn Any
impl std::fmt::Debug for AnyPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyPointerBox").finish_non_exhaustive()
    }
}

/// A [AnyPointerBox] that specifically optimizes for raw pointers.
///
/// By only admitting raw pointer, we were relieved of performing polymorphic drop (destruction).
pub struct AnyRawPointer {
    type_id: TypeId,
    buffer: [*const (); 2],
}

impl AnyRawPointer {
    pub fn new_const<T: ?Sized + 'static>(ptr: *const T) -> Self {
        if std::mem::size_of::<*const T>() > std::mem::size_of::<[*const (); 2]>() {
            panic!("RawPointerBox can only hold data up to two pointer size!")
        }
        let mut buffer = MaybeUninit::uninit();
        unsafe {
            (buffer.as_mut_ptr() as *mut *const T).write(ptr);
            Self {
                type_id: TypeId::of::<*const T>(),
                buffer: buffer.assume_init(),
            }
        }
    }

    pub fn downcast_const<T: ?Sized + 'static>(self) -> Result<*const T, Self> {
        if TypeId::of::<*const T>() == self.type_id {
            unsafe {
                let result = std::mem::transmute_copy::<_, *const T>(&self.buffer);
                std::mem::forget(self);
                Ok(result)
            }
        } else {
            Err(self)
        }
    }
}

pub trait CastInterfaceByRawPtr {
    fn cast_interface_raw(&self, raw_ptr_type_id: TypeId) -> Option<AnyRawPointer>;
}

impl dyn CastInterfaceByRawPtr {
    pub fn query_interface<T: ?Sized + 'static>(&self) -> Option<&T> {
        self.cast_interface_raw(TypeId::of::<*const T>())
            .map(|ptr| {
                let downcasted = ptr.downcast_const::<T>().ok().expect(
                    "Interface query table function should return a raw fat pointer \
                    with the type it claimed",
                );
                unsafe { downcasted.as_ref().expect("Impossible to fail") }
            })
    }
}

pub fn cast_interface_by_table_raw<'a, T>(
    this: &'a T,
    raw_ptr_type_id: TypeId,
    table: impl IntoIterator<Item = &'a (TypeId, fn(*const T) -> AnyRawPointer)>,
) -> Option<AnyRawPointer> {
    for (type_id, cast) in table.into_iter() {
        if *type_id == raw_ptr_type_id {
            let ptr = cast(this);
            return Some(ptr);
        }
    }
    return None;
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;

    trait TestTrait {
        fn value(&self) -> i32;
    }

    struct TestStruct(i32);

    lazy_static::lazy_static! {
        static ref INTERFACE_TABLE: [(TypeId, fn(*const TestStruct) -> AnyRawPointer);1] =
            [(TypeId::of::<*const dyn TestTrait>(), |x| {
                AnyRawPointer::new_const(x as *const dyn TestTrait)
            })];
    }

    impl CastInterfaceByRawPtr for TestStruct {
        fn cast_interface_raw(&self, raw_ptr_type_id: TypeId) -> Option<AnyRawPointer> {
            cast_interface_by_table_raw(self, raw_ptr_type_id, INTERFACE_TABLE.as_slice())
        }
    }

    impl TestTrait for TestStruct {
        fn value(&self) -> i32 {
            self.0
        }
    }

    #[test]
    fn query_interface_raw() {
        const I: i32 = 42;
        let x: TestStruct = TestStruct(I);
        let x_ref: &TestStruct = &x;
        let x_up = x_ref as &dyn CastInterfaceByRawPtr;
        let x_down = x_up.query_interface::<dyn TestTrait>().unwrap();
        assert_eq!(x_down.value(), I)
    }
}
