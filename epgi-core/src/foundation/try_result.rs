pub enum TryResult<T, E1 = (), E2 = ()> {
    Success(T),
    Blocked(E1),
    Yielded(E2),
}
use TryResult::*;

impl<T, E1, E2> TryResult<T, E1, E2> {
    pub fn expect(self, msg: &str) -> T
    where
        E1: std::fmt::Debug,
        E2: std::fmt::Debug,
    {
        match self {
            Success(t) => t,
            Blocked(e) => unwrap_failed(msg, &e),
            Yielded(e) => unwrap_failed(msg, &e),
        }
    }

    pub fn ok(self) -> Option<T> {
        match self {
            Success(t) => Some(t),
            _ => None,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, op: F) -> TryResult<U, E1, E2> {
        match self {
            Success(t) => Success(op(t)),
            Blocked(e) => Blocked(e),
            Yielded(e) => Yielded(e),
        }
    }

    pub fn map_blocked<F, O: FnOnce(E1) -> F>(self, op: O) -> TryResult<T, F, E2> {
        match self {
            Success(t) => Success(t),
            Blocked(e) => Blocked(op(e)),
            Yielded(e) => Yielded(e),
        }
    }

    pub const fn is_err(&self) -> bool {
        !self.is_ok()
    }

    pub const fn is_ok(&self) -> bool {
        matches!(*self, Success(_))
    }
}

// This is a separate function to reduce the code size of the methods
#[cfg(not(feature = "panic_immediate_abort"))]
#[inline(never)]
#[cold]
#[track_caller]
fn unwrap_failed(msg: &str, error: &dyn std::fmt::Debug) -> ! {
    panic!("{msg}: {error:?}")
}

// This is a separate function to avoid constructing a `dyn Debug`
// that gets immediately thrown away, since vtables don't get cleaned up
// by dead code elimination if a trait object is constructed even if it goes
// unused
#[cfg(feature = "panic_immediate_abort")]
#[inline]
#[cold]
#[track_caller]
fn unwrap_failed<T>(_msg: &str, _error: &T) -> ! {
    panic!()
}
