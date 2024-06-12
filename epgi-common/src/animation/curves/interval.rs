use crate::Tween;

/// A curve that is 0.0 until [Self::begin], then curved from
/// 0.0 at [Self::begin] to 1.0 at [Self::end], then remains 1.0 past [Self::end].
///
/// An [Interval] can be used to delay an animation. For example, a six second
/// animation that uses an [Interval] with its [Self::begin] set to 0.5 and its [Self::end]
/// set to 1.0 will essentially become a three-second animation that starts
/// three seconds later.
///
/// https://api.flutter.dev/flutter/animation/Interval-class.html
#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Interval {
    /// The largest value for which this interval is 0.0.
    ///
    /// From t=0.0 to t=[Self::begin], the interval's value is 0.0.
    pub begin: f32,
    /// The smallest value for which this interval is 1.0.
    ///
    /// From t=[Self::end] to t=1.0, the interval's value is 1.0.
    pub end: f32,
}

impl Tween for Interval {
    type Output = f32;

    fn interp(&self, t: f32) -> Self::Output {
        debug_assert!(self.begin >= 0.0);
        debug_assert!(self.begin <= 1.0);
        debug_assert!(self.end >= 0.0);
        debug_assert!(self.end <= 1.0);
        debug_assert!(self.begin < self.end);

        ((t - self.begin) / (self.end - self.begin)).clamp(0.0, 1.0)
    }
}
