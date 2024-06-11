use std::time::{Duration, Instant};

use epgi_core::{hooks::DispatchReducer, scheduler::JobBuilder, tree::BuildContext};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{lerp, AnimationFrame, Simulation};

pub trait BuildContextUseAnimationControllerExt {
    fn use_animation_controller(
        &mut self,
        init: impl FnOnce() -> AnimationControllerState,
        animation_frame: AnimationFrame,
    ) -> (f32, AnimationController);

    fn use_animation_controller_with_simulation(
        &mut self,
        simulation: impl Simulation,
        animation_frame: AnimationFrame,
    ) -> (f32, AnimationController);
}

impl BuildContextUseAnimationControllerExt for BuildContext<'_> {
    fn use_animation_controller(
        &mut self,
        init: impl FnOnce() -> AnimationControllerState,
        animation_frame: AnimationFrame,
    ) -> (f32, AnimationController) {
        use_animation_controller(self, init, animation_frame)
    }

    fn use_animation_controller_with_simulation(
        &mut self,
        simulation: impl Simulation,
        animation_frame: AnimationFrame,
    ) -> (f32, AnimationController) {
        use_animation_controller(
            self,
            || AnimationControllerState!(simulation),
            animation_frame,
        )
    }
}

pub fn use_animation_controller(
    ctx: &mut BuildContext<'_>,
    init: impl FnOnce() -> AnimationControllerState,
    animation_frame: AnimationFrame,
) -> (f32, AnimationController) {
    let (state, dispatch_reducer) = ctx.use_reducer_ref_with(init);
    let x = state
        .simulation
        .x(animation_frame.time.duration_since(state.origin_time));
    (x, AnimationController::new(dispatch_reducer))
}

pub struct AnimationController {
    dispatch_reducer: DispatchReducer<AnimationControllerState>,
}

impl AnimationController {
    pub fn new(dispatch_reducer: DispatchReducer<AnimationControllerState>) -> Self {
        Self { dispatch_reducer }
    }

    pub fn start_repeat(
        &self,
        reverse: bool,
        duration: Option<Duration>,
        job_builder: &mut JobBuilder,
    ) -> bool {
        self.start_repeat_with(reverse, AnimationControllerConf!(duration), job_builder)
    }
    pub fn start_repeat_with(
        &self,
        reverse: bool,
        conf: AnimationControllerConf,
        job_builder: &mut JobBuilder,
    ) -> bool {
        self.dispatch_reducer.dispatch(
            move |state| {
                let now = Instant::now();
                let time = now.duration_since(state.origin_time);
                let x = state.simulation.x(time);
                let min = conf.lower_bound.unwrap_or(state.lower_bound);
                let max = conf.upper_bound.unwrap_or(state.upper_bound);
                let period = conf.duration.or(state.duration).expect(
                    "Animation controller's duration should be set before starting an animation",
                );
                let mut initial_percent = (x - min) / (max - min);
                if reverse && state.simulation.dx(time) < 0.0 {
                    initial_percent = 2.0 - initial_percent;
                }
                AnimationControllerState!(
                    origin_time = now,
                    simulation = RepeatingSimulation {
                        initial_percent,
                        min,
                        max,
                        reverse,
                        period,
                    },
                    duration = state.duration,
                    reverse_duration = state.reverse_duration,
                    lower_bound = state.lower_bound,
                    upper_bound = state.upper_bound,
                )
            },
            job_builder,
        )
    }
}

#[derive(Clone, Debug, Declarative, TypedBuilder)]
pub struct AnimationControllerConf {
    #[builder(default)]
    duration: Option<Duration>,
    #[builder(default)]
    reverse_duration: Option<Duration>,
    #[builder(default)]
    lower_bound: Option<f32>,
    #[builder(default)]
    upper_bound: Option<f32>,
}

#[derive(Clone, Debug, Declarative, TypedBuilder)]
pub struct AnimationControllerState {
    // Animation states
    #[builder(default = std::time::Instant::now())]
    origin_time: Instant,
    #[builder(setter(transform = |simulation: impl Simulation| Box::new(simulation) as _))]
    simulation: Box<dyn Simulation>,

    // Configuration states (To generate new simulations on demand)
    #[builder(default)]
    duration: Option<Duration>,
    #[builder(default)]
    reverse_duration: Option<Duration>,
    #[builder(default = 0.0)]
    lower_bound: f32,
    #[builder(default = 1.0)]
    upper_bound: f32,
}

#[derive(Clone, Debug)]
struct RepeatingSimulation {
    initial_percent: f32,
    min: f32,
    max: f32,
    reverse: bool,
    period: Duration,
}

impl Simulation for RepeatingSimulation {
    fn x(&self, time: Duration) -> f32 {
        let percent = (time.as_nanos() % self.period.as_nanos()) as f32
            / self.period.as_nanos() as f32
            + self.initial_percent;
        let n = time.as_nanos() / self.period.as_nanos();
        if self.reverse && n % 2 == 1 {
            lerp(self.max, self.min, percent)
        } else {
            lerp(self.max, self.min, percent)
        }
    }

    fn dx(&self, _time: Duration) -> f32 {
        (self.max - self.min) / self.period.as_secs_f32()
    }

    fn completed(&self, _time: Duration) -> bool {
        false
    }

    fn clone_box(&self) -> Box<dyn Simulation> {
        Box::new(self.clone())
    }
}
