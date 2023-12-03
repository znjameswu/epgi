use std::any::Any;

pub struct HitTestResults {
    candidate: Vec<Box<dyn Fn(&dyn Any) -> bool>>,
}

impl HitTestResults {
    pub fn push(&mut self, candidate: impl Fn(&dyn Any) -> bool + 'static) {
        self.candidate.push(Box::new(candidate))
    }
}
