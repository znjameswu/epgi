use std::{marker::PhantomData, mem::MaybeUninit};

use crate::foundation::ThreadPoolExt;

/// Workaround for https://github.com/bluss/either/issues/
///
/// The issue is explained in https://github.com/bluss/either/issues/31#issuecomment-422210358

pub trait IntoSendExactSizeIterator:
    IntoIterator<
    Item = <Self as IntoSendExactSizeIterator>::Item,
    IntoIter = <Self as IntoSendExactSizeIterator>::IntoIter,
>
{
    type Item: Send;
    type IntoIter: ExactSizeIterator<Item = <Self as IntoIterator>::Item> + Send;
}

impl<T> IntoSendExactSizeIterator for T
where
    T: IntoIterator,
    T::IntoIter: ExactSizeIterator<Item = T::Item> + Send,
    T::Item: Send,
{
    type Item = T::Item;
    type IntoIter = T::IntoIter;
}

pub trait AsIterator {
    type IntoIter<'a>: Iterator<Item = &'a <Self as AsIterator>::Item>
    where
        Self: 'a;

    type Item;

    fn as_iter(&self) -> Self::IntoIter<'_>;
}

impl<T: ?Sized> AsIterator for T
where
    T: IntoIterator,
    for<'any> &'any T: IntoIterator<Item = &'any <T as IntoIterator>::Item>,
{
    type IntoIter<'a> = <&'a Self as IntoIterator>::IntoIter where Self: 'a;
    type Item = <T as IntoIterator>::Item;
    fn as_iter(&self) -> Self::IntoIter<'_> {
        self.into_iter()
    }
}

pub trait HktContainer {
    type Container<T>: Container<Item = T, HktContainer = Self> + Send + Sync
    where
        T: Send + Sync;

    const IS_ALWAYS_EMPTY: bool = false;
    #[inline(always)]
    fn try_create_empty<T: Send + Sync>() -> Self::Container<T> {
        panic!("The container is not always empty. Therefore, an empty container cannot be created out of thin air")
    }
}

