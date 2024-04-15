use crate::{
    foundation::{AsAny, Asc, Error, InlinableDwsizeVec, Key, SyncMutex, VecPushLastExt},
    scheduler::JobId,
};

use super::{AweakElementContextNode, ElementContextNode};

pub(crate) type HooksWithTearDowns = HooksWith<Option<Box<dyn EffectCleanup>>>;
pub(crate) type HooksWithEffects = HooksWith<Option<Box<dyn Effect>>>;

#[derive(Clone, Default)]
pub struct HooksWith<T> {
    pub array_hooks: Vec<(Box<dyn HookState>, T)>,
}

impl<T> HooksWith<T> {
    fn read<R>(&self, mut init: impl FnMut() -> R) -> HooksWith<R> {
        HooksWith {
            array_hooks: self
                .array_hooks
                .iter()
                .map(|(hook_state, _)| (hook_state.clone(), init()))
                .collect(),
        }
    }

    pub(crate) fn get(&self, index: HookIndex) -> Option<&(Box<dyn HookState>, T)> {
        self.array_hooks.get(index.index)
    }

    pub(crate) fn get_mut(&mut self, index: HookIndex) -> Option<&mut (Box<dyn HookState>, T)> {
        self.array_hooks.get_mut(index.index)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HookIndex {
    pub(crate) index: usize,
}

pub trait Hook {
    type HookState: HookState;

    fn create_hook_state(self) -> (Self::HookState, Option<impl Effect>);

    fn update_hook_state(self, state: &mut Self::HookState) -> Option<impl Effect>;
}

pub trait HookState: AsAny + 'static + Send + Sync {
    fn clone_box(&self) -> Box<dyn HookState>;
}

impl Clone for Box<dyn HookState> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub trait Effect: Send + Sync + 'static {
    fn fire(self) -> Option<Box<dyn EffectCleanup>>;
}

impl Effect for () {
    fn fire(self) -> Option<Box<dyn EffectCleanup>> {
        None
    }
}

pub trait EffectCleanup: Send + Sync + 'static {
    fn cleanup(self: Box<Self>);
}

impl EffectCleanup for () {
    fn cleanup(self: Box<Self>) {}
}

impl<F> EffectCleanup for F
where
    F: FnOnce() + Send + Sync + 'static,
{
    fn cleanup(self: Box<Self>) {
        (self)()
    }
}

#[derive(Clone)]
pub struct Update {
    pub(crate) hook_index: HookIndex,
    pub(crate) op: Box<dyn HookCallback>,
}

impl Update {
    pub fn new<T: HookState>(
        hook_index: HookIndex,
        op: impl FnOnce(&mut T) + Clone + Send + Sync + 'static,
    ) -> Self {
        Self {
            hook_index,
            op: Box::new(move |hook: &mut dyn HookState| {
                let Some(hook) = hook.as_any_mut().downcast_mut::<T>() else {
                    return Err(Error::HookError);
                };
                op(hook);
                return Ok(());
            }),
        }
    }
}

pub trait HookCallback:
    FnOnce(&mut dyn HookState) -> Result<(), Error> + 'static + Send + Sync
{
    fn clone_box(&self) -> Box<dyn HookCallback>;
}

impl<T> HookCallback for T
where
    T: FnOnce(&mut dyn HookState) -> Result<(), Error> + Clone + 'static + Send + Sync,
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

// pub(super) enum HookUpdateSink {
//     Buffering(Vec<Update>),
//     Mounted(AweakElementContextNode),
// }

// impl HookUpdateSink {
//     pub(super) fn push(&mut self, job_id: JobId, update: Update) {
//         match self {
//             HookUpdateSink::Buffering(buffer) => {
//                 buffer.push(update);
//             }
//             HookUpdateSink::Mounted(context) => {
//                 if let Some(context) = context.upgrade() {
//                     ElementContextNode::push_update(&context, job_id, update);
//                 }
//             }
//         };
//     }
// }

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WorkMode {
    Inflate,
    Rebuild,
    PollInflate,
    // Retry,
}

pub struct HookContext {
    pub(crate) hooks: HooksWithTearDowns,
    pub(crate) index: usize,
    pub(crate) mode: WorkMode,
}

impl HookContext {
    pub(crate) fn new_rebuild(hooks: HooksWithTearDowns) -> Self {
        todo!()
        // Self {
        //     hooks,
        //     index: 0,
        //     mode: WorkMode::Rebuild,
        //     effects: Default::default(),
        //     // layout_effects: Default::default(),
        // }
    }

    pub(crate) fn new_inflate() -> Self {
        Self {
            hooks: Default::default(),
            index: 0,
            mode: WorkMode::Inflate,
            // layout_effects: Default::default(),
        }
    }

