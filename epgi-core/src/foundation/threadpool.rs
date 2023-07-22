use rayon::prelude::{IntoParallelIterator, ParallelIterator};

pub trait ThreadPoolExt {
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

/// We do not need rayon's FIFO feature, since:
/// 1. We do not preserve a per-thread cache. All contexts are already explicitly passed.
/// 2. We desire to have one thread to process a full subtree. FIFO actually prevent this behavior.
/// 3. Each FIFO scope comes with additional overhead of a vector of N_THREADS FIFO queues for **each** scope.
/// Our trees are deep and would spawn a great deal of scopes.
/// Our trees have small radix numbers and FIFO scope's cost is proportional to N_THREADS only.
impl ThreadPoolExt for rayon::ThreadPool {
    fn par_for_each_vec<T: Send, F: Fn(T) + Send + Sync>(&self, mut vec: Vec<T>, f: F) {
        match vec.len() {
            0 => {}
            1 => f(vec.remove(0)),
            2..=16 => {
                self.scope(|s| {
                    let f_ref = &f;
                    for elem in vec {
                        s.spawn(move |_| f_ref(elem));
                    }
                });
            }
            _ => self.install(|| vec.into_par_iter().for_each(f)),
        };
    }

    // fn par_for_each_slice<T: Sync, F: Fn(&T) + Send + Sync>(&self, iter: &[T], f: F) {
    //     match iter.len() {
    //         0 => {}
    //         1 => f(&iter[0]),
    //         _ => iter.into_par_iter().for_each(f),
    //     };
    // }

    fn par_for_each_arr<T: Send, F: Fn(T) + Send + Sync, const N: usize>(&self, arr: [T; N], f: F) {
        match N {
            0 => {}
            1 => {
                arr.map(f);
            }
            2..=16 => {
                self.scope(|s| {
                    let f_ref = &f;
                    for elem in arr {
                        s.spawn(move |_| f_ref(elem));
                    }
                });
            }
            _ => self.install(|| arr.into_par_iter().for_each(f)),
        };
    }

    fn par_map_collect_vec<T: Send, R: Send, F: Fn(T) -> R + Send + Sync>(
        &self,
        mut iter: Vec<T>,
        f: F,
    ) -> Vec<R> {
        match iter.len() {
            0 => Vec::new(),
            1 => [f(iter.remove(0))].into(),
            len @ 2..=16 => {
                let mut output = std::iter::repeat_with(|| None)
                    .take(len)
                    .collect::<Vec<_>>(); // Brilliant answer from https://www.reddit.com/r/rust/comments/qjh00f/comment/hiqe32i
                self.scope(|s| {
                    let f_ref = &f;
                    for (elem, out) in std::iter::zip(iter, output.iter_mut()) {
                        s.spawn(move |_| *out = Some(f_ref(elem)));
                    }
                });
                return output.into_iter().collect::<Option<Vec<_>>>().unwrap();
            }
            _ => self.install(|| iter.into_par_iter().map(f).collect()),
        }
    }

    // fn par_map_collect_slice<T: Send, R: Send, F: FnMut(T) -> R>(
    //     &self,
    //     iter: &[T],
    //     f: F,
    // ) -> Vec<R> {
    //     todo!()
    // }

    fn par_map_collect_arr<T: Send, R: Send, F: Fn(T) -> R + Send + Sync, const N: usize>(
        &self,
        iter: [T; N],
        f: F,
    ) -> [R; N] {
        match N {
            0 | 1 => iter.map(f),
            2..=16 => {
                let mut output = std::array::from_fn(|_| None);
                self.scope(|s| {
                    let f_ref = &f;
                    for (elem, out) in std::iter::zip(iter, output.iter_mut()) {
                        s.spawn(move |_| *out = Some(f_ref(elem)));
                    }
                });
                output.map(Option::unwrap)
            }
            _ => self.install(|| {
                iter.into_par_iter()
                    .map(f)
                    .collect::<Vec<_>>()
                    .try_into()
                    .ok()
                    .unwrap()
            }),
        }
    }
}
