use glam::Vec3;
use instant::Instant;

pub struct Tweeny3D {
    func: fn(f32) -> f32,
    pub start: Vec3,
    pub end: Vec3,
    pub last_pos: Vec3,
    pub start_time: Instant,
    pub duration: f32,
}

impl Tweeny3D {
    pub fn new(func: fn(f32) -> f32, start: Vec3, end: Vec3, duration: f32) -> Self {
        Self {
            func,
            start,
            end,
            last_pos: start,
            start_time: Instant::now(),
            duration,
        }
    }

    pub fn update(&mut self) -> Vec3 {
        let time = self.start_time.elapsed().as_secs_f32();
        let t = (time / self.duration).clamp(0., 1.);
        let s = (self.func)(t);

        let new_pos = self.start.lerp(self.end, s);
        self.last_pos = new_pos;
        new_pos
    }

    pub fn is_finished(&self) -> bool {
        self.start_time.elapsed().as_secs_f32() >= self.duration
    }
}

// https://easings.net/#easeOutExpo
pub fn ease_out_exponential(x: f32) -> f32 {
    if x == 1.0 {
        1.0
    } else {
        1.0 - 2f32.powf(-10. * x)
    }
}

// // https://easings.net/#easeInOutCubic
// pub fn ease_in_out_cubic(x: f32) -> f32 {
//     if x < 0.5 {
//         4.0 * x * x * x
//     } else {
//         1.0 - (-2.0 * x + 2.0).powf(3.0) / 2.0
//     }
// }

// https://easings.net/#easeInOutSine
pub fn ease_in_out_sine(x: f32) -> f32 {
    -((std::f32::consts::PI * x).cos() - 1.0) / 2.0
}
