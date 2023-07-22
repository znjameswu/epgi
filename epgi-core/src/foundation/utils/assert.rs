pub trait BoolExpectExt {
    fn assert(self, msg: &str);
}

impl BoolExpectExt for bool {
    fn assert(self, msg: &str) {
        assert!(self, "{}", msg)
    }
}

pub trait AssertExt: Sized {
    #[inline(always)]
    fn assert_with(self, f: impl FnOnce(&Self) -> bool, msg: &str) -> Self {
        assert!(f(&self), "{}", msg);
        self
    }
    #[inline(always)]
    fn assert_eq_with<S: PartialEq + std::fmt::Debug>(
        self,
        f: impl FnOnce(&Self) -> &S,
        value: &S,
        msg: &str,
    ) -> Self {
        assert_eq!(f(&self), value, "{}", msg);
        self
    }
    #[inline(always)]
    fn assert_ne_with<S: PartialEq + std::fmt::Debug>(
        self,
        f: impl FnOnce(&Self) -> &S,
        value: &S,
        msg: &str,
    ) -> Self {
        assert_ne!(f(&self), value, "{}", msg);
        self
    }

    #[inline(always)]
    fn debug_assert_with(self, f: impl FnOnce(&Self) -> bool, msg: &str) -> Self {
        debug_assert!(f(&self), "{}", msg);
        self
    }
    #[inline(always)]
    fn debug_assert_eq_with<S: PartialEq + std::fmt::Debug>(
        self,
        f: impl FnOnce(&Self) -> &S,
        value: &S,
        msg: &str,
    ) -> Self {
        debug_assert_eq!(f(&self), value, "{}", msg);
        self
    }
    #[inline(always)]
    fn debug_assert_ne_with<S: PartialEq + std::fmt::Debug>(
        self,
        f: impl FnOnce(&Self) -> &S,
        value: &S,
        msg: &str,
    ) -> Self {
        debug_assert_ne!(f(&self), value, "{}", msg);
        self
    }
}

pub trait AssertEqExt: PartialEq + std::fmt::Debug + Sized {
    #[inline(always)]
    fn assert_eq(self, value: &Self, msg: &str) -> Self {
        assert_eq!(&self, value, "{}", msg);
        self
    }
    #[inline(always)]
    fn assert_ne(self, value: &Self, msg: &str) -> Self {
        assert_ne!(&self, value, "{}", msg);
        self
    }

    #[inline(always)]
    fn debug_assert_eq(self, value: &Self, msg: &str) -> Self {
        debug_assert_eq!(&self, value, "{}", msg);
        self
    }
    #[inline(always)]
    fn debug_assert_ne(self, value: &Self, msg: &str) -> Self {
        debug_assert_ne!(&self, value, "{}", msg);
        self
    }
}

impl<T> AssertExt for T {}

impl<T> AssertEqExt for T where T: PartialEq + std::fmt::Debug {}
