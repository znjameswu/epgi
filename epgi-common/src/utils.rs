use epgi_core::foundation::Asc;



pub type ArcCallback = Asc<dyn Fn() + Send + Sync>;

// // No, this will not work. They are not alias but two traits whose trait objects cannot be coerced into each other
// // The real trait alias in Rust nightly circumvented the trait coercion issue
// pub trait Callback: Fn() + Send + Sync {
//     fn call(&self);
// }

// impl<F: ?Sized> Callback for F
// where
//     F: Fn() + Send + Sync,
// {
//     fn call(&self) {
//         (self)()
//     }
// }

// pub trait Callback1<T>: Send + Sync {
//     fn call(&self, t: T);
// }

// impl<F, T> Callback1<T> for F
// where
//     F: Fn(T) + Send + Sync,
// {
//     fn call(&self, t: T) {
//         (self)(t)
//     }
// }

// pub trait Callback1Ref<T>: Send + Sync {
//     fn call(&self, t: &T);
// }

// impl<F, T> Callback1Ref<T> for F
// where
//     F: Fn(&T) + Send + Sync,
// {
//     fn call(&self, t: &T) {
//         (self)(t)
//     }
// }