    pub(crate) fn new_poll_inflate(hooks: HooksWithTearDowns) -> Self {
        todo!()
        // Self {
        //     hooks,
        //     index: 0,
        //     mode: WorkMode::PollInflate,
        //     effects: Default::default(),
        //     // layout_effects: Default::default(),
        // }
    }

    pub fn has_finished(&mut self) -> bool {
        self.index == self.hooks.array_hooks.len()
    }

    // pub fn use_hook<T: Hook>(&mut self, hook: T) -> (&mut T::HookState, usize) {
    //     let hooks_len = self.hooks.array_hooks.len();

    //     if self.index < hooks_len {
    //         debug_assert!(matches!(
    //             self.mode,
    //             WorkMode::Rebuild | WorkMode::PollInflate
    //         ));
    //         let (hook_state, effect) = hook.create_hook_state();
    //         self.append_hook(hook_state, effect)
    //     } else if self.index == hooks_len {
    //         debug_assert!(matches!(
    //             self.mode,
    //             WorkMode::Inflate | WorkMode::PollInflate
    //         ));
    //         let (hook_state, effect, index) = self.read_hook();
    //         *effect = hook.update_hook_state(hook_state);
    //         (hook_state, index)
    //     } else {
    //         panic!("Hook reads should not be out of bound")
    //     }
    // }

    // pub(crate) fn use_hook<T: Hook>(&mut self, init: impl FnOnce() -> T) -> (&mut T, usize) {
    //     match self.mode {
    //         WorkMode::Inflate => {
    //             let hooks_len = self.hooks.array_hooks.len();
    //             debug_assert_eq!(
    //                 self.index, hooks_len,
    //                 "Hook index should match with current hook count during inflating"
    //             );
    //             self.append_hook(init)
    //         }
    //         WorkMode::Rebuild => {
    //             let hooks_len = self.hooks.array_hooks.len();
    //             debug_assert!(
    //                 self.index < hooks_len,
    //                 "Hook reads should not be out of bound"
    //             );
    //             self.read_hook()
    //         }
    //         WorkMode::PollInflate => {
    //             let hooks_len = self.hooks.array_hooks.len();
    //             if self.index == hooks_len {
    //                 self.append_hook(init)
    //             } else {
    //                 debug_assert!(
    //                     self.index < hooks_len,
    //                     "Hook reads should not be out of bound"
    //                 );
    //                 self.read_hook()
    //             }
    //         }
    //     }
    // }

    // pub(crate) fn use_hook_with<T: Hook, R>(
    //     &mut self,
    //     resources: R,
    //     init: impl FnOnce(R) -> T,
    //     update: impl FnOnce(&mut T, R),
    // ) -> (&mut T, usize) {
    //     match self.mode {
    //         WorkMode::Inflate => {
    //             let hooks_len = self.hooks.array_hooks.len();
    //             debug_assert_eq!(
    //                 self.index, hooks_len,
    //                 "Hook index should match with current hook count during inflating"
    //             );
    //             self.append_hook(|| init(resources))
    //         }
    //         WorkMode::Rebuild => {
    //             let hooks_len = self.hooks.array_hooks.len();
    //             debug_assert!(
    //                 self.index < hooks_len,
    //                 "Hook reads should not be out of bound"
    //             );
    //             let res = self.read_hook();
    //             update(res.0, resources);
    //             res
    //         }
    //         WorkMode::PollInflate => {
    //             let hooks_len = self.hooks.array_hooks.len();
    //             if self.index == hooks_len {
    //                 self.append_hook(|| init(resources))
    //             } else {
    //                 debug_assert!(
    //                     self.index < hooks_len,
    //                     "Hook reads should not be out of bound"
    //                 );
    //                 let res = self.read_hook();
    //                 update(res.0, resources);
    //                 res
    //             }
    //         }
    //     }
    // }

    // fn append_hook<T: HookState>(
    //     &mut self,
    //     hook: impl HookState,
    //     effect: Option<Box<dyn Effect>>,
    // ) -> (&mut T, usize) {
    //     let hook_ref = self.hooks.array_hooks.push_last((Box::new(hook), effect));
    //     let hook_ref = hook_ref
    //         .as_any_mut()
    //         .downcast_mut::<T>()
    //         .expect("Impossible to fail");
    //     let index = self.index;
    //     self.index += 1;
    //     (hook_ref, index)
    // }

    // fn read_hook<T: HookState>(&mut self) -> (&mut T, &mut Option<Box<dyn Effect>>, usize) {
    //     let (hook_ref, effect) = self
    //         .hooks
    //         .array_hooks
    //         .get_mut(self.index)
    //         .and_then(|(x, effect)| x.as_any_mut().downcast_mut::<T>().map(|x| (x, effect)))
    //         .expect("Hook should be only be read with correct type and position");
    //     let index = self.index;
    //     self.index += 1;
    //     (hook_ref, effect, index)
    // }
}
