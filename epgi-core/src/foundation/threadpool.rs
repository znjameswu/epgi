use std::mem::MaybeUninit;

use rayon::{
    iter::IndexedParallelIterator,
    prelude::{IntoParallelIterator, ParallelIterator},
};

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

    // fn par_zip_ref_for_each_vec<T1: Sync, T2: Sync, F: Fn(&T1, &T2) + Send + Sync>(
    //     &self,
    //     vec1: &Vec<T1>,
    //     vec2: &Vec<T2>,
    //     f: F,
    // );

    // fn par_zip_ref_map_collect_vec<
    //     T1: Sync,
    //     T2: Sync,
    //     R: Send,
    //     F: Fn(&T1, &T2) -> R + Send + Sync,
    // >(
    //     &self,
    //     vec1: &Vec<T1>,
    //     vec2: &Vec<T2>,
    //     f: F,
    // ) -> Vec<R>;

    /// This peculiar API is a temporary workaround for Stack widget. See https://github.com/rayon-rs/rayon/issues/1179
    fn par_zip3_ref_ref_mut_map_collect_vec<
        T1: Sync,
        T2: Sync,
        T3: Send,
        R: Send,
        F: Fn(&T1, &T2, &mut T3) -> R + Send + Sync,
    >(
        &self,
        vec1: &Vec<T1>,
        vec2: &Vec<T2>,
        vec3: &mut Vec<T3>,
        f: F,
    ) -> Vec<R>;

    /// This peculiar API is a temporary workaround for Stack widget. See https://github.com/rayon-rs/rayon/issues/1179
    fn par_zip3_ref_ref_mut_map_reduce_vec<
        T1: Sync,
        T2: Sync,
        T3: Send,
        R: Send,
        F: Fn(&T1, &T2, &mut T3) -> R + Send + Sync,
        OP: Fn(R, R) -> R + Send + Sync,
        ID: Fn() -> R + Send + Sync,
    >(
        &self,
        vec1: &Vec<T1>,
        vec2: &Vec<T2>,
        vec3: &mut Vec<T3>,
        f: F,
        identity: ID,
        reduce: OP,
    ) -> R;
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
                let mut output = std::iter::repeat_with(|| MaybeUninit::uninit())
                    .take(len)
                    .collect::<Vec<_>>(); // Brilliant answer from https://www.reddit.com/r/rust/comments/qjh00f/comment/hiqe32i
                self.scope(|s| {
                    let f_ref = &f;
                    for (elem, out) in std::iter::zip(iter, output.iter_mut()) {
                        s.spawn(move |_| {
                            out.write(f_ref(elem));
                        });
                    }
                });
                unsafe { std::mem::transmute(output) }
            }
            _ => self.install(|| {
                let mut res = Vec::new();
                iter.into_par_iter().map(f).collect_into_vec(&mut res);
                res
            }),
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
        // const generics ensures redundant branches are shaken out of the compiled product
        match N {
            0 | 1 => iter.map(f),
            2 => {
                let [elem1, elem2] = unsafe { std::mem::transmute_copy(&iter) };
                std::mem::forget(iter);
                let f_ref = &f;
                let res2: [R; 2] = self.join(move || f_ref(elem1), move || f_ref(elem2)).into();
                let res: [R; N] = unsafe { std::mem::transmute_copy(&res2) };
                std::mem::forget(res2);
                res
            }
            3..=16 => {
                let mut output: [MaybeUninit<R>; N] =
                    std::array::from_fn(|_| MaybeUninit::uninit());
                self.scope(|s| {
                    let f_ref = &f;
                    for (elem, out) in std::iter::zip(iter, output.iter_mut()) {
                        s.spawn(move |_| {
                            out.write(f_ref(elem));
                        });
                    }
                });
                unsafe { std::mem::transmute_copy(&output) } // Original output is MaybeUnint, they will be automatically forgotten
            }
            _ => self.install(|| {
                let mut res = Vec::new();
                iter.into_par_iter().map(f).collect_into_vec(&mut res);
                res.try_into().ok().unwrap()
            }),
        }
    }

    // fn par_zip_ref_for_each_vec<T1: Sync, T2: Sync, F: Fn(&T1, &T2) + Send + Sync>(
    //     &self,
    //     vec1: &Vec<T1>,
    //     vec2: &Vec<T2>,
    //     f: F,
    // ) {
    //     let len = std::cmp::min(vec1.len(), vec2.len());
    //     match len {
    //         0 => {}
    //         1 => f(&vec1[0], &vec2[0]),
    //         2..16 => {
    //             self.scope(|s| {
    //                 let f_ref = &f;
    //                 for (elem1, elem2) in std::iter::zip(vec1, vec2) {
    //                     s.spawn(move |_| f_ref(elem1, elem2));
    //                 }
    //             });
    //         }
    //         _ => self.install(|| {
    //             (vec1, vec2)
    //                 .into_par_iter()
    //                 .for_each(|(elem1, elem2)| f(elem1, elem2))
    //         }),
    //     }
    // }

    // fn par_zip_ref_map_collect_vec<
    //     T1: Sync,
    //     T2: Sync,
    //     R: Send,
    //     F: Fn(&T1, &T2) -> R + Send + Sync,
    // >(
    //     &self,
    //     vec1: &Vec<T1>,
    //     vec2: &Vec<T2>,
    //     f: F,
    // ) -> Vec<R> {
    //     let len = std::cmp::min(vec1.len(), vec2.len());
    //     match len {
    //         0 => Vec::new(),
    //         1 => [f(&vec1[0], &vec2[0])].into(),
    //         2..16 => {
    //             let mut output = std::iter::repeat_with(|| None)
    //                 .take(len)
    //                 .collect::<Vec<_>>(); // Brilliant answer from https://www.reddit.com/r/rust/comments/qjh00f/comment/hiqe32i
    //             self.scope(|s| {
    //                 let f_ref = &f;
    //                 for ((elem1, elem2), out) in
    //                     std::iter::zip(std::iter::zip(vec1, vec2), output.iter_mut())
    //                 {
    //                     s.spawn(move |_| *out = Some(f_ref(elem1, elem2)));
    //                 }
    //             });
    //             output.into_iter().map(Option::unwrap).collect()
    //         }
    //         _ => {
    //             let mut res = Vec::new();
    //             self.install(|| {
    //                 (vec1, vec2)
    //                     .into_par_iter()
    //                     .map(|(elem1, elem2)| f(elem1, elem2))
    //                     .collect_into_vec(&mut res)
    //             });
    //             res
    //         }
    //     }
    // }

    fn par_zip3_ref_ref_mut_map_collect_vec<
        T1: Sync,
        T2: Sync,
        T3: Send,
        R: Send,
        F: Fn(&T1, &T2, &mut T3) -> R + Send + Sync,
    >(
        &self,
        vec1: &Vec<T1>,
        vec2: &Vec<T2>,
        vec3: &mut Vec<T3>,
        f: F,
    ) -> Vec<R> {
        let len = std::cmp::min(std::cmp::min(vec1.len(), vec2.len()), vec3.len());
        match len {
            0 => Vec::new(),
            1 => [f(&vec1[0], &vec2[0], &mut vec3[0])].into(),
            2..16 => {
                let mut output = std::iter::repeat_with(|| None)
                    .take(len)
                    .collect::<Vec<_>>(); // Brilliant answer from https://www.reddit.com/r/rust/comments/qjh00f/comment/hiqe32i
                self.scope(|s| {
                    let f_ref = &f;
                    for (((elem1, elem2), elem3), out) in std::iter::zip(
                        std::iter::zip(std::iter::zip(vec1, vec2), vec3),
                        output.iter_mut(),
                    ) {
                        s.spawn(move |_| *out = Some(f_ref(elem1, elem2, elem3)));
                    }
                });
                output.into_iter().map(Option::unwrap).collect()
            }
            _ => {
                let mut res = Vec::new();
                self.install(|| {
                    (vec1, vec2, vec3)
                        .into_par_iter()
                        .map(|(elem1, elem2, elem3)| f(elem1, elem2, elem3))
                        .collect_into_vec(&mut res)
                });
                res
            }
        }
    }

    fn par_zip3_ref_ref_mut_map_reduce_vec<
        T1: Sync,
        T2: Sync,
        T3: Send,
        R: Send,
        F: Fn(&T1, &T2, &mut T3) -> R + Send + Sync,
        OP: Fn(R, R) -> R + Send + Sync,
        ID: Fn() -> R + Send + Sync,
    >(
        &self,
        vec1: &Vec<T1>,
        vec2: &Vec<T2>,
        vec3: &mut Vec<T3>,
        f: F,
        identity: ID,
        reduce: OP,
    ) -> R {
        let len = std::cmp::min(std::cmp::min(vec1.len(), vec2.len()), vec3.len());
        match len {
            0 => identity(),
            1 => f(&vec1[0], &vec2[0], &mut vec3[0]),
            2..16 => {
                let mut output = std::iter::repeat_with(|| None)
                    .take(len)
                    .collect::<Vec<_>>(); // Brilliant answer from https://www.reddit.com/r/rust/comments/qjh00f/comment/hiqe32i
                self.scope(|s| {
                    let f_ref = &f;
                    for (((elem1, elem2), elem3), out) in std::iter::zip(
                        std::iter::zip(std::iter::zip(vec1, vec2), vec3),
                        output.iter_mut(),
                    ) {
                        s.spawn(move |_| *out = Some(f_ref(elem1, elem2, elem3)));
                    }
                });
                output
                    .into_iter()
                    .map(Option::unwrap)
                    .reduce(reduce)
                    .unwrap()
            }
            _ => self.install(|| {
                (vec1, vec2, vec3)
                    .into_par_iter()
                    .map(|(elem1, elem2, elem3)| f(elem1, elem2, elem3))
                    .reduce(identity, reduce)
            }),
        }
    }
}
