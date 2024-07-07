use epgi_2d::Color;

pub trait Lerp {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

// impl<T> Lerp for T
// where
//     for<'a> &'a T: Mul<f32, Output = T>,
//     T: Add<Output = T>,
// {
//     fn lerp(&self, other: &Self, t: f32) -> Self {
//         self * (1.0 - t) + other * t
//     }
// }

macro_rules! impl_lerp_for_tuple {
    ($($t: ident),*; $($i:tt),*) => {
        impl<$($t: Lerp),*> Lerp for ($($t),*) {
            fn lerp(&self, other: &Self, t: f32) -> Self {
                ($(self.$i.lerp(&other.$i, t)),*)
            }
        }
    };
}

impl_lerp_for_tuple!(T0, T1; 0, 1);
impl_lerp_for_tuple!(T0, T1, T2; 0, 1, 2);
impl_lerp_for_tuple!(T0, T1, T2, T3; 0, 1, 2, 3);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4; 0, 1, 2, 3, 4);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4, T5; 0, 1, 2, 3, 4, 5);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4, T5, T6; 0, 1, 2, 3, 4, 5, 6);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4, T5, T6, T7; 0, 1, 2, 3, 4, 5, 6, 7);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8; 0, 1, 2, 3, 4, 5, 6, 7, 8);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
impl_lerp_for_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);

macro_rules! impl_lerp_for_int {
    ($t: ty) => {
        impl Lerp for $t {
            fn lerp(&self, other: &Self, t: f32) -> Self {
                self + ((other - self) as f32 * t) as Self
            }
        }
    };
}

impl_lerp_for_int!(i8);
impl_lerp_for_int!(i16);
impl_lerp_for_int!(i32);
impl_lerp_for_int!(i64);
impl_lerp_for_int!(i128);
impl_lerp_for_int!(isize);

macro_rules! impl_lerp_for_uint {
    ($t: ty) => {
        impl Lerp for $t {
            fn lerp(&self, other: &Self, t: f32) -> Self {
                if self <= other {
                    self + ((other - self) as f32 * t) as Self
                } else {
                    self - ((self - other) as f32 * t) as Self
                }
            }
        }
    };
}

impl_lerp_for_uint!(u8);
impl_lerp_for_uint!(u16);
impl_lerp_for_uint!(u32);
impl_lerp_for_uint!(u64);
impl_lerp_for_uint!(u128);
impl_lerp_for_uint!(usize);

macro_rules! impl_lerp_for_float {
    ($t: ty) => {
        impl Lerp for $t {
            fn lerp(&self, other: &Self, t: f32) -> Self {
                self + (other - self) * t as Self
            }
        }
    };
}

impl_lerp_for_float!(f32);
impl_lerp_for_float!(f64);
// impl_lerp_for_big_float!(f128);

impl Lerp for Color {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Color {
            r: (self.r as i16).lerp(&(other.r as i16), t).clamp(0, 255) as u8,
            g: (self.g as i16).lerp(&(other.g as i16), t).clamp(0, 255) as u8,
            b: (self.b as i16).lerp(&(other.b as i16), t).clamp(0, 255) as u8,
            a: (self.a as i16).lerp(&(other.a as i16), t).clamp(0, 255) as u8,
        }
    }
}
