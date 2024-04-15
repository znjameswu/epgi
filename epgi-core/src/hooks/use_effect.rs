// use crate::{
//     foundation::{Asc, SyncMutex},
//     tree::{BuildContext, HookState},
// };

use crate::tree::{Effect, Hook};



// impl<'a> BuildContext<'a> {
//     pub fn use_effect<E: FnOnce() -> T + Send + Sync + 'static, T: TearDown>(&mut self, effect: E) {
//         let (hook, index) = self.hooks.use_hook(|| EffectHook {
//             dependencies: (),
//             effect: Asc::new(SyncMutex::new(EffectState {
//                 inner: EffectStateInner::Hold {
//                     fire: move || effect(),
//                 },
//             })),
//         });
//         let effect = hook.effect.clone();
//         self.hooks.effects.push(effect)
//     }

//     pub fn use_effect_with<
//         E: FnOnce(D) -> T + Send + Sync + 'static,
//         T: TearDown,
//         D: PartialEq + Clone + Send + Sync + 'static,
//     >(
//         &mut self,
//         effect: E,
//         dependencies: D,
//     ) {
//         let (hook, index) = self.hooks.use_hook_with(
//             (effect, dependencies),
//             |(effect, dependencies)| {
//                 let dependencies_clone = dependencies.clone();
//                 EffectHook {
//                     dependencies,
//                     effect: Asc::new(SyncMutex::new(EffectState {
//                         inner: EffectStateInner::Hold {
//                             fire: move || effect(dependencies_clone),
//                         },
//                     })),
//                 }
//             },
//             |hook, (effect, dependencies)| {
//                 if hook.dependencies != dependencies {
//                     let dependencies_clone = dependencies.clone();
//                     hook.dependencies = dependencies;
//                     hook.effect = Asc::new(SyncMutex::new(EffectState {
//                         inner: EffectStateInner::Hold {
//                             fire: move || effect(dependencies_clone),
//                         },
//                     }))
//                 }
//             },
//         );
//         let effect = hook.effect.clone();
//         self.hooks.effects.push(effect)
//     }
// }

// #[derive(Clone)]
// pub struct EffectHook<D> {
//     dependencies: D,
//     // Use ref-counting to assure the persistence of fired effects and its tear-downs
//     // Must be private to disallow leaking of ref-counts
//     effect: Asc<SyncMutex<dyn Effect>>,
// }

// impl<D> HookState for EffectHook<D>
// where
//     D: Clone + Send + Sync + 'static,
// {
//     fn clone_box(&self) -> Box<dyn HookState> {
//         Box::new(self.clone())
//     }
// }

// pub trait Effect: Send + Sync + 'static {
//     fn fire(&mut self);
//     fn tear_down(&mut self);
// }

// pub struct EffectState<F, T>
// where
//     F: FnOnce() -> T + Send + Sync + 'static,
//     T: TearDown,
// {
//     inner: EffectStateInner<F, T>,
// }

// enum EffectStateInner<F, T> {
//     Hold { fire: F },
//     Fired { tear_down: T },
//     TearDown,
// }

// impl<F, T> Effect for EffectState<F, T>
// where
//     F: FnOnce() -> T + Send + Sync + 'static,
//     T: TearDown,
// {
//     fn fire(&mut self) {
//         let inner = std::mem::replace(&mut self.inner, EffectStateInner::TearDown);
//         use EffectStateInner::*;
//         match inner {
//             Hold { fire } => {
//                 let tear_down = fire();
//                 self.inner = EffectStateInner::Fired { tear_down };
//             }
//             Fired { .. } => {}
//             TearDown => panic!(),
//         }
//     }

//     fn tear_down(&mut self) {
//         let inner = std::mem::replace(&mut self.inner, EffectStateInner::TearDown);
//         use EffectStateInner::*;
//         match inner {
//             Hold { .. } => {}
//             Fired { tear_down } => {
//                 tear_down.teardown();
//             }
//             TearDown => {}
//         }
//     }
// }

// impl<F, T> Drop for EffectState<F, T>
// where
//     F: FnOnce() -> T + Send + Sync + 'static,
//     T: TearDown,
// {
//     fn drop(&mut self) {
//         self.tear_down()
//     }
// }
