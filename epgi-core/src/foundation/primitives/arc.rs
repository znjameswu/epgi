/// Atomic Strong-Only Counting
pub type Asc<T> = std::sync::Arc<T>;

pub type Sc<T> = std::rc::Rc<T>;

pub type Arc<T> = std::sync::Arc<T>;

pub type Aweak<T> = std::sync::Weak<T>;