pub trait Container:
    IntoSendExactSizeIterator<Item = <Self as Container>::Item>
    + AsIterator<Item = <Self as Container>::Item>
    + Send
{
    // We use a duplicate type parameter because it helps to avoid GAT lifetime error (TODO: issue link)
    type Item: Send + Sync;

    // We use an explicit centralized HKT type instead of GAT, because GAT cannot guarantee type equality after two hops away.
    // E.g., GAT can guarantee `Vec::<i32>::Container<i64>::Container::<i32> == Vec<i32>`.
    // But GAT cannot infer anything on `Vec::<i32>::Container<i64>::Container::<i16>::Container<i32>`.
    type HktContainer: HktContainer<Container<<Self as Container>::Item> = Self>;

    fn par_for_each<P: ThreadPoolExt>(
        self,
        pool: &P,
        f: impl Fn(<Self as Container>::Item) + Send + Sync,
    );

    fn par_map_collect<R: Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: impl Fn(<Self as Container>::Item) -> R + Send + Sync,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn map_collect<R: Send + Sync>(
        self,
        f: impl FnMut(<Self as Container>::Item) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn map_ref_collect<R: Send + Sync>(
        &self,
        f: impl FnMut(&<Self as Container>::Item) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    /// This differs from a normal iterator zip!!!
    ///
    /// 1. It collects into the same container
    /// 2. Semantically, it pairs element by their conceptual key, rather than index (which is meaningless in some unordered containers). E.g. HashMap
    /// 3. It may panic if fail the pair the values
    fn zip_collect<T: Send + Sync, R: Send + Sync>(
        self,
        other: <Self::HktContainer as HktContainer>::Container<T>,
        op: impl FnMut(<Self as Container>::Item, T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(<Self as Container>::Item) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    );

    fn zip_ref_collect<T: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T>,
        op: impl FnMut(&<Self as Container>::Item, T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;
}

pub struct VecContainer;

impl HktContainer for VecContainer {
    type Container<T> = Vec<T> where T:Send + Sync;
}

impl<T> Container for Vec<T>
where
    T: Send + Sync,
{
    type Item = T;

    type HktContainer = VecContainer;

    fn par_for_each<P: ThreadPoolExt>(self, pool: &P, f: impl Fn(T) + Send + Sync) {
        pool.par_for_each_vec(self, f)
    }

    fn par_map_collect<R: Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: impl Fn(T) -> R + Send + Sync,
    ) -> Vec<R> {
        pool.par_map_collect_vec(self, f)
    }

    fn map_collect<R: Send + Sync>(self, f: impl FnMut(T) -> R) -> Vec<R> {
        self.into_iter().map(f).collect()
    }

    fn map_ref_collect<R: Send + Sync>(&self, f: impl FnMut(&T) -> R) -> Vec<R> {
        self.as_iter().map(f).collect()
    }

    fn zip_collect<T1: Send + Sync, R: Send + Sync>(
        self,
        other: Vec<T1>,
        mut op: impl FnMut(T, T1) -> R,
    ) -> Vec<R> {
        self.into_iter().zip(other).map(|(x, y)| op(x, y)).collect()
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(T) -> (R1, R2),
    ) -> (Vec<R1>, Vec<R2>) {
        self.into_iter().map(op).unzip()
    }

    fn zip_ref_collect<T1: Send + Sync, R: Send + Sync>(
        &self,
        other: Vec<T1>,
        mut op: impl FnMut(&T, T1) -> R,
    ) -> Vec<R> {
        self.iter().zip(other).map(|(x, y)| op(x, y)).collect()
    }
}

pub struct ArrayContainer<const N: usize>;

impl<const N: usize> HktContainer for ArrayContainer<N> {
    type Container<T> = [T;N] where T:Send + Sync;

    const IS_ALWAYS_EMPTY: bool = N == 0;
    fn try_create_empty<T: Send + Sync>() -> [T; N] {
        if N == 0 {
            std::array::from_fn(|_| unsafe { std::mem::MaybeUninit::uninit().assume_init() })
        } else {
            panic!("The container is not always empty. Therefore, an empty container cannot be created out of thin air")
        }
    }
}

impl<T, const N: usize> Container for [T; N]
where
    T: Send + Sync,
{
    type Item = T;

    type HktContainer = ArrayContainer<N>;

    fn par_for_each<P: ThreadPoolExt>(self, pool: &P, f: impl Fn(T) + Send + Sync) {
        pool.par_for_each_arr(self, f)
    }

    fn par_map_collect<R: Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: impl Fn(T) -> R + Send + Sync,
    ) -> [R; N] {
        pool.par_map_collect_arr(self, f)
    }

    fn map_collect<R: Send + Sync>(self, f: impl FnMut(T) -> R) -> [R; N] {
        self.map(f)
    }

    fn map_ref_collect<R: Send + Sync>(&self, mut f: impl FnMut(&T) -> R) -> [R; N] {
        // std::array::each_ref
        std::array::from_fn::<_, N, _>(|i| f(&self[i]))
    }

    fn zip_collect<T1: Send + Sync, R: Send + Sync>(
        self,
        other: [T1; N],
        mut op: impl FnMut(T, T1) -> R,
    ) -> [R; N] {
        // Optimization result: https://godbolt.org/z/4eo43qfhd
        // Very good zero cost abstraction for our concerned scenarios. In constrast, IntoIterator::into_iter causes asm code explosion.
        // SAFETY: https://github.com/rust-lang/rust/blob/28345f06d785213e6d37de5464c7070a4fc9ca67/library/core/src/array/iter.rs#L58
        unsafe {
            let self_data: [MaybeUninit<T>; N] = self.map(|x| MaybeUninit::new(x));
            let other_data: [MaybeUninit<T1>; N] = other.map(|x| MaybeUninit::new(x));
            let result: [MaybeUninit<R>; N] = MaybeUninit::uninit().assume_init();

            // Guard types to ensure drops are properly executed in case of a panic unwind
            struct Guard<T, T1, R, const N: usize> {
                index: usize,
                self_data: [MaybeUninit<T>; N],
                other_data: [MaybeUninit<T1>; N],
                result: [MaybeUninit<R>; N],
            }

            let mut guard = Guard {
                index: 0,
                self_data,
                other_data,
                result,
            };

            while guard.index < N {
                let elem = op(
                    guard
                        .self_data
                        .get_unchecked(guard.index)
                        .assume_init_read(),
                    guard
                        .other_data
                        .get_unchecked(guard.index)
                        .assume_init_read(),
                );
                impl<T, T1, R, const N: usize> Drop for Guard<T, T1, R, N> {
                    // Drop resources when unwinded.
                    fn drop(&mut self) {
                        // If index == N, it means we must have successfully exited the loop
                        // It also means we must have successfully taken out the result. Therefore, we do not need to drop the result.
                        // Also, by this check, we avoid a negative length range indexing problem
                        unsafe {
                            if self.index < N {
                                // The panic must have happened after we read the indexed elements out from the sources.
                                // Therefore, drop from the element after it. Do not include the current element.
                                let self_slice =
                                    self.self_data.get_unchecked_mut(self.index + 1..N);
                                let other_slice =
                                    self.other_data.get_unchecked_mut(self.index + 1..N);
                                // The panic must have happened before we write the indexed element into the result.
                                // Therefore, drop until the element before it. Do not include the current element.
                                let result_slice = self.result.get_unchecked_mut(0..self.index);
                                std::ptr::drop_in_place(slice_assume_init_mut(self_slice));
                                std::ptr::drop_in_place(slice_assume_init_mut(other_slice));
                                std::ptr::drop_in_place(slice_assume_init_mut(result_slice));
                            }
                        }
                    }
                }
                guard.result.get_unchecked_mut(guard.index).write(elem);
                guard.index += 1;
            }

            debug_assert!(guard.index == N);
            // We bit copy the result out, while the guard and the MaybeUninit field all forget about dropping. Mission accomplished.
            return std::mem::transmute_copy(&guard.result);
        }
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        mut op: impl FnMut(T) -> (R1, R2),
    ) -> ([R1; N], [R2; N]) {
        unsafe {
            // SAFETY:
            //https://github.com/rust-lang/rust/blob/28345f06d785213e6d37de5464c7070a4fc9ca67/library/core/src/mem/maybe_uninit.rs#L115
            let self_data: [MaybeUninit<T>; N] = self.map(|x| MaybeUninit::new(x));
            let result1: [MaybeUninit<R1>; N] = MaybeUninit::uninit().assume_init();
            let result2: [MaybeUninit<R2>; N] = MaybeUninit::uninit().assume_init();

            // Guard types to ensure drops are properly executed in case of a panic unwind
            struct Guard<T, R1, R2, const N: usize> {
                index: usize,
                self_data: [MaybeUninit<T>; N],
                result1: [MaybeUninit<R1>; N],
                result2: [MaybeUninit<R2>; N],
            }

            let mut guard = Guard {
                index: 0,
                self_data,
                result1,
                result2,
            };

            while guard.index < N {
                let elem = guard
                    .self_data
                    .get_unchecked(guard.index)
                    .assume_init_read();
                let (elem1, elem2) = op(elem);

                impl<T, R1, R2, const N: usize> Drop for Guard<T, R1, R2, N> {
                    // Drop resources when unwinded.
                    fn drop(&mut self) {
                        unsafe {
                            if self.index < N {
                                // The panic must have happened after we read the indexed elements out from the sources.
                                // Therefore, drop from the element after it. Do not include the current element.
                                let self_slice =
                                    self.self_data.get_unchecked_mut(self.index + 1..N);
                                // The panic must have happened before we write the indexed element into the result.
                                // Therefore, drop until the element before it. Do not include the current element.
                                let result1_slice = self.result1.get_unchecked_mut(0..self.index);
                                let result2_slice = self.result2.get_unchecked_mut(0..self.index);
                                std::ptr::drop_in_place(slice_assume_init_mut(self_slice));
                                std::ptr::drop_in_place(slice_assume_init_mut(result1_slice));
                                std::ptr::drop_in_place(slice_assume_init_mut(result2_slice));
                            }
                        }
                    }
                }
                guard.result1.get_unchecked_mut(guard.index).write(elem1);
                guard.result2.get_unchecked_mut(guard.index).write(elem2);
                guard.index += 1;
            }

            debug_assert!(guard.index == N);
            // We bit copy the result out, while the guard and the MaybeUninit field all forget about dropping. Mission accomplished.
            return (
                std::mem::transmute_copy(&guard.result1),
                std::mem::transmute_copy(&guard.result2),
            );
        }
    }

    fn zip_ref_collect<T1: Send + Sync, R: Send + Sync>(
        &self,
        other: [T1; N],
        mut op: impl FnMut(&T, T1) -> R,
    ) -> [R; N] {
        unsafe {
            let other_data: [MaybeUninit<T1>; N] = other.map(|x| MaybeUninit::new(x));
            let result: [MaybeUninit<R>; N] = MaybeUninit::uninit().assume_init();

            // Guard types to ensure drops are properly executed in case of a panic unwind
            struct Guard<T1, R, const N: usize> {
                index: usize,
                other_data: [MaybeUninit<T1>; N],
                result: [MaybeUninit<R>; N],
            }

            let mut guard = Guard {
                index: 0,
                other_data,
                result,
            };

            while guard.index < N {
                let elem = op(
                    self.get_unchecked(guard.index),
                    guard
                        .other_data
                        .get_unchecked(guard.index)
                        .assume_init_read(),
                );
                impl<T1, R, const N: usize> Drop for Guard<T1, R, N> {
                    // Drop resources when unwinded.
                    fn drop(&mut self) {
                        // If index == N, it means we must have successfully exited the loop
                        // It also means we must have successfully taken out the result. Therefore, we do not need to drop the result.
                        // Also, by this check, we avoid a negative length range indexing problem
                        unsafe {
                            if self.index < N {
                                // The panic must have happened after we read the indexed elements out from the sources.
                                // Therefore, drop from the element after it. Do not include the current element.
                                let other_slice =
                                    self.other_data.get_unchecked_mut(self.index + 1..N);
                                // The panic must have happened before we write the indexed element into the result.
                                // Therefore, drop until the element before it. Do not include the current element.
                                let result_slice = self.result.get_unchecked_mut(0..self.index);
                                std::ptr::drop_in_place(slice_assume_init_mut(other_slice));
                                std::ptr::drop_in_place(slice_assume_init_mut(result_slice));
                            }
                        }
                    }
                }
                guard.result.get_unchecked_mut(guard.index).write(elem);
                guard.index += 1;
            }

            debug_assert!(guard.index == N);
            // We bit copy the result out, while the guard and the MaybeUninit field all forget about dropping. Mission accomplished.
            return std::mem::transmute_copy(&guard.result);
        }
    }
}

/// Copied from [std::mem::MaybeUninit]
unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
    // SAFETY: similar to safety notes for `slice_get_ref`, but we have a
    // mutable reference which is also guaranteed to be valid for writes.
    unsafe { &mut *(slice as *mut [MaybeUninit<T>] as *mut [T]) }
}

pub struct OptionContainer;

impl HktContainer for OptionContainer {
    type Container<T> = Option<T> where T: Send + Sync;
}

impl<T> Container for Option<T>
where
    T: Send + Sync,
{
    type Item = T;

    type HktContainer = OptionContainer;

    fn par_for_each<P: ThreadPoolExt>(self, _pool: &P, f: impl Fn(T) + Send + Sync) {
        let Some(item) = self else { return };
        f(item)
    }

    fn par_map_collect<R: Send + Sync, P: ThreadPoolExt>(
        self,
        _pool: &P,
        f: impl Fn(T) -> R + Send + Sync,
    ) -> Option<R> {
        let Some(item) = self else { return None };
        Some(f(item))
    }

    fn map_collect<R: Send + Sync>(self, f: impl FnMut(T) -> R) -> Option<R> {
        self.map(f)
    }

    fn map_ref_collect<R: Send + Sync>(&self, f: impl FnMut(&T) -> R) -> Option<R> {
        self.as_ref().map(f)
    }

    fn zip_collect<T1: Send + Sync, R: Send + Sync>(
        self,
        other: Option<T1>,
        mut op: impl FnMut(T, T1) -> R,
    ) -> Option<R> {
        match (self, other) {
            (Some(x), Some(y)) => Some(op(x, y)),
            (None, None) => None,
            _ => panic!("The two containers cannot be zipped together"),
        }
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        mut op: impl FnMut(T) -> (R1, R2),
    ) -> (Option<R1>, Option<R2>) {
        if let Some(elem) = self {
            let (elem1, elem2) = op(elem);
            return (Some(elem1), Some(elem2));
        } else {
            return (None, None);
        }
    }

    fn zip_ref_collect<T1: Send + Sync, R: Send + Sync>(
        &self,
        other: Option<T1>,
        mut op: impl FnMut(&T, T1) -> R,
    ) -> Option<R> {
        match (self, other) {
            (Some(x), Some(y)) => Some(op(x, y)),
            (None, None) => None,
            _ => panic!("The two containers cannot be zipped together"),
        }
    }
}

pub struct EitherParallel<A, B>(pub either::Either<A, B>)
where
    A: Container,
    B: Container<Item = <A as Container>::Item>;

impl<A, B> EitherParallel<A, B>
where
    A: Container,
    B: Container<Item = <A as Container>::Item>,
{
    pub fn new_left(value: A) -> Self {
        Self(either::Either::Left(value))
    }

    pub fn new_right(value: B) -> Self {
        Self(either::Either::Right(value))
    }
}

impl<A, B> IntoIterator for EitherParallel<A, B>
where
    A: Container,
    B: Container<Item = <A as Container>::Item>,
{
    type IntoIter = either::Either<<A as IntoIterator>::IntoIter, <B as IntoIterator>::IntoIter>;
    type Item = <A as Container>::Item;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, A, B> IntoIterator for &'a EitherParallel<A, B>
where
    A: Container,
    B: Container<Item = <A as Container>::Item>,
{
    type IntoIter =
        either::Either<<A as AsIterator>::IntoIter<'a>, <B as AsIterator>::IntoIter<'a>>;
    type Item = &'a <EitherParallel<A, B> as IntoIterator>::Item;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        match &self.0 {
            either::Either::Left(x) => either::Either::Left(x.as_iter()),
            either::Either::Right(x) => either::Either::Right(x.as_iter()),
        }
        // either::Either bug here. WE CAN'T USE MAP_EITHER
    }
}

