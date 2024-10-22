use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use epgi_core::{
    foundation::{
        Arc, Asc, AscProvideExt, InlinableDwsizeVec, Protocol, Provide, SmallVecExt, TypeKey,
    },
    hooks::{SetState, State},
    nodes::{ConsumerElement, ConsumerWidget},
    read_providers,
    scheduler::get_current_scheduler,
    tree::{ArcChildWidget, BuildContext, ElementBase, Widget},
};
use epgi_macro::Declarative;
use lazy_static::lazy_static;
use typed_builder::TypedBuilder;

use crate::{AnimationFrame, Lerp, Tween};

pub trait BuildContextImplicitAnimationExt {
    /// A hook providing the state for [ImplicitlyAnimated] to draw.
    ///
    /// Everytime when the hook is executed with a different value, it will try to smoothly 
    /// continues the current animation to the new value.
    /// 
    /// If you want to trigger the animation using a setter. You will have to manage a state hook
    /// outside to have a setter, and pass the state in as a parameter.
    ///
    /// To avoid wasting CPU-cycles when the animation has reached their target,
    /// we have to cancel the subscription to the animation frame while static, and add the subscription back while active.
    /// whether to subscribe to the animation frame can only be decided *after* computing the animation progress
    /// (which requires accessing local states).
    /// Therefore, the states has to be stored by a hook in its parent, and the subscription is performed by a child widget.
    /// Hence this hook.
    ///
    /// See also:
    /// - [ImplicitlyAnimated]]
    // Design note: we cannot provide an external setter to trigger the implicit animation without 
    // adding another state hook.
    // The reason is that any setter that guarantees animation continuity have to set `target`, 
    // `start_time`, `source` *all at the same time*. Failing to do so will result in glitches.
    // Therefore, we have to either:
    // - Carry the `current` state inside the setter to prepare to set into the `source` state.
    //      Which is totally unacceptable because then we are generating new setters every frame.
    //      Anyone letting their widgets consuming this setter would be stupid or crazy.
    //      No one will be expecting their widgets to be constantly rebuilding due to a setter.
    // - Hiding the `current` state behind a `Arc<Mutex<>>` and carry it in setter.
    //      This could work, but it needs to keep track of a Arc<Mutex<>> and keep it in sync 
    //      using a use_effect. Making it a worse version of the next candidate.
    // - Introduce another state hook as a buffer. The setter only sets that buffer state. And
    //      a use_effect watching that buffer will trigger state updates. We will be better off
    //      by asking users to use a use_state and use_effect if they really want it.
    fn use_implicitly_animated_value<T: Lerp + State + PartialEq>(
        &mut self,
        value: &T,
        duration: Duration,
        curve: Option<&Asc<dyn Tween<Output = f32> + Send + Sync>>,
    ) -> ImplicitlyAnimatedValue<T>;

    /// A hook providing the state for [ImplicitlyAnimated] to draw an entrance animation.
    ///
    /// When the widget is mounted for the first time, it will start from the state given by `init`.
    /// Then it will immediately start transition to the state given by `value.`
    ///
    /// See also:
    /// - [use_implicitly_animated_value](BuildContextImplicitAnimationExt::use_implicitly_animated_value)
    /// - [ImplicitlyAnimated]
    fn use_entrance_animation_value_with<T: Lerp + State + PartialEq>(
        &mut self,
        init: impl FnOnce() -> T,
        value: T,
        duration: Duration,
        curve: Option<&Asc<dyn Tween<Output = f32> + Send + Sync>>,
    ) -> ImplicitlyAnimatedValue<T>;

    /// A hook providing the state for [ImplicitlyAnimated] to draw an entrance animation.
    ///
    /// When the widget is mounted for the first time, it will start from the state given by `init`.
    /// Then it will immediately start transition to the state given by `value.`
    ///
    /// See also:
    /// - [use_implicitly_animated_value](BuildContextImplicitAnimationExt::use_implicitly_animated_value)
    /// - [ImplicitlyAnimated]
    fn use_entrance_animation_value<T: Lerp + State + PartialEq>(
        &mut self,
        init: T,
        value: T,
        duration: Duration,
        curve: Option<&Asc<dyn Tween<Output = f32> + Send + Sync>>,
    ) -> ImplicitlyAnimatedValue<T> {
        self.use_entrance_animation_value_with(move || init, value, duration, curve)
    }
}

