use crate::foundation::{Protocol, Canvas};



pub trait PaintingContext<C: Canvas> {

}

trait A{}

trait B{
    fn b(&self);
}

impl<T> B for T where T: A{
fn b(&self) {}
}

fn test(a: &impl A) {
    a.b()
}