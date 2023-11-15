use std::marker::PhantomData;

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
    type Container<T>: Parallel<Item = T, HktContainer = Self> + Send + Sync
    where
        T: Send + Sync;

    const IS_ALWAYS_EMPTY: bool = false;
    #[inline(always)]
    fn try_create_empty<T: Send + Sync>() -> Self::Container<T> {
        panic!("The container is not always empty. Therefore, an empty container cannot be created out of thin air")
    }
}

pub trait Parallel:
    IntoSendExactSizeIterator<Item = <Self as Parallel>::Item>
    + AsIterator<Item = <Self as Parallel>::Item>
    + Send
{
    // We use a duplicate type parameter because it helps to avoid GAT lifetime error (TODO: issue link)
    type Item: Send + Sync;

    // We use an explicit centralized HKT type instead of GAT, because GAT cannot guarantee type equality after two hops away.
    // E.g., GAT can guarantee `Vec::<i32>::Container<i64>::Container::<i32> == Vec<i32>`.
    // But GAT cannot infer anything on `Vec::<i32>::Container<i64>::Container::<i16>::Container<i32>`.
    type HktContainer: HktContainer<Container<<Self as Parallel>::Item> = Self>;

    fn par_for_each<F: Fn(<Self as Parallel>::Item) + Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: F,
    );

    fn par_map_collect<
        F: Fn(<Self as Parallel>::Item) -> R + Send + Sync,
        R: Send + Sync,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn map_collect<F: FnMut(<Self as Parallel>::Item) -> R, R: Send + Sync>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn map_ref_collect<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send + Sync>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn zip_collect<T: Send + Sync, R: Send + Sync>(
        self,
        other: <Self::HktContainer as HktContainer>::Container<T>,
        op: impl FnMut(<Self as Parallel>::Item, T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(<Self as Parallel>::Item) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    );

    fn zip_ref_collect<T: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T>,
        op: impl FnMut(&<Self as Parallel>::Item, T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn any(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool;

    fn all(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool;
}

pub struct VecContainer;

impl HktContainer for VecContainer {
    type Container<T> = Vec<T> where T:Send + Sync;
}

impl<T> Parallel for Vec<T>
where
    T: Send + Sync,
{
    type Item = T;

    type HktContainer = VecContainer;

    fn par_for_each<F: Fn(<Self as Parallel>::Item) + Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: F,
    ) {
        pool.par_for_each_vec(self, f)
    }

    fn par_map_collect<
        F: Fn(<Self as Parallel>::Item) -> R + Send + Sync,
        R: Send + Sync,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        pool.par_map_collect_vec(self, f)
    }

    fn map_collect<F: FnMut(<Self as Parallel>::Item) -> R, R: Send + Sync>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        self.into_iter().map(f).collect()
    }

    fn map_ref_collect<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send + Sync>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        self.as_iter().map(f).collect()
    }

    fn zip_collect<T1: Send + Sync, R: Send + Sync>(
        self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(<Self as Parallel>::Item, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(<Self as Parallel>::Item) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    ) {
        todo!()
    }

    fn zip_ref_collect<T1: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(&<Self as Parallel>::Item, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn any(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
    }

    fn all(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
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

impl<T, const N: usize> Parallel for [T; N]
where
    T: Send + Sync,
{
    type Item = T;

    type HktContainer = ArrayContainer<N>;

    fn par_for_each<F: Fn(<Self as Parallel>::Item) + Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: F,
    ) {
        pool.par_for_each_arr(self, f)
    }

    fn par_map_collect<
        F: Fn(<Self as Parallel>::Item) -> R + Send + Sync,
        R: Send + Sync,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        pool.par_map_collect_arr(self, f)
    }

    fn map_collect<F: FnMut(<Self as Parallel>::Item) -> R, R: Send + Sync>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        self.map(f)
    }

    fn map_ref_collect<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send + Sync>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        std::array::from_fn::<_, N, _>(|i| &self[i]).map(f)
    }

    fn zip_collect<T1: Send + Sync, R: Send + Sync>(
        self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(<Self as Parallel>::Item, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(<Self as Parallel>::Item) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    ) {
        todo!()
    }

    fn zip_ref_collect<T1: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(&<Self as Parallel>::Item, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn any(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
    }

    fn all(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
    }
}

pub struct OptionContainer;

impl HktContainer for OptionContainer {
    type Container<T> = Option<T> where T: Send + Sync;
}

impl<T> Parallel for Option<T>
where
    T: Send + Sync,
{
    type Item = T;

    type HktContainer = OptionContainer;

    fn par_for_each<F: Fn(<Self as Parallel>::Item) + Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: F,
    ) {
        todo!()
    }

    fn par_map_collect<
        F: Fn(<Self as Parallel>::Item) -> R + Send + Sync,
        R: Send + Sync,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn map_collect<F: FnMut(<Self as Parallel>::Item) -> R, R: Send + Sync>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn map_ref_collect<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send + Sync>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn zip_collect<T1: Send + Sync, R: Send + Sync>(
        self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(<Self as Parallel>::Item, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(<Self as Parallel>::Item) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    ) {
        todo!()
    }

    fn zip_ref_collect<T1: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T1>,
        op: impl FnMut(&<Self as Parallel>::Item, T1) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn any(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
    }

    fn all(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
    }
}

pub struct EitherParallel<A, B>(pub either::Either<A, B>)
where
    A: Parallel,
    B: Parallel<Item = <A as Parallel>::Item>;

impl<A, B> EitherParallel<A, B>
where
    A: Parallel,
    B: Parallel<Item = <A as Parallel>::Item>,
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
    A: Parallel,
    B: Parallel<Item = <A as Parallel>::Item>,
{
    type IntoIter = either::Either<<A as IntoIterator>::IntoIter, <B as IntoIterator>::IntoIter>;
    type Item = <A as Parallel>::Item;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, A, B> IntoIterator for &'a EitherParallel<A, B>
where
    A: Parallel,
    B: Parallel<Item = <A as Parallel>::Item>,
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

impl<A, B> Parallel for EitherParallel<A, B>
where
    A: Parallel,
    B: Parallel<Item = <A as Parallel>::Item>,
{
    type Item = <A as Parallel>::Item;

    type HktContainer = EitherContainer<A::HktContainer, B::HktContainer>;

    fn par_for_each<F: Fn(<Self as Parallel>::Item) + Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: F,
    ) {
        todo!()
    }

    fn par_map_collect<
        F: Fn(<Self as Parallel>::Item) -> R + Send + Sync,
        R: Send + Sync,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn map_collect<F: FnMut(<Self as Parallel>::Item) -> R, R: Send + Sync>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        match self.0 {
            either::Either::Left(a) => EitherParallel(either::Either::Left(a.map_collect(f))),
            either::Either::Right(b) => EitherParallel(either::Either::Right(b.map_collect(f))),
        }
    }

    fn map_ref_collect<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send + Sync>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        match &self.0 {
            either::Either::Left(a) => EitherParallel(either::Either::Left(a.map_ref_collect(f))),
            either::Either::Right(b) => EitherParallel(either::Either::Right(b.map_ref_collect(f))),
        }
    }

    fn zip_collect<T: Send + Sync, R: Send + Sync>(
        self,
        other: <Self::HktContainer as HktContainer>::Container<T>,
        op: impl FnMut(<Self as Parallel>::Item, T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl FnMut(<Self as Parallel>::Item) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    ) {
        todo!()
    }

    fn zip_ref_collect<T: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T>,
        op: impl FnMut(&<Self as Parallel>::Item, T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn any(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
    }

    fn all(&self, op: impl Fn(&<Self as Parallel>::Item) -> bool) -> bool {
        todo!()
    }
}
