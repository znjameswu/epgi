use epgi_core::{foundation::Asc, scheduler::JobBuilder};

pub type ArcCallback = Asc<dyn Fn() + Send + Sync>;

pub type ArcJobCallback = Asc<dyn Fn(&mut JobBuilder) + Send + Sync>;

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

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    // // https://godbolt.org/z/sjzzWYWY8
    // a * (1.0 - t) + b * t
    a + (b - a) * t
}
