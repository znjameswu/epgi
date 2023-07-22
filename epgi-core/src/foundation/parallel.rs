// pub trait IntoExactSizeIterator:
//     IntoIterator<IntoIter = <Self as IntoExactSizeIterator>::IntoIter>
// {
//     type IntoIter: ExactSizeIterator<Item = <Self as IntoIterator>::Item>;

//     fn len(&self) -> usize;
// }

// impl<T> IntoExactSizeIterator for Vec<T> {
//     type IntoIter = <Self as IntoIterator>::IntoIter;

//     // fn into_iter(self) -> <Self as IntoExactSizeIterator>::IntoIter {
//     //     <Self as IntoIterator>::into_iter(self)
//     // }

//     fn len(&self) -> usize {
//         Self::len(self)
//     }
// }

// impl<T, const N: usize> IntoExactSizeIterator for [T; N] {
//     type IntoIter = <Self as IntoIterator>::IntoIter;

//     // fn into_iter(self) -> <Self as IntoExactSizeIterator>::IntoIter {
//     //     <Self as IntoIterator>::into_iter(self)
//     // }

//     fn len(&self) -> usize {
//         N
//     }
// }

// impl<T, const N: usize> IntoExactSizeIterator for &[T; N] {
//     type IntoIter = <Self as IntoIterator>::IntoIter;

//     // fn into_iter(self) -> <Self as IntoExactSizeIterator>::IntoIter {
//     //     <Self as IntoIterator>::into_iter(self)
//     // }

//     fn len(&self) -> usize {
//         N
//     }
// }

// impl<T> IntoExactSizeIterator for &[T] {
//     type IntoIter = <Self as IntoIterator>::IntoIter;

//     // fn into_iter(self) -> <Self as IntoExactSizeIterator>::IntoIter {
//     //     <Self as IntoIterator>::into_iter(self)
//     // }

//     fn len(&self) -> usize {
//         (*self).len()
//     }
// }

// impl<T> IntoExactSizeIterator for Option<T> {
//     type IntoIter = <Self as IntoIterator>::IntoIter;

//     // fn into_iter(self) -> <Self as IntoExactSizeIterator>::IntoIter {
//     //     <Self as IntoIterator>::into_iter(self)
//     // }

//     fn len(&self) -> usize {
//         match self {
//             Some(_) => 1,
//             None => 0,
//         }
//     }
// }

// // // Work-around for E0119 "upstream crates may add a new impl of trait"
// // pub struct IteratorWrapper<T: Iterator>(T);

// // impl<T> IntoIterator for IteratorWrapper<T>
// // where
// //     T: Iterator,
// // {
// //     type IntoIter = T;
// //     type Item = T::Item;

// //     fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
// //         self.0
// //     }
// // }

// // impl<T> IntoExactSizeIterator for IteratorWrapper<T>
// // where
// //     T: ExactSizeIterator,
// // {
// //     type IntoIter = T;

// //     fn into_iter(self) -> <Self as IntoExactSizeIterator>::IntoIter {
// //         self.0
// //     }

// //     fn len(&self) -> usize {
// //         self.0.len()
// //     }
// // }

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
    type Container<T>: Parallel<Item = T, HktContainer = Self>
    where
        T: Send;
}

pub trait Parallel: IntoSendExactSizeIterator<Item = <Self as Parallel>::Item> {
    type Item: Send;

    type HktContainer: HktContainer<Container<<Self as Parallel>::Item> = Self>;

    fn par_for_each<F: Fn(<Self as Parallel>::Item) + Send + Sync, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: F,
    );

    fn par_map_collect<F: Fn(<Self as Parallel>::Item) -> R + Send + Sync, R: Send, P: ThreadPoolExt>(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn map<F: FnMut(<Self as Parallel>::Item) -> R, R: Send>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;

    fn map_ref<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R>;
}

pub struct VecContainer;

impl HktContainer for VecContainer {
    type Container<T> = Vec<T> where T:Send;
}

impl<T> Parallel for Vec<T>
where
    T: Send,
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
        R: Send,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        pool.par_map_collect_vec(self, f)
    }

    fn map<F: FnMut(<Self as Parallel>::Item) -> R, R: Send>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        self.into_iter().map(f).collect()
    }

    fn map_ref<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        self.iter().map(f).collect()
    }
}

pub struct ArrayContainer<const N: usize>;

impl<const N: usize> HktContainer for ArrayContainer<N> {
    type Container<T> = [T;N] where T:Send;
}

impl<T, const N: usize> Parallel for [T; N]
where
    T: Send,
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
        R: Send,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        pool.par_map_collect_arr(self, f)
    }

    fn map<F: FnMut(<Self as Parallel>::Item) -> R, R: Send>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        self.map(f)
    }

    fn map_ref<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        std::array::from_fn::<_, N, _>(|i| &self[i]).map(f)
    }
}

// impl<A, B> IntoExactSizeIterator for EitherIntoExactSizeIterator<A, B>
// where
//     A: IntoExactSizeIterator,
//     B: IntoExactSizeIterator<Item = A::Item>,
// {
//     type IntoIter = either::Either<
//         <A as IntoExactSizeIterator>::IntoIter,
//         <B as IntoExactSizeIterator>::IntoIter,
//     >;
//     fn len(&self) -> usize {
//         match &self.0 {
//             either::Either::Left(x) => x.len(),
//             either::Either::Right(x) => x.len(),
//         }
//     }
// }

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
    type Container<T> = EitherParallel<A::Container<T>, B::Container<T>> where T:Send;
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
        R: Send,
        P: ThreadPoolExt,
    >(
        self,
        pool: &P,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        todo!()
    }

    fn map<F: FnMut(<Self as Parallel>::Item) -> R, R: Send>(
        self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        match self.0 {
            either::Either::Left(a) => EitherParallel(either::Either::Left(a.map(f))),
            either::Either::Right(b) => EitherParallel(either::Either::Right(b.map(f))),
        }
    }

    fn map_ref<F: FnMut(&<Self as Parallel>::Item) -> R, R: Send>(
        &self,
        f: F,
    ) -> <Self::HktContainer as HktContainer>::Container<R> {
        match &self.0 {
            either::Either::Left(a) => EitherParallel(either::Either::Left(a.map_ref(f))),
            either::Either::Right(b) => EitherParallel(either::Either::Right(b.map_ref(f))),
        }
    }
}
