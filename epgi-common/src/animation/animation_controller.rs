use std::time::{Duration, Instant};

use epgi_core::{
    hooks::{DispatchReducer, Reduce},
    scheduler::JobBuilder,
    tree::BuildContext,
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{lerp, AnimationFrame, Simulation, SimulationState};

pub trait BuildContextUseAnimationControllerExt {
    fn use_animation_controller(
        &mut self,
        init: impl FnOnce() -> AnimationControllerState,
        animation_frame: Option<&AnimationFrame>,
    ) -> (f32, AnimationController);

    fn use_animation_controller_with_simulation(
        &mut self,
        simulation: impl Simulation,
        animation_frame: Option<&AnimationFrame>,
    ) -> (f32, AnimationController);

    fn use_animation_controller_repeating_with(
        &mut self,
        reverse: bool,
        conf: AnimationControllerConf,
        animation_frame: Option<&AnimationFrame>,
    ) -> (f32, AnimationController);
}

impl BuildContextUseAnimationControllerExt for BuildContext<'_> {
    fn use_animation_controller(
        &mut self,
        init: impl FnOnce() -> AnimationControllerState,
        animation_frame: Option<&AnimationFrame>,
    ) -> (f32, AnimationController) {
        use_animation_controller(self, init, animation_frame)
    }

    fn use_animation_controller_with_simulation(
        &mut self,
        simulation: impl Simulation,
        animation_frame: Option<&AnimationFrame>,
    ) -> (f32, AnimationController) {
        use_animation_controller(
            self,
            || AnimationControllerState!(simulation_state = SimulationState::ZERO, simulation),
            animation_frame,
        )
    }

    fn use_animation_controller_repeating_with(
        &mut self,
        reverse: bool,
        conf: AnimationControllerConf,
        animation_frame: Option<&AnimationFrame>,
    ) -> (f32, AnimationController) {
        use_animation_controller(
            self,
            || {
                let now = Instant::now();
                let mut state = AnimationControllerState!(
                    origin_time = now,
                    simulation_state = SimulationState::ZERO,
                );
                state.reduce((now, AnimationControllerAction::Repeat { reverse }, conf));
                state
            },
            animation_frame,
        )
    }
}

pub fn use_animation_controller(
    ctx: &mut BuildContext<'_>,
    init: impl FnOnce() -> AnimationControllerState,
    animation_frame: Option<&AnimationFrame>,
) -> (f32, AnimationController) {
    let (state, dispatch_reducer) = ctx.use_reducer_ref_with(init);
    let controller = AnimationController::new(dispatch_reducer);
    if let Some(animation_frame) = animation_frame {
        if let Some(simulation) = state.simulation.as_ref() {
            let x = simulation.x(animation_frame.time.duration_since(state.origin_time));
            return (x, controller);
        }
    };
    (state.simulation_state.x, controller)
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
            (
                Instant::now(),
                AnimationControllerAction::Repeat { reverse },
                conf,
            ),
            job_builder,
        )
    }
}

#[derive(Clone, Debug, Default, Declarative, TypedBuilder)]
pub struct AnimationControllerConf {
    #[builder(default, setter(into))]
    duration: Option<Duration>,
    #[builder(default, setter(into))]
    reverse_duration: Option<Duration>,
    #[builder(default, setter(into))]
    lower_bound: Option<f32>,
    #[builder(default, setter(into))]
    upper_bound: Option<f32>,
}

#[derive(Clone, Debug, Declarative, TypedBuilder)]
pub struct AnimationControllerState {
    // Animation states
    #[builder(default = std::time::Instant::now())]
    origin_time: Instant,
    simulation_state: SimulationState,
    #[builder(default, setter(transform = |simulation: impl Simulation| Some(Box::new(simulation) as _)))]
    simulation: Option<Box<dyn Simulation>>,

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
pub enum AnimationControllerAction {
    Repeat { reverse: bool },
}

impl Reduce for AnimationControllerState {
    type Action = (Instant, AnimationControllerAction, AnimationControllerConf);

    fn reduce(&mut self, (time, action, conf): Self::Action) {
        if let Some(simulation) = self.simulation.as_ref() {
            // Update the old simulation state for the last time
            // to ensure a smooth transition
            self.simulation_state = simulation.state(time.duration_since(self.origin_time));
        }
        self.duration = conf.duration.or(self.duration);
        self.reverse_duration = conf.reverse_duration.or(self.reverse_duration);
        self.lower_bound = conf.lower_bound.unwrap_or(self.lower_bound);
        self.upper_bound = conf.upper_bound.unwrap_or(self.upper_bound);
        use AnimationControllerAction::*;
        match action {
            Repeat { reverse } => {
                let mut initial_percent = (self.simulation_state.x - self.lower_bound)
                    / (self.upper_bound - self.lower_bound);
                if reverse && self.simulation_state.dx < 0.0 {
                    initial_percent = 2.0 - initial_percent;
                }
                self.simulation = Some(Box::new(RepeatingSimulation {
                    initial_percent,
                    min: self.lower_bound,
                    max: self.upper_bound,
                    reverse,
                    period: self.duration.expect(
                        "Duration of an animation controller needs to be set \
                        before a repeat action can be issued",
                    ),
                }) as _);
            }
        }
    }
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
