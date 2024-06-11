use std::time::Duration;

pub trait Simulation: std::fmt::Debug + Send + Sync + 'static {
    fn state(&self, time: Duration) -> SimulationState {
        SimulationState {
            x: self.x(time),
            dx: self.dx(time),
            completed: self.completed(time),
        }
    }

    fn x(&self, time: Duration) -> f32;

    fn dx(&self, time: Duration) -> f32;

    fn completed(&self, time: Duration) -> bool;

    fn clone_box(&self) -> Box<dyn Simulation>;
}

impl Clone for Box<dyn Simulation> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Clone, Debug)]
pub struct SimulationState {
    pub x: f32,
    pub dx: f32,
    pub completed: bool,
}

impl SimulationState {
    pub const ZERO: Self = Self {
        x: 0.0,
        dx: 0.0,
        completed: false,
    };
}
