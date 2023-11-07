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

pub trait HktContainer {
    type Container<T>: Parallel<Item = T, HktContainer = Self> + Send + Sync
    where
        T: Send + Sync;
}

pub trait Parallel: IntoSendExactSizeIterator<Item = <Self as Parallel>::Item> + Send {
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
        op: impl Fn(<Self as Parallel>::Item, T) -> R,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn unzip_collect<R1: Send + Sync, R2: Send + Sync>(
        self,
        op: impl Fn(<Self as Parallel>::Item) -> (R1, R2),
    ) -> (
        <Self::HktContainer as HktContainer>::Container<R1>,
        <Self::HktContainer as HktContainer>::Container<R2>,
    );

    fn zip_ref_collect<T: Send + Sync, R: Send + Sync>(
        &self,
        other: <Self::HktContainer as HktContainer>::Container<T>,
        op: impl Fn(&<Self as Parallel>::Item, T) -> R,
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
        self.iter().map(f).collect()
    }
}

pub struct ArrayContainer<const N: usize>;

impl<const N: usize> HktContainer for ArrayContainer<N> {
    type Container<T> = [T;N] where T:Send + Sync;
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
}

pub struct EitherParallel<Left, Right>(pub either::Either<Left, Right>);

impl<A, B> IntoIterator for EitherParallel<A, B>
where
    A: IntoIterator,
    B: IntoIterator<Item = A::Item>,
{
    type IntoIter = either::Either<A::IntoIter, B::IntoIter>;
    type Item = A::Item;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.0.into_iter()
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
}
