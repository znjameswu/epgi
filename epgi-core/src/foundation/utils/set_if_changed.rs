pub fn set_if_changed<T: PartialEq>(dst: &mut T, src: T) -> bool {
    if dst != &src {
        *dst = src;
        return true;
    }
    return false;
}

pub fn set_if_changed_ref<T: PartialEq + Clone>(dst: &mut T, src: &T) -> bool {
    if dst != src {
        *dst = src.clone();
        return true;
    }
    return false;
}
