use crate::{
    foundation::{AsAny, Error, Inlinable64Vec},
    scheduler::JobId,
};

use super::ElementContextNode;

pub(crate) type HooksWithTearDowns = HooksWith<Option<Box<dyn EffectCleanup>>>;
pub(crate) type HooksWithEffects = HooksWith<Option<Box<dyn Effect>>>;

#[derive(Clone, Default)]
pub struct HooksWith<T> {
    pub array_hooks: Vec<(Box<dyn HookState>, T)>,
}

impl<T> HooksWith<T> {
    pub(crate) fn read<R>(&self, mut init: impl FnMut() -> R) -> HooksWith<R> {
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

impl HooksWithEffects {
    pub(crate) fn fire_effects(self) -> HooksWithTearDowns {
        HooksWith {
            array_hooks: self
                .array_hooks
                .into_iter()
                .map(|(hook_state, effect)| {
                    (
                        hook_state.clone(),
                        effect.and_then(|effect| effect.fire_box()),
                    )
                })
                .collect(),
        }
    }
}

impl HooksWithTearDowns {
    pub(crate) fn merge_with(
        &mut self,
        new_hooks: HooksWithEffects,
        suspended: bool,
        mode: HookContextMode,
    ) {
        let mut self_array = self.array_hooks.iter_mut();
        let mut new_array = new_hooks.array_hooks.into_iter();
        while let Some((hook_state, tear_down)) = self_array.next() {
            let Some((new_hook_state, new_effect)) = new_array.next() else {
                // The new hooks does not cover all existing hooks
                debug_assert!(
                    suspended,
                    "All hooks must be called in a build unless it suspended"
                );
                break;
            };
            *hook_state = new_hook_state;
            if let Some(new_effect) = new_effect {
                tear_down.take().map(|tear_down| tear_down.cleanup());
                *tear_down = new_effect.fire_box()
            }
        }
        while let Some((new_hook_state, new_effect)) = new_array.next() {
            // The new hooks is longer than exisiting hooks
            debug_assert!(matches!(
                mode,
                HookContextMode::Inflate | HookContextMode::PollInflate
            ));
            self.array_hooks
                .push((new_hook_state, new_effect.and_then(Effect::fire_box)));
        }
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
    fn fire_box(self: Box<Self>) -> Option<Box<dyn EffectCleanup>>;
}

/// Helper impl, if you have no effect to return but the signauter requires to specify a return type
impl Effect for () {
    fn fire(self) -> Option<Box<dyn EffectCleanup>> {
        None
    }

    fn fire_box(self: Box<Self>) -> Option<Box<dyn EffectCleanup>> {
        None
    }
}

impl<F> Effect for F
where
    F: FnOnce() -> Option<Box<dyn EffectCleanup>> + Send + Sync + 'static,
{
    fn fire(self) -> Option<Box<dyn EffectCleanup>> {
        (self)()
    }

    fn fire_box(self: Box<Self>) -> Option<Box<dyn EffectCleanup>> {
        self.fire()
    }
}

pub trait EffectCleanup: Send + Sync + 'static {
    fn is_noop(&self) -> bool;
    fn cleanup(self: Box<Self>);
}

impl EffectCleanup for () {
    fn cleanup(self: Box<Self>) {}
    fn is_noop(&self) -> bool {
        true
    }
}

impl<F> EffectCleanup for F
where
    F: FnOnce() + Send + Sync + 'static,
{
    fn cleanup(self: Box<Self>) {
        (self)()
    }
    fn is_noop(&self) -> bool {
        false
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HookContextMode {
    Inflate,
    Rebuild,
    PollInflate,
    // Retry,
}

pub(crate) fn apply_hook_updates<T>(
    element_context: &ElementContextNode,
    job_ids: &Inlinable64Vec<JobId>,
    hooks: &mut HooksWith<T>,
) {
    let mut jobs = {
        element_context
            .mailbox
            .lock()
            .extract_if(|job_id, _| job_ids.contains(job_id))
            .collect::<Vec<_>>()
    };
    jobs.sort_by_key(|(job_id, ..)| *job_id);

    let updates = jobs
        .into_iter()
        .flat_map(|(_, updates)| updates)
        .collect::<Vec<_>>();

    for update in updates {
        (update.op)(
            hooks
                .get_mut(update.hook_index)
                .expect("Update should not contain an invalid index")
                .0
                .as_mut(),
        )
        .ok()
        .expect("We currently do not handle hook failure") //TODO
    }
}
