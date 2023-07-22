/// Idea from https://internals.rust-lang.org/t/idea-return-a-reference-to-the-element-which-were-just-pushed-onto-a-vec/9069/10
pub trait VecPushLastExt<T> {
    fn push_last(&mut self, value: T) -> &mut T;
}

impl<T> VecPushLastExt<T> for Vec<T> {
    fn push_last(&mut self, value: T) -> &mut T {
        self.push(value);
        unsafe { self.last_mut().unwrap_unchecked() }
    }
}