impl BuildContextImplicitAnimationExt for BuildContext<'_> {
    fn use_implicitly_animated_value<T: Lerp + State + PartialEq>(
        &mut self,
        value: &T,
        duration: Duration,
        curve: Option<&Asc<dyn Tween<Output = f32> + Send + Sync>>,
    ) -> ImplicitlyAnimatedValue<T> {
        let (target, set_target) = self.use_state_with(|| value.clone());
        let (start_time, set_start_time) = self.use_state_with(|| Instant::now());
        let (source, set_source) = self.use_state_with(|| value.clone());
        let (current, set_current) = self.use_state_with(|| value.clone());

        // First effect, watch the changes on incoming target.
        let set_source_clone = set_source.clone();
        let current_clone = current.clone();
        self.use_effect(
            move |value| {
                let now = Instant::now();
                get_current_scheduler().create_sync_job(|job_builder| {
                    // We use the current value as the new start
                    set_target.set(value, job_builder);
                    set_source_clone.set(current_clone, job_builder);
                    set_start_time.set(now, job_builder);
                });
            },
            value.clone(),
        );

        // Second effect, watch the progress reported by the child. Decide which variant to use.
        self.use_effect_2(
            move |current, target| {
                if current == target {
                    get_current_scheduler().create_sync_job(|job_builder| {
                        set_source.set(current, job_builder);
                    });
                }
            },
            current.clone(),
            target.clone(),
        );

        let state = if current == target {
            ImplicitlyAnimatedValueState::Static { value: current }
        } else {
            ImplicitlyAnimatedValueState::Active {
                source,
                target,
                start_time,
                duration,
                curve: curve.cloned(),
            }
        };
        ImplicitlyAnimatedValue { set_current, state }
    }

    fn use_entrance_animation_value_with<T: Lerp + State + PartialEq>(
        &mut self,
        init: impl FnOnce() -> T,
        value: T,
        duration: Duration,
        curve: Option<&Asc<dyn Tween<Output = f32> + Send + Sync>>,
    ) -> ImplicitlyAnimatedValue<T> {
        let (state, set_state) = self.use_state_with(init);
        let animated_value = self.use_implicitly_animated_value(&state, duration, curve);
        self.use_effect(
            move |_| {
                get_current_scheduler().create_sync_job(|job_builder| {
                    set_state.set(value, job_builder);
                })
            },
            (),
        );
        animated_value
    }
}

impl<T: Lerp + State + PartialEq> ImplicitlyAnimatedValue<T> {
    pub fn build<P: Protocol>(
        self,
        builder: impl Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    ) -> ArcChildWidget<P> {
        ImplicitlyAnimated!(value = self, builder)
    }
}

pub struct ImplicitlyAnimatedValue<T> {
    set_current: SetState<T>,
    state: ImplicitlyAnimatedValueState<T>,
}

#[derive(Clone)]
enum ImplicitlyAnimatedValueState<T> {
    Static {
        value: T,
    },
    Active {
        source: T,
        target: T,
        start_time: Instant,
        duration: Duration,
        curve: Option<Asc<dyn Tween<Output = f32> + Send + Sync>>,
    },
}

/// Draws implicitly animated widget using the value provided by [BuildContext::use_implicitly_animated_value]
///
/// This widget is responsible for optionally subscribe to animation frame.
///
/// To avoid wasting CPU-cycles when the animation has reached their target,
/// we have to cancel the subscription to the animation frame while static, and add the subscription back while active.
/// whether to subscribe to the animation frame can only be decided *after* computing the animation progress
/// (which requires accessing local states).
/// Therefore, the states has to be stored by a hook in its parent, and the subscription is performed by a child widget.
#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<ImplicitlyAnimated<T, F, P>>))]
pub struct ImplicitlyAnimated<
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
> {
    value: ImplicitlyAnimatedValue<T>,
    builder: F,
}

