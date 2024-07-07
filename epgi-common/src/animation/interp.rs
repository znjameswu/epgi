use std::ops::{Add, Mul};

pub trait Lerp {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl<T> Lerp for T
where
    for<'a> &'a T: Mul<f32, Output = T>,
    T: Add<Output = T>,
{
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }
}
