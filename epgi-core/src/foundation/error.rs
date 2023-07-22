use event_listener::EventListener;

use crate::common::SuspendWaker;

pub enum Error {
    Suspended,
    BuildError,
    HookError,
    RawError(ErrorKind),
    Custom(Box<dyn std::error::Error + 'static + Send + Sync>),
}

pub enum ErrorKind {
    Suspended { listener: EventListener },
    BuildError,
    HookError,

    ProviderNotFound,
    ProviderImmutable,
    ProviderTypeMismatch,
}

struct TracedError {
    kind: ErrorKind,
    payload: Option<Box<dyn std::error::Error + 'static + Send + Sync>>,
    previous: Vec<TracedError>,
}

pub struct BuildSuspendedError {
    pub(crate) waker: SuspendWaker,
}