pub struct EitherContainer<A, B>(PhantomData<A>, PhantomData<B>);

impl<A, B> HktContainer for EitherContainer<A, B>
where
    A: HktContainer,
    B: HktContainer,
{
    type Container<T> = EitherParallel<A::Container<T>, B::Container<T>> where T:Send + Sync;
}

impl<A, B, T> Container for EitherParallel<A, B>
where
    A: Container<Item = T>,
    B: Container<Item = T>,
    T: Send + Sync,
{
    type Item = <A as Container>::Item;

    type HktContainer = EitherContainer<A::HktContainer, B::HktContainer>;

    fn par_for_each<P: ThreadPoolExt>(self, pool: &P, f: impl Fn(T) + Send + Sync) {
        use either::Either::*;
        match self.0 {
            Left(x) => x.par_for_each(pool, f),
            Right(x) => x.par_for_each(pool, f),
        }
    }

    fn par_map_collect<R: Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: impl Fn(T) -> R + Send + Sync,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        use either::Either::*;
        match self.0 {
            Left(x) => EitherParallel(Left(x.par_map_collect(pool, f))),
            Right(x) => EitherParallel(Right(x.par_map_collect(pool, f))),
        }
    }

    fn map_collect<R: Send + Sync>(
        self,
        f: impl FnMut(T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        use either::Either::*;
        match self.0 {
            Left(x) => EitherParallel(Left(x.map_collect(f))),
            Right(x) => EitherParallel(Right(x.map_collect(f))),
        }
    }

    fn map_ref_collect<R: Send + Sync>(
        &self,
        f: impl FnMut(&T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        use either::Either::*;
        match &self.0 {
            Left(x) => EitherParallel(Left(x.map_ref_collect(f))),
            Right(x) => EitherParallel(Right(x.map_ref_collect(f))),
        }
    }

    fn zip_collect<T1: Send + Sync, R: Send + Sync>(
        self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(T, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        use either::Either::*;
        match (self.0, other.0) {
            (Left(x), Left(y)) => EitherParallel(Left(x.zip_collect(y, op))),
            (Right(x), Right(y)) => EitherParallel(Right(x.zip_collect(y, op))),
            _ => panic!("The two containers cannot be zipped together"),
        }
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(T) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    ) {
        use either::Either::*;
        match self.0 {
            Left(x) => {
                let (_0, _1) = x.unzip_collect(op);
                (EitherParallel(Left(_0)), EitherParallel(Left(_1)))
            }
            Right(x) => {
                let (_0, _1) = x.unzip_collect(op);
                (EitherParallel(Right(_0)), EitherParallel(Right(_1)))
            }
        }
    }

    fn zip_ref_collect<T1: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(&T, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        use either::Either::*;
        match (&self.0, other.0) {
            (Left(x), Left(y)) => EitherParallel(Left(x.zip_ref_collect(y, op))),
            (Right(x), Right(y)) => EitherParallel(Right(x.zip_ref_collect(y, op))),
            _ => panic!("The two containers cannot be zipped together"),
        }
    }
}
