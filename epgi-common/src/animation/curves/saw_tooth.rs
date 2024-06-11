use crate::Tween;

pub struct SawTooth {
    pub count: u32,
}

impl Tween for SawTooth {
    type Output = f32;

    fn lerp(&self, t: f32) -> Self::Output {
        let t = t * self.count as f32;
        return t % 1.0;
    }
}
