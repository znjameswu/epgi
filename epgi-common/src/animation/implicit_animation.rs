use std::{
    borrow::Cow,
    fmt::Debug,
    ops::Deref,
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
    /// A state hook providing the state for [ImplicitlyAnimated] to draw.
    ///
    /// To avoid wasting CPU-cycles when the animation has reached their target,
    /// we have to cancel the subscription to the animation frame while static, and add the subscription back while active.
    /// whether to subscribe to the animation frame can only be decided *after* computing the animation progress
    /// (which requires accessing local states).
    /// Therefore, the states has to be stored by a hook in its parent, and the subscription is performed by a child widget.
    /// Hence this hook.
    fn use_implicitly_animated_value<T: Lerp + State + PartialEq>(
        &mut self,
        value: &T,
        duration: Duration,
        curve: Option<&Asc<dyn Tween<Output = f32> + Send + Sync>>,
    ) -> ImplicitlyAnimatedValue<T>;
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
    fn get_consumed_types(&self) -> Cow<[TypeKey]> {
        match &self.value.state {
            ImplicitlyAnimatedValueState::Static { .. } => (&[]).into(),
            ImplicitlyAnimatedValueState::Active { .. } => {
                IMPLICITLY_ANIMATED_BUILDER_CONSUMED_TYPES_ACTIVE
                    .deref()
                    .into()
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
