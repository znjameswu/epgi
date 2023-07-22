pub mod backend;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

pub trait ThreadPool {
    fn execute_detached<F: FnOnce() + Send + 'static>(&self, op: F);

    fn par_for_each_vec<T: Send, F: Fn(T) + Send + Sync>(&self, vec: Vec<T>, f: F);
    // fn par_for_each_slice<T: Sync, F: Fn(&T) + Send + Sync>(&self, iter: &[T], f: F);
    fn par_for_each_arr<T: Send, F: Fn(T) + Send + Sync, const N: usize>(&self, arr: [T; N], f: F);

    fn par_map_collect_vec<T: Send, R: Send, F: Fn(T) -> R + Send + Sync>(
        &self,
        vec: Vec<T>,
        f: F,
    ) -> Vec<R>;
    // fn par_map_collect_slice<T: Send, R: Send, F: FnMut(T) -> R>(&self, iter: &[T], f: F)
    //     -> Vec<R>;
    fn par_map_collect_arr<T: Send, R: Send, F: Fn(T) -> R + Send + Sync, const N: usize>(
        &self,
        arr: [T; N],
        f: F,
    ) -> [R; N];
}

// trait ExactSizeSplit: IntoSendExactSizeIterator + Send + Sized {
//     type Producer: ExactSizeSplitProducer<Item = <Self as IntoSendExactSizeIterator>::Item>;
//     fn len(&self) -> usize;
//     fn into_producer(self) -> Self::Producer;
// }

// trait ExactSizeSplitProducer: Send + Sized {
//     type Item;
//     type IntoIter: Iterator<Item = Self::Item> + DoubleEndedIterator + ExactSizeIterator;
//     fn into_iter(self) -> Self::IntoIter;
//     fn split_at(self, index: usize) -> (Self, Self);
// }

// impl<T> ExactSizeSplit for Vec<T>
// where
//     T: Send,
// {
//     type Producer<'a> = VecProducer<'a, T>;

//     fn len(&self) -> usize {
//         todo!()
//     }

//     fn into_producer(self) -> Self::Producer {
//         let len = self.len();
//         unsafe {
//             // Make the vector forget about all items, since the producer will decide on moving/dropping those elements.
//             self.set_len(0);

//             // Create the producer as the exclusive "owner" of the slice.
//             let producer = VecProducer {
//                 vec: std::slice::from_raw_parts_mut(self.as_mut_ptr(), len),
//             };

//             // The producer will move or drop each item from the drained range.
//             callback.callback(producer)
//         }
//     }
// }

// struct VecProducer<'a, T> {
//     vec: &'a mut [T],
// }

// impl<'a, T> ExactSizeSplitProducer for VecProducer<'a, T>
// where
//     T: Send,
// {
//     type Item = T;

//     type IntoIter = SliceDrain<'a, T>;

//     fn into_iter(self) -> Self::IntoIter {
//         SliceDrain { iter: self.vec }
//     }

//     fn split_at(self, index: usize) -> (Self, Self) {}
// }

// struct RayonIntoParIter<T: ExactSizeSplit>(T);

// struct RayonParIter<T: ExactSizeSplit>(T);

// struct RayonProducer<T: ExactSizeSplitProducer>(T);

// impl<T> rayon::iter::IntoParallelIterator for RayonIntoParIter<T>
// where
//     T: ExactSizeSplit,
// {
//     type Iter = RayonParIter<T>;

//     type Item = <T as IntoSendExactSizeIterator>::Item;

//     fn into_par_iter(self) -> Self::Iter {
//         RayonParIter(self.0)
//     }
// }

// impl<T> rayon::iter::ParallelIterator for RayonParIter<T>
// where
//     T: ExactSizeSplit,
// {
//     type Item = <T as IntoSendExactSizeIterator>::Item;

//     fn drive_unindexed<C>(self, consumer: C) -> C::Result
//     where
//         C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
//     {
//         rayon::iter::plumbing::bridge(self, consumer)
//     }
// }

// impl<T> rayon::iter::IndexedParallelIterator for RayonParIter<T>
// where
//     T: ExactSizeSplit,
// {
//     fn len(&self) -> usize {
//         self.0.len()
//     }

//     fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
//         rayon::iter::plumbing::bridge(self, consumer)
//     }

//     fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
//         let producer = RayonProducer(self.0.into_producer());
//         callback.callback(producer)
//     }
// }

// impl<T> rayon::iter::plumbing::Producer for RayonProducer<T>
// where
//     T: ExactSizeSplitProducer,
// {
//     type Item = T::Item;

//     type IntoIter = T::IntoIter;

//     fn into_iter(self) -> Self::IntoIter {
//         self.0.into_iter()
//     }

//     fn split_at(self, index: usize) -> (Self, Self) {
//         let (a, b) = self.0.split_at(index);
//         (RayonProducer(a), RayonProducer(b))
//     }
// }

// // From rayon::vec
// /// ////////////////////////////////////////////////////////////////////////

// // like std::vec::Drain, without updating a source Vec
// pub(crate) struct SliceDrain<'data, T> {
//     iter: std::slice::IterMut<'data, T>,
// }

// impl<'data, T: 'data> Iterator for SliceDrain<'data, T> {
//     type Item = T;

//     fn next(&mut self) -> Option<T> {
//         // Coerce the pointer early, so we don't keep the
//         // reference that's about to be invalidated.
//         let ptr: *const T = self.iter.next()?;
//         Some(unsafe { std::ptr::read(ptr) })
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         self.iter.size_hint()
//     }

//     fn count(self) -> usize {
//         self.iter.len()
//     }
// }

// impl<'data, T: 'data> DoubleEndedIterator for SliceDrain<'data, T> {
//     fn next_back(&mut self) -> Option<Self::Item> {
//         // Coerce the pointer early, so we don't keep the
//         // reference that's about to be invalidated.
//         let ptr: *const T = self.iter.next_back()?;
//         Some(unsafe { std::ptr::read(ptr) })
//     }
// }

// impl<'data, T: 'data> ExactSizeIterator for SliceDrain<'data, T> {
//     fn len(&self) -> usize {
//         self.iter.len()
//     }
// }

// impl<'data, T: 'data> std::iter::FusedIterator for SliceDrain<'data, T> {}

// impl<'data, T: 'data> Drop for SliceDrain<'data, T> {
//     fn drop(&mut self) {
//         // extract the iterator so we can use `Drop for [T]`
//         let slice_ptr: *mut [T] = std::mem::replace(&mut self.iter, [].iter_mut()).into_slice();
//         unsafe { std::ptr::drop_in_place::<[T]>(slice_ptr) };
//     }
// }
