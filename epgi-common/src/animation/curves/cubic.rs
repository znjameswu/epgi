use crate::Tween;

/// The [Cubic] class implements third-order Bézier curves.
///
/// https://api.flutter.dev/flutter/animation/Cubic-class.html
#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Cubic {
    /// The x coordinate of the first control point.
    ///
    /// The line through the point (0, 0) and the first control point is tangent
    /// to the curve at the point (0, 0).
    pub a: f32,
    /// The y coordinate of the first control point.
    ///
    /// The line through the point (0, 0) and the first control point is tangent
    /// to the curve at the point (0, 0).
    pub b: f32,
    /// The x coordinate of the second control point.
    ///
    /// The line through the point (1, 1) and the second control point is tangent
    /// to the curve at the point (1, 1).
    pub c: f32,
    /// The y coordinate of the second control point.
    ///
    /// The line through the point (1, 1) and the second control point is tangent
    /// to the curve at the point (1, 1).
    pub d: f32,
}

const CUBIC_ERROR_BOUND: f32 = 0.001;

impl Tween for Cubic {
    type Output = f32;

    fn interp(&self, t: f32) -> Self::Output {
        fn evaluate_cubic(a: f32, b: f32, m: f32) -> f32 {
            3.0 * a * (1.0 - m) * (1.0 - m) * m + 3.0 * b * (1.0 - m) * m * m + m * m * m
        }
        let mut start = 0.0;
        let mut end = 1.0;
        loop {
            let mid = (start + end) / 2.0;
            let estimate = evaluate_cubic(self.a, self.c, mid);
            if (t - estimate).abs() < CUBIC_ERROR_BOUND {
                return evaluate_cubic(self.b, self.d, mid);
            }
            if estimate < t {
                start = mid;
            } else {
                end = mid;
            }
        }
    }
}

/// A curve that starts quickly and eases into its final position.
///
/// Over the course of the animation, the object spends more time near its
/// final destination. As a result, the user isn’t left waiting for the
/// animation to finish, and the negative effects of motion are minimized.
///
/// https://api.flutter.dev/flutter/animation/Curves/fastOutSlowIn-constant.html
pub const FAST_OUT_SLOW_IN: Cubic = Cubic {
    a: 0.4,
    b: 0.0,
    c: 0.2,
    d: 1.0,
};
