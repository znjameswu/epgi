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
