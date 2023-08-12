/// A sync Mutex without poisoning.
///
/// This abstraction paves the way for an optional parking-lot feature.
#[derive(Debug, Default)]
pub struct SyncMutex<T: ?Sized>(std::sync::Mutex<T>);

impl<T: ?Sized> SyncMutex<T> {
    #[inline]
    pub fn lock(&self) -> std::sync::MutexGuard<'_, T> {
        match self.0.lock() {
            Ok(guard) => guard,
            Err(p_err) => p_err.into_inner(),
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<std::sync::MutexGuard<'_, T>> {
        match self.0.try_lock() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(p_err)) => Some(p_err.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }
}

impl<T> SyncMutex<T> {
    #[inline]
    pub fn new(t: T) -> Self {
        Self(std::sync::Mutex::new(t))
    }
}

/// A sync Rwlock without poisoning.
///
/// This abstraction paves the way for an optional parking-lot feature.
///
/// The std RwLock does not have a correct poisoning behavior whatsoever [https://github.com/rust-lang/rust/issues/89832]
#[derive(Debug, Default)]
pub struct SyncRwLock<T>(std::sync::RwLock<T>);

impl<T> SyncRwLock<T> {
    #[inline]
    pub fn new(t: T) -> Self {
        Self(std::sync::RwLock::new(t))
    }

    #[inline]
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, T> {
        match self.0.read() {
            Ok(guard) => guard,
            Err(p_err) => p_err.into_inner(),
        }
    }

    #[inline]
    pub fn try_read(&self) -> Option<std::sync::RwLockReadGuard<'_, T>> {
        match self.0.try_read() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(p_err)) => Some(p_err.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    #[inline]
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, T> {
        match self.0.write() {
            Ok(guard) => guard,
            Err(p_err) => p_err.into_inner(),
        }
    }

    #[inline]
    pub fn try_write(&self) -> Option<std::sync::RwLockWriteGuard<'_, T>> {
        match self.0.try_write() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(p_err)) => Some(p_err.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }
}
