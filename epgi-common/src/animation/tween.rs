pub trait Tween {
    type Output;

    fn interp(&self, t: f32) -> Self::Output;

    fn chain_before<T: Tween>(self, other: T) -> ChainedTween<Self, T>
    where
        Self: Sized,
        Self: Tween<Output = f32>,
    {
        ChainedTween { a: self, b: other }
    }

    fn chain_after<T: Tween<Output = f32>>(self, other: T) -> ChainedTween<T, Self>
    where
        Self: Sized,
    {
        ChainedTween { a: other, b: self }
    }
}

pub struct ChainedTween<A, B> {
    a: A,
    b: B,
}

impl<A, B> Tween for ChainedTween<A, B>
where
    A: Tween<Output = f32>,
    B: Tween,
{
    type Output = B::Output;

    fn interp(&self, t: f32) -> Self::Output {
        self.b.interp(self.a.interp(t))
    }
}
