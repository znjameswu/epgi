use crate::{
    foundation::{AsAny, Asc, Error, InlinableDwsizeVec, SyncMutex, VecPushLastExt},
    hooks::Effect,
    scheduler::JobId,
};

use super::{AweakElementContextNode, ElementContextNode, Hooks};

pub trait Hook: AsAny + 'static + Send + Sync {
    fn clone_box(&self) -> Box<dyn Hook>;
}

impl Clone for Box<dyn Hook> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// #[derive(Clone)]
// pub struct Update {
//     pub(crate) hook_index: usize,
//     pub(crate) op: Box<dyn HookCallback>,
// }

#[derive(Clone)]
pub struct Update {
    pub(crate) hook_index: usize,
    pub(crate) op: Box<dyn HookCallback>,
}

impl Update {
    pub fn new<T: Hook>(
        hook_index: usize,
        op: impl FnOnce(&mut T) + Clone + Send + Sync + 'static,
    ) -> Self {
        Self {
            hook_index,
            op: Box::new(move |hook: &mut dyn Hook| {
                let Some(hook) = hook.as_any_mut().downcast_mut::<T>() else {
                    return Err(Error::HookError);
                };
                op(hook);
                return Ok(());
            }),
        }
    }
}

pub trait HookCallback: FnOnce(&mut dyn Hook) -> Result<(), Error> + 'static + Send + Sync {
    fn clone_box(&self) -> Box<dyn HookCallback>;
}

impl<T> HookCallback for T
where
    T: FnOnce(&mut dyn Hook) -> Result<(), Error> + Clone + 'static + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn HookCallback> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn HookCallback> {
    fn clone(&self) -> Box<dyn HookCallback> {
        self.clone_box()
    }
}

pub(super) enum HookUpdateSink {
    Buffering(Vec<Update>),
    Mounted(AweakElementContextNode),
}

impl HookUpdateSink {
    pub(super) fn push(&mut self, job_id: JobId, update: Update) {
        match self {
            HookUpdateSink::Buffering(buffer) => {
                buffer.push(update);
            }
            HookUpdateSink::Mounted(context) => {
                if let Some(context) = context.upgrade() {
                    ElementContextNode::push_update(&context, job_id, update);
                }
            }
        };
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WorkMode {
    Inflate,
    Rebuild,
    PollInflate,
    // Retry,
}

pub struct HookContext {
    pub(crate) hooks: Hooks,
    pub(crate) index: usize,
    pub(crate) mode: WorkMode,
    pub(crate) effects: InlinableDwsizeVec<Asc<SyncMutex<dyn Effect>>>,
    // pub(crate) layout_effects: InlinableDwsizeVec<Box<dyn FnOnce() + Send + Sync>>,
}

impl HookContext {
    pub(crate) fn new_rebuild(hooks: Hooks) -> Self {
        Self {
            hooks,
            index: 0,
            mode: WorkMode::Rebuild,
            effects: Default::default(),
            // layout_effects: Default::default(),
        }
    }

    pub(crate) fn new_inflate() -> Self {
        Self {
            hooks: Default::default(),
            index: 0,
            mode: WorkMode::Inflate,
            effects: Default::default(),
            // layout_effects: Default::default(),
        }
    }

    pub(crate) fn new_poll_inflate(hooks: Hooks) -> Self {
        Self {
            hooks,
            index: 0,
            mode: WorkMode::PollInflate,
            effects: Default::default(),
            // layout_effects: Default::default(),
        }
    }

    pub fn has_finished(&mut self) -> bool {
        self.index == self.hooks.array_hooks.len()
    }

    pub(crate) fn use_hook<T: Hook>(&mut self, init: impl FnOnce() -> T) -> (&mut T, usize) {
        match self.mode {
            WorkMode::Inflate => {
                let hooks_len = self.hooks.array_hooks.len();
                debug_assert_eq!(
                    self.index, hooks_len,
                    "Hook index should match with current hook count during inflating"
                );
                self.append_hook(init)
            }
            WorkMode::Rebuild => {
                let hooks_len = self.hooks.array_hooks.len();
                debug_assert!(
                    self.index < hooks_len,
                    "Hook reads should not be out of bound"
                );
                self.read_hook()
            }
            WorkMode::PollInflate => {
                let hooks_len = self.hooks.array_hooks.len();
                if self.index == hooks_len {
                    self.append_hook(init)
                } else {
                    debug_assert!(
                        self.index < hooks_len,
                        "Hook reads should not be out of bound"
                    );
                    self.read_hook()
                }
            }
        }
    }

    pub(crate) fn use_hook_with<T: Hook, R>(
        &mut self,
        resources: R,
        init: impl FnOnce(R) -> T,
        update: impl FnOnce(&mut T, R),
    ) -> (&mut T, usize) {
        match self.mode {
            WorkMode::Inflate => {
                let hooks_len = self.hooks.array_hooks.len();
                debug_assert_eq!(
                    self.index, hooks_len,
                    "Hook index should match with current hook count during inflating"
                );
                self.append_hook(|| init(resources))
            }
            WorkMode::Rebuild => {
                let hooks_len = self.hooks.array_hooks.len();
                debug_assert!(
                    self.index < hooks_len,
                    "Hook reads should not be out of bound"
                );
                let res = self.read_hook();
                update(res.0, resources);
                res
            }
            WorkMode::PollInflate => {
                let hooks_len = self.hooks.array_hooks.len();
                if self.index == hooks_len {
                    self.append_hook(|| init(resources))
                } else {
                    debug_assert!(
                        self.index < hooks_len,
                        "Hook reads should not be out of bound"
                    );
                    let res = self.read_hook();
                    update(res.0, resources);
                    res
                }
            }
        }
    }

    fn append_hook<T: Hook>(&mut self, init: impl FnOnce() -> T) -> (&mut T, usize) {
        let hook_ref = self.hooks.array_hooks.push_last(Box::new(init()));
        let hook_ref = hook_ref
            .as_mut()
            .as_any_mut()
            .downcast_mut::<T>()
            .expect("Impossible to fail");
        let index = self.index;
        self.index += 1;
        (hook_ref, index)
    }

    fn read_hook<T: Hook>(&mut self) -> (&mut T, usize) {
        let hook_ref = self
            .hooks
            .array_hooks
            .get_mut(self.index)
            .and_then(|x| x.as_mut().as_any_mut().downcast_mut::<T>())
            .expect("Hook should be only be read with correct type and position");
        let index = self.index;
        self.index += 1;
        (hook_ref, index)
    }
}