impl<T, F, P> Debug for ImplicitlyAnimated<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImplicitlyAnimatedBuilderInner").finish()
    }
}

impl<T, F, P> Widget for ImplicitlyAnimated<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = ConsumerElement<P>;

    fn into_arc_widget(self: Asc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

lazy_static! {
    static ref IMPLICITLY_ANIMATED_BUILDER_CONSUMED_TYPES_ACTIVE: [TypeKey; 1] =
        [TypeKey::of::<AnimationFrame>(),];
}

impl<T, F, P> ConsumerWidget<P> for ImplicitlyAnimated<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    fn get_consumed_types(&self) -> &[TypeKey] {
        match &self.value.state {
            ImplicitlyAnimatedValueState::Static { .. } => &[],
            ImplicitlyAnimatedValueState::Active { .. } => {
                IMPLICITLY_ANIMATED_BUILDER_CONSUMED_TYPES_ACTIVE.as_ref()
            }
        }
    }

    fn build(
        &self,
        ctx: &mut BuildContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> ArcChildWidget<P> {
        let (value, is_active) = match self.value.state.clone() {
            ImplicitlyAnimatedValueState::Static { value } => {
                debug_assert!(
                    provider_values.is_empty(),
                    "Inactive animated widget should not receive animation frame subscription"
                );
                (value, false)
            }
            ImplicitlyAnimatedValueState::Active {
                source,
                target,
                start_time,
                duration,
                curve,
            } => {
                let frame = read_providers!(provider_values, AnimationFrame);
                let elapsed_duration = frame.time.duration_since(start_time);
                let mut t = ((elapsed_duration.as_nanos() as f64 / duration.as_nanos() as f64)
                    as f32)
                    .clamp(0.0, 1.0);

                if let Some(curve) = curve {
                    t = curve.interp(t);
                }

                let current = source.lerp(&target, t);
                (current, true)
            }
        };

        // Child report their progress to the parent
        let set_current = self.value.set_current.clone();
        ctx.use_effect(
            move |(current, is_active)| {
                if is_active {
                    get_current_scheduler().create_sync_job(|job_builder| {
                        set_current.set(current, job_builder);
                    })
                }
            },
            (value.clone(), is_active),
        );

        (self.builder)(ctx, value)
    }
}

// // The following suffers from name collision from TypedBuilder
// #[derive(Declarative, TypedBuilder)]
// #[builder(build_method(into=Asc<ImplicitlyAnimatedBuilder<T, F, P>>))]
// pub struct ImplicitlyAnimatedBuilder<
//     T: Lerp + State + PartialEq,
//     F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
//     P: Protocol,
// > {
//     value: T,
//     duration: Duration,
//     #[builder(default, setter(transform = |curve: impl Tween<Output = f32> + Send + Sync + 'static| Some(Asc::new(curve) as _)))]
//     curve: Option<Asc<dyn Tween<Output = f32> + Send + Sync>>,
//     builder: F,
// }

// impl<T, F, P> Debug for ImplicitlyAnimatedBuilder<T, F, P>
// where
//     T: Lerp + State + PartialEq,
//     F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
//     P: Protocol,
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("ImplicitlyAnimatedBuilder")
//             .field("value", &self.value)
//             .finish()
//     }
// }

// impl<T, F, P> Widget for ImplicitlyAnimatedBuilder<T, F, P>
// where
//     T: Lerp + State + PartialEq,
//     F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
//     P: Protocol,
// {
//     type ParentProtocol = P;
//     type ChildProtocol = P;
//     type Element = ComponentElement<P>;

//     fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
//         self
//     }
// }

// impl<T, F, P> ComponentWidget<P> for ImplicitlyAnimatedBuilder<T, F, P>
// where
//     T: Lerp + State + PartialEq,
//     F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
//     P: Protocol,
// {
//     fn build(&self, ctx: &mut BuildContext<'_>) -> ArcChildWidget<P> {
//         let value = ctx.use_implicitly_animated_value(&self.value, duration, curve.as_ref());
//         value.build(self.builder.clone())
//     }
// }
