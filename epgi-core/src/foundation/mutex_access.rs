pub(crate) trait MutexAccessor {
    type Mutex: AccessMutex;

    fn need_access(&self, mutex: &Self::Mutex) -> bool;

    fn access(self, inner: &mut <Self::Mutex as AccessMutex>::Inner);
}

pub(crate) trait AccessMutex {
    type Inner;

    fn with_inner(&self, op: impl FnMut(&mut Self::Inner));
}




pub(crate) fn access_node<A: MutexAccessor>(node: A::Mutex) {

}