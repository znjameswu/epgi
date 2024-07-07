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
    nodes::{ComponentElement, ComponentWidget, ConsumerElement, ConsumerWidget},
    read_providers,
    scheduler::get_current_scheduler,
    tree::{ArcChildWidget, BuildContext, ElementBase, Widget},
};
use epgi_macro::Declarative;
use lazy_static::lazy_static;
use typed_builder::TypedBuilder;

use crate::{AnimationFrame, Lerp, Tween};

pub trait BuildContextImplicitAnimationExt {
    fn use_animated_state<T: Tween>(&mut self, init: impl FnOnce() -> T) -> ();
}

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<ImplicitlyAnimatedBuilder<T, F, P>>))]
pub struct ImplicitlyAnimatedBuilder<
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
    P: Protocol,
> {
    value: T,
    duration: Duration,
    #[builder(default, setter(transform = |curve: impl Tween<Output = f32> + Send + Sync + 'static| Some(Asc::new(curve) as _)))]
    curve: Option<Asc<dyn Tween<Output = f32> + Send + Sync>>,
    builder: F,
}

impl<T, F, P> Debug for ImplicitlyAnimatedBuilder<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
    P: Protocol,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImplicitlyAnimatedBuilder")
            .field("value", &self.value)
            .finish()
    }
}

impl<T, F, P> Widget for ImplicitlyAnimatedBuilder<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
    P: Protocol,
{
    type ParentProtocol = P;

    type ChildProtocol = P;

    type Element = ComponentElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl<T, F, P> ComponentWidget<P> for ImplicitlyAnimatedBuilder<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Clone + Send + Sync + 'static,
    P: Protocol,
{
    fn build(&self, ctx: &mut BuildContext<'_>) -> ArcChildWidget<P> {
        let (target, set_target) = ctx.use_state(self.value.clone());
        let (start_time, set_start_time) = ctx.use_state_with(|| Instant::now());
        let (source, set_source) = ctx.use_state(self.value.clone());
        let (current, set_current) = ctx.use_state(self.value.clone());

        let set_source_clone = set_source.clone();
        let current_clone = current.clone();
        ctx.use_effect(
            move |value| {
                let now = Instant::now();
                get_current_scheduler().create_sync_job(|job_builder| {
                    set_target.set(value, job_builder);
                    set_source_clone.set(current_clone, job_builder);
                    set_start_time.set(now, job_builder);
                });
            },
            self.value.clone(),
        );

        ctx.use_effect_2(
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

        let value_state = if current == target {
            ImplicitlyAnimatedBuilderValueState::Static { value: current }
        } else {
            ImplicitlyAnimatedBuilderValueState::Active {
                source,
                target,
                start_time,
                duration: self.duration,
                curve: self.curve.clone(),
            }
        };

        Asc::new(ImplicitlyAnimatedBuilderInner {
            value_state,
            set_current,
            builder: self.builder.clone(),
        })
    }
}

struct ImplicitlyAnimatedBuilderInner<
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
> {
    value_state: ImplicitlyAnimatedBuilderValueState<T>,
    set_current: SetState<T>,
    builder: F,
}

#[derive(Clone)]
enum ImplicitlyAnimatedBuilderValueState<T> {
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

impl<T, F, P> Debug for ImplicitlyAnimatedBuilderInner<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImplicitlyAnimatedBuilderInner").finish()
    }
}

impl<T, F, P> Widget for ImplicitlyAnimatedBuilderInner<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = ConsumerElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

lazy_static! {
    static ref IMPLICITLY_ANIMATED_BUILDER_CONSUMED_TYPES_ACTIVE: [TypeKey; 1] =
        [TypeKey::of::<AnimationFrame>(),];
}

impl<T, F, P> ConsumerWidget<P> for ImplicitlyAnimatedBuilderInner<T, F, P>
where
    T: Lerp + State + PartialEq,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    fn get_consumed_types(&self) -> Cow<[TypeKey]> {
        match &self.value_state {
            ImplicitlyAnimatedBuilderValueState::Static { .. } => (&[]).into(),
            ImplicitlyAnimatedBuilderValueState::Active { .. } => {
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
        let (value, is_active) = match self.value_state.clone() {
            ImplicitlyAnimatedBuilderValueState::Static { value } => {
                debug_assert!(
                    provider_values.is_empty(),
                    "Inactive animated widget should not receive animation frame subscription"
                );
                (value, false)
            }
            ImplicitlyAnimatedBuilderValueState::Active {
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

        let set_current = self.set_current.clone();
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
